use crate::twitter_client::TwitterClient;
use crate::types::{AppConfig, LogPayload, MediaItem, ProgressPayload, UserInfo};
use chrono::Local;
use futures_util::StreamExt;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Manager};
use tokio::fs::{self, File};
use tokio::io::AsyncWriteExt;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct DownloadManager {
    cancel_token: CancellationToken,
    transfer_bytes: Arc<AtomicUsize>,
    saved_bytes: Arc<AtomicUsize>,
}

impl DownloadManager {
    pub fn new() -> Self {
        Self {
            cancel_token: CancellationToken::new(),
            transfer_bytes: Arc::new(AtomicUsize::new(0)),
            saved_bytes: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn cancel(&self) {
        self.cancel_token.cancel();
    }

    pub async fn run(&self, app: AppHandle, config: AppConfig) -> Result<(), String> {
        let screen_name = config.user_id.trim_start_matches('@').to_string();
        eprintln!("[DIAG] === DownloadManager::run start for @{} ===", screen_name);

        let client = match TwitterClient::new(
            &config.auth_token,
            &config.ct0,
            &screen_name,
            &config.time_range(),
        ) {
            Ok(c) => { eprintln!("[DIAG] TwitterClient created OK"); c }
            Err(e) => { eprintln!("[DIAG] TwitterClient create FAILED: {}", e); return Err(e); }
        };
        self.log(&app, "info", &format!("正在连接 Twitter 并获取 @{} 元数据...", screen_name));
        let user_info = match client.fetch_user_info().await {
            Ok(info) => {
                eprintln!("[DIAG] fetch_user_info OK: rest_id={}", info.rest_id);
                crate::log_to_file("info", &format!("[DIAG] fetch_user_info OK: rest_id={} media_count={}", info.rest_id, info.media_count));
                info
            }
            Err(e) => {
                eprintln!("[DIAG] fetch_user_info FAILED: {}", e);
                crate::log_to_file("error", &format!("[DIAG] fetch_user_info FAILED: {e}"));
                return Err(e);
            }
        };
        self.log(&app, "info", &format!("成功识别用户: {} (@{}) | 估算媒体数: {}", user_info.name, user_info.screen_name, user_info.media_count));

        let save_dir = PathBuf::from(&config.save_path).join(&user_info.screen_name);
        fs::create_dir_all(&save_dir).await.map_err(|e| format!("创建保存目录失败: {e}"))?;

        // 1. 扫描磁盘已有文件，建立去重集合
        let mut seen_ids = HashSet::new();
        self.scan_existing_files(&save_dir, &mut seen_ids).await;
        if !seen_ids.is_empty() {
            self.log(&app, "info", &format!("[去重] 本地已有 {} 个完整文件", seen_ids.len()));
        }

        // 2. 全局收集下载目标（跨页去重）
        let (targets, skip_count) = match self.collect_targets(&app, &client, &user_info, &mut seen_ids, &config.media_filter).await {
            Ok(result) => {
                eprintln!("[DIAG] collect_targets OK: {} targets, {} skipped", result.0.len(), result.1);
                crate::log_to_file("info", &format!("[DIAG] collect_targets OK: {} targets, {} skipped", result.0.len(), result.1));
                result
            }
            Err(e) => {
                eprintln!("[DIAG] collect_targets FAILED: {}", e);
                crate::log_to_file("error", &format!("[DIAG] collect_targets FAILED: {e}"));
                return Err(e);
            }
        };
        let skip_count = Arc::new(AtomicUsize::new(skip_count));
        let downloaded = Arc::new(AtomicUsize::new(0));
        let failed = Arc::new(AtomicUsize::new(0));
        let processed = Arc::new(AtomicUsize::new(0));

        if targets.is_empty() {
            self.log(&app, "success", ">>> 完成：未发现可下载的新媒体（本地已存在或账号无媒体）");
            self.emit_done(&app, 0, 0, 0);
            return Ok(());
        }

        let total = targets.len();
        self.log(&app, "info", &format!("[信息] 队列就绪：待下载 {} · 跳过 {} · 开始下载", total, skip_count.load(Ordering::Relaxed)));
        self.emit_progress(&app, 0, total, 0, skip_count.load(Ordering::Relaxed), 0, 0.0);

        // 3. 并发下载
        let concurrency = config.concurrency.unwrap_or(8).clamp(1, 32);
        let semaphore = Arc::new(Semaphore::new(concurrency));
        // 与 Python 参考版一致：媒体下载复用同一个带鉴权头的 client
        let http_client = client.media_client().clone();

        // 启动实时速度监控
        let speed_monitor = self.start_speed_monitor(app.clone(), total, downloaded.clone(), skip_count.clone(), failed.clone(), processed.clone());

        let mut tasks = Vec::new();
        for item in targets {
            let sem = semaphore.clone();
            let client = http_client.clone();
            let dir = save_dir.clone();
            let cancel = self.cancel_token.clone();
            let app_handle = app.clone();
            let downloaded_cnt = downloaded.clone();
            let skipped_cnt = skip_count.clone();
            let failed_cnt = failed.clone();
            let processed_cnt = processed.clone();
            let transfer_bytes = self.transfer_bytes.clone();
            let saved_bytes = self.saved_bytes.clone();

            tasks.push(tokio::spawn(async move {
                if cancel.is_cancelled() {
                    return;
                }

                let file_name = format!("{}_{}.{}", item.filename, item.media_id, item.ext);
                let file_path = dir.join(&file_name);
                let partial_path = file_path.with_extension(format!("{}.part", item.ext));

                // 最终兜底：磁盘已有完整文件则跳过
                if let Ok(meta) = fs::metadata(&file_path).await {
                    if meta.len() > 0 {
                        skipped_cnt.fetch_add(1, Ordering::Relaxed);
                        let p = processed_cnt.fetch_add(1, Ordering::Relaxed) + 1;
                        Self::emit_progress_static(&app_handle, p, total, downloaded_cnt.load(Ordering::Relaxed), skipped_cnt.load(Ordering::Relaxed), failed_cnt.load(Ordering::Relaxed), 0.0);
                        return;
                    }
                }

                let _permit = sem.acquire().await.unwrap();
                if cancel.is_cancelled() {
                    return;
                }

                // 与 Python 一致：图片追加 ?name=orig；视频走原始直链
                let is_jpg = item.ext == "jpg";
                let req_url = if is_jpg {
                    format!("{}?name=orig", item.url)
                } else {
                    item.url.clone()
                };
                let mut use_fallback = false;
                let url_fallback = if is_jpg {
                    format!("{}?name=large", item.url)
                } else {
                    item.url.clone()
                };

                let mut last_status = 0u16;
                let mut last_err = String::new();

                // 与 Python 一致：每个文件最多重试 3 次
                for attempt in 0..3 {
                    if cancel.is_cancelled() {
                        let _ = fs::remove_file(&partial_path).await;
                        return;
                    }

                    let effective = if is_jpg && use_fallback { &url_fallback } else { &req_url };
                    let parsed_url = match reqwest::Url::parse(effective) {
                        Ok(u) => u,
                        Err(e) => {
                            failed_cnt.fetch_add(1, Ordering::Relaxed);
                            Self::log_static(&app_handle, "error", &format!("[失败] {} URL 非法: {e}", file_name));
                            let p = processed_cnt.fetch_add(1, Ordering::Relaxed) + 1;
                            Self::emit_progress_static(&app_handle, p, total, downloaded_cnt.load(Ordering::Relaxed), skipped_cnt.load(Ordering::Relaxed), failed_cnt.load(Ordering::Relaxed), 0.0);
                            return;
                        }
                    };

                    // 单次尝试整体限时 600s：client 已无总超时，这里兜底防服务器挂起；
                    // 大视频正常下载远小于该上限，不会被误杀（Python 版为读空闲 20s）。
                    let outcome = tokio::time::timeout(
                        std::time::Duration::from_secs(600),
                        Self::download_once(
                            &client,
                            &parsed_url,
                            &partial_path,
                            &file_path,
                            &file_name,
                            &cancel,
                            &transfer_bytes,
                            &saved_bytes,
                            &downloaded_cnt,
                            &skipped_cnt,
                            &failed_cnt,
                            &processed_cnt,
                            total,
                            &app_handle,
                        ),
                    )
                    .await;

                    match outcome {
                        // 成功：计数、日志、进度均已在 download_once 内完成
                        Ok(Ok(())) => return,
                        Ok(Err((status, msg))) => {
                            last_status = status;
                            last_err = msg;
                            // 原图 404 时回退到 4096x4096 尺寸（对齐参考版 name=orig→4096x4096）
                            if is_jpg && status == 404 {
                                use_fallback = true;
                            }
                        }
                        Err(_) => {
                            last_status = 0;
                            last_err = "单个文件下载超时（超过 600 秒）".to_string();
                        }
                    }
                    let _ = fs::remove_file(&partial_path).await;

                    if attempt < 2 && !cancel.is_cancelled() {
                        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
                    }
                }

                // 3 次尝试均失败
                let _ = fs::remove_file(&partial_path).await;
                failed_cnt.fetch_add(1, Ordering::Relaxed);
                Self::log_static(&app_handle, "error", &format!("[失败] {} HTTP {} {}", file_name, last_status, last_err));
                let p = processed_cnt.fetch_add(1, Ordering::Relaxed) + 1;
                Self::emit_progress_static(&app_handle, p, total, downloaded_cnt.load(Ordering::Relaxed), skipped_cnt.load(Ordering::Relaxed), failed_cnt.load(Ordering::Relaxed), 0.0);
            }));
        }

        for t in tasks {
            let _ = t.await;
        }

        speed_monitor.abort();
        let _ = speed_monitor.await;
        self.update_speed(&app, 0.0, total, downloaded.load(Ordering::Relaxed), skip_count.load(Ordering::Relaxed), failed.load(Ordering::Relaxed), processed.load(Ordering::Relaxed));

        if self.cancel_token.is_cancelled() {
            self.log(&app, "warn", ">>> 下载任务已被用户手动取消");
        } else {
            let down = downloaded.load(Ordering::Relaxed);
            let skip = skip_count.load(Ordering::Relaxed);
            let fail = failed.load(Ordering::Relaxed);
            let mb = self.saved_bytes.load(Ordering::Relaxed) as f64 / (1024.0 * 1024.0);
            self.log(&app, "success", &format!(">>> 完成：新下载 {} · 跳过 {} · 失败 {} · {:.2}MB", down, skip, fail, mb));
        }

        Ok(())
    }

    /// 单个文件的一次下载尝试：发送请求 + 流式写盘 + 原子重命名。
    /// 成功返回 Ok(())（计数/日志/进度已在此内完成）；
    /// 失败返回 Err((HTTP 状态码, 可读原因))，供外层重试。
    async fn download_once(
        client: &reqwest::Client,
        url: &reqwest::Url,
        partial_path: &PathBuf,
        file_path: &PathBuf,
        file_name: &str,
        cancel: &CancellationToken,
        transfer_bytes: &Arc<AtomicUsize>,
        saved_bytes: &Arc<AtomicUsize>,
        downloaded_cnt: &Arc<AtomicUsize>,
        skipped_cnt: &Arc<AtomicUsize>,
        failed_cnt: &Arc<AtomicUsize>,
        processed_cnt: &Arc<AtomicUsize>,
        total: usize,
        app: &AppHandle,
    ) -> Result<(), (u16, String)> {
        eprintln!("[DIAG] download_once starting: {} -> {}", file_name, url);

        let resp = match client.get(url.clone()).send().await {
            Ok(r) => r,
            Err(e) => {
                let err_msg = format!("请求失败: {e}");
                eprintln!("[DIAG] download_once request error: {}", err_msg);
                crate::log_to_file("error", &format!("[DIAG] download_once 请求失败 {} : {}", file_name, err_msg));
                return Err((0, err_msg));
            }
        };
        let status = resp.status().as_u16();
        eprintln!("[DIAG] download_once HTTP {} for {}", status, file_name);
        if !resp.status().is_success() {
            crate::log_to_file("error", &format!("[DIAG] download_once HTTP {status} : {file_name}"));
            return Err((status, format!("HTTP {status}")));
        }

        let mut stream = resp.bytes_stream();
        let mut file = match File::create(partial_path).await {
            Ok(f) => f,
            Err(e) => return Err((status, format!("创建临时文件失败: {e}"))),
        };
        let mut file_bytes: usize = 0;
        loop {
            if cancel.is_cancelled() {
                let _ = fs::remove_file(partial_path).await;
                return Err((status, "用户已取消".into()));
            }
            // 单个 chunk 读取空闲超过 60s 即视为代理中断，失败以便外层重试（而非永久挂起）
            let chunk = match tokio::time::timeout(std::time::Duration::from_secs(60), stream.next()).await {
                Ok(Some(Ok(bytes))) => bytes,
                Ok(Some(Err(e))) => {
                    let _ = fs::remove_file(partial_path).await;
                    return Err((status, format!("读取流中断: {e}")));
                }
                Ok(None) => break, // 流正常结束
                Err(_) => {
                    let _ = fs::remove_file(partial_path).await;
                    return Err((status, "读取超时（60秒无新数据），代理可能中断，将重试".into()));
                }
            };
            if !chunk.is_empty() {
                if let Err(e) = file.write_all(&chunk).await {
                    let _ = fs::remove_file(partial_path).await;
                    return Err((status, format!("写入失败: {e}")));
                }
                file_bytes += chunk.len();
                transfer_bytes.fetch_add(chunk.len(), Ordering::Relaxed);
            }
        }
        let _ = file.flush().await;

        if let Err(e) = fs::rename(partial_path, file_path).await {
            let _ = fs::remove_file(partial_path).await;
            return Err((status, format!("重命名失败: {e}")));
        }

        saved_bytes.fetch_add(file_bytes, Ordering::Relaxed);
        downloaded_cnt.fetch_add(1, Ordering::Relaxed);
        eprintln!("[DIAG] download_once OK: {} ({} bytes)", file_name, file_bytes);
        Self::log_static(app, "success", &format!("[成功] {}", file_name));
        let p = processed_cnt.fetch_add(1, Ordering::Relaxed) + 1;
        Self::emit_progress_static(
            app,
            p,
            total,
            downloaded_cnt.load(Ordering::Relaxed),
            skipped_cnt.load(Ordering::Relaxed),
            failed_cnt.load(Ordering::Relaxed),
            0.0,
        );
        Ok(())
    }

    /// 扫描用户目录下已有文件，从文件名提取 media_id 加入去重集合。
    async fn scan_existing_files(&self, user_folder: &PathBuf, seen: &mut HashSet<String>) {
        let mut entries = match fs::read_dir(user_folder).await {
            Ok(e) => e,
            Err(_) => return,
        };
        while let Ok(Some(entry)) = entries.next_entry().await {
            let name = entry.file_name().to_string_lossy().to_string();
            if let Some(mid) = parse_media_id_from_filename(&name) {
                if let Ok(meta) = entry.metadata().await {
                    if meta.len() > 0 {
                        seen.insert(mid);
                    }
                }
            }
        }
    }

    /// 遍历全部媒体时间线，返回全局去重后的下载队列。
    async fn collect_targets(
        &self,
        app: &AppHandle,
        client: &TwitterClient,
        user_info: &UserInfo,
        seen_ids: &mut HashSet<String>,
        media_filter: &str,
    ) -> Result<(Vec<MediaItem>, usize), String> {
        let mut cursor: Option<String> = None;
        let mut targets = Vec::new();
        let mut skip_count = 0usize;
        let mut seen_this_run = HashSet::new();
        let max_pages = 500;

        let mut page_idx = 0usize;
        for _ in 0..max_pages {
            if self.cancel_token.is_cancelled() {
                break;
            }

            // 对齐参考版：单页请求失败重试 3 次（代理偶发截断大响应体，重试即可恢复）
            let mut page: Option<(Vec<MediaItem>, Option<String>)> = None;
            let mut last_err = String::new();
            for attempt in 0..3 {
                match client.fetch_media_page(&user_info.rest_id, cursor.as_deref()).await {
                    Ok(p) => {
                        page = Some(p);
                        break;
                    }
                    Err(e) => {
                        last_err = e;
                        if attempt < 2 && !self.cancel_token.is_cancelled() {
                            tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
                        }
                    }
                }
            }
            let (media_items, next_cursor) = match page {
                Some(p) => p,
                None => {
                    let msg = format!("获取媒体列表失败（已重试 3 次）: {last_err}");
                    crate::log_to_file("error", &format!("[DIAG] collect_targets FAILED: {msg}"));
                    return Err(msg);
                }
            };
            page_idx += 1;
            crate::log_to_file(
                "info",
                &format!(
                    "[DIAG] 第{page_idx}页 提取 {} 个 (累计队列 {})",
                    media_items.len(),
                    targets.len() + skip_count
                ),
            );
            if media_items.is_empty() {
                break;
            }

            for item in media_items {
                // 类型过滤
                match media_filter {
                    "image" if item.ext != "jpg" => continue,
                    "video" if item.ext != "mp4" => continue,
                    _ => {}
                }

                if seen_ids.contains(&item.media_id) {
                    skip_count += 1;
                } else if seen_this_run.contains(&item.media_id) {
                    skip_count += 1;
                } else {
                    seen_this_run.insert(item.media_id.clone());
                    targets.push(item);
                }
            }

            self.emit_progress(app, targets.len() + skip_count, user_info.media_count.max(targets.len() as u64 + skip_count as u64) as usize, 0, skip_count, 0, 0.0);

            if next_cursor.is_none() || next_cursor == cursor {
                break;
            }
            cursor = next_cursor;
        }

        Ok((targets, skip_count))
    }

    fn start_speed_monitor(
        &self,
        app: AppHandle,
        total: usize,
        downloaded: Arc<AtomicUsize>,
        skipped: Arc<AtomicUsize>,
        failed: Arc<AtomicUsize>,
        processed: Arc<AtomicUsize>,
    ) -> tokio::task::JoinHandle<()> {
        let transfer_bytes = self.transfer_bytes.clone();
        let cancel = self.cancel_token.clone();
        tokio::spawn(async move {
            let mut last_bytes = 0usize;
            let mut last_time = std::time::Instant::now();
            let interval = tokio::time::Duration::from_millis(400);
            loop {
                tokio::select! {
                    _ = tokio::time::sleep(interval) => {}
                    _ = cancel.cancelled() => break,
                }
                let now = std::time::Instant::now();
                let current = transfer_bytes.load(Ordering::Relaxed);
                let elapsed = now.duration_since(last_time).as_secs_f64();
                let speed = if elapsed > 0.0 { (current - last_bytes) as f64 / elapsed } else { 0.0 };
                last_bytes = current;
                last_time = now;
                let p = processed.load(Ordering::Relaxed);
                Self::emit_progress_static(
                    &app,
                    p,
                    total,
                    downloaded.load(Ordering::Relaxed),
                    skipped.load(Ordering::Relaxed),
                    failed.load(Ordering::Relaxed),
                    speed.max(0.0),
                );
            }
        })
    }

    fn emit_progress(&self, app: &AppHandle, current: usize, total: usize, downloaded: usize, skipped: usize, failed: usize, speed: f64) {
        Self::emit_progress_static(app, current, total, downloaded, skipped, failed, speed);
    }

    fn emit_progress_static(app: &AppHandle, current: usize, total: usize, downloaded: usize, skipped: usize, failed: usize, speed: f64) {
        let percent = if total > 0 { (current as f64 / total as f64) * 100.0 } else { 0.0 };
        let _ = app.emit_all("download-progress", ProgressPayload {
            current,
            total,
            downloaded,
            skipped,
            failed,
            percent: percent.min(100.0),
            speed,
        });
    }

    fn update_speed(&self, app: &AppHandle, speed: f64, total: usize, downloaded: usize, skipped: usize, failed: usize, processed: usize) {
        Self::emit_progress_static(app, processed, total, downloaded, skipped, failed, speed);
    }

    fn emit_done(&self, app: &AppHandle, downloaded: usize, skipped: usize, failed: usize) {
        Self::emit_progress_static(app, 0, 0, downloaded, skipped, failed, 0.0);
    }

    fn log(&self, app: &AppHandle, level: &str, msg: &str) {
        Self::log_static(app, level, msg);
    }

    fn log_static(app: &AppHandle, level: &str, msg: &str) {
        crate::log_to_file(level, msg);
        let _ = app.emit_all("download-log", LogPayload {
            level: level.to_string(),
            message: msg.to_string(),
            timestamp: Local::now().format("%H:%M:%S").to_string(),
        });
    }
}

/// 从已保存文件名中还原 media_id。
/// 文件名格式: {timestr}-{img|vid}_{media_id}.{ext}
fn parse_media_id_from_filename(filename: &str) -> Option<String> {
    let base = filename.rsplit_once('.').map(|(b, _)| b).unwrap_or(filename);
    let parts: Vec<&str> = base.splitn(2, '_').collect();
    if parts.len() == 2 && !parts[1].is_empty() {
        Some(parts[1].to_string())
    } else {
        None
    }
}
