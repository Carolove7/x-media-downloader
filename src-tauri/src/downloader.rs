use crate::twitter_client::TwitterClient;
use crate::types::{AppConfig, LogPayload, MediaItem, ProgressPayload};
use chrono::Local;
use futures_util::StreamExt;
use reqwest::Client;
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
}

impl DownloadManager {
    pub fn new() -> Self {
        Self {
            cancel_token: CancellationToken::new(),
        }
    }

    pub fn cancel(&self) {
        self.cancel_token.cancel();
    }

    pub async fn run(&self, app: AppHandle, config: AppConfig) -> Result<(), String> {
        let client = TwitterClient::new(&config.auth_token, &config.ct0, &config.user_id)?;
        
        self.log(&app, "info", &format!("正在连接 Twitter 并获取 @{} 元数据...", config.user_id));
        let user_info = client.fetch_user_info().await?;
        
        self.log(&app, "info", &format!("成功识别用户: {} (@{}) | 估算媒体数: {}", user_info.name, user_info.screen_name, user_info.media_count));

        let save_dir = PathBuf::from(&config.save_path).join(&user_info.screen_name);
        fs::create_dir_all(&save_dir).await.map_err(|e| format!("创建保存目录失败: {e}"))?;

        let mut cursor: Option<String> = None;
        let mut total_discovered = 0;
        let downloaded = Arc::new(AtomicUsize::new(0));
        let skipped = Arc::new(AtomicUsize::new(0));
        let processed = Arc::new(AtomicUsize::new(0));

        let concurrency = config.concurrency.unwrap_or(8).clamp(1, 32);
        let semaphore = Arc::new(Semaphore::new(concurrency));
        let http_client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| e.to_string())?;

        while !self.cancel_token.is_cancelled() {
            let (media_items, next_cursor) = client.fetch_media_page(&user_info.rest_id, cursor.as_deref()).await?;
            if media_items.is_empty() {
                break;
            }

            let mut tasks = Vec::new();
            for item in media_items {
                total_discovered += 1;
                let index = total_discovered;
                let sem = semaphore.clone();
                let client = http_client.clone();
                let dir = save_dir.clone();
                let cancel = self.cancel_token.clone();
                let app_handle = app.clone();
                let downloaded_cnt = downloaded.clone();
                let skipped_cnt = skipped.clone();
                let processed_cnt = processed.clone();
                let total_target = user_info.media_count.max(total_discovered as u64) as usize;

                tasks.push(tokio::spawn(async move {
                    if cancel.is_cancelled() {
                        return;
                    }

                    let file_name = format!("{}_{}.{}", item.filename, index, item.ext);
                    let target_path = dir.join(&file_name);

                    if target_path.exists() {
                        if let Ok(meta) = fs::metadata(&target_path).await {
                            if meta.len() > 0 {
                                skipped_cnt.fetch_add(1, Ordering::Relaxed);
                                let p = processed_cnt.fetch_add(1, Ordering::Relaxed) + 1;
                                Self::emit_progress(&app_handle, p, total_target, downloaded_cnt.load(Ordering::Relaxed), skipped_cnt.load(Ordering::Relaxed));
                                return;
                            }
                        }
                    }

                    let _permit = sem.acquire().await.unwrap();
                    if cancel.is_cancelled() {
                        return;
                    }

                    if let Ok(resp) = client.get(&item.url).send().await {
                        if resp.status().is_success() {
                            if let Ok(mut file) = File::create(&target_path).await {
                                let mut stream = resp.bytes_stream();
                                while let Some(chunk) = stream.next().await {
                                    if cancel.is_cancelled() {
                                        let _ = fs::remove_file(&target_path).await;
                                        return;
                                    }
                                    if let Ok(bytes) = chunk {
                                        if file.write_all(&bytes).await.is_err() {
                                            break;
                                        }
                                    }
                                }
                                let _ = file.flush().await;
                                downloaded_cnt.fetch_add(1, Ordering::Relaxed);
                            }
                        }
                    }

                    let p = processed_cnt.fetch_add(1, Ordering::Relaxed) + 1;
                    Self::emit_progress(&app_handle, p, total_target, downloaded_cnt.load(Ordering::Relaxed), skipped_cnt.load(Ordering::Relaxed));
                }));
            }

            for t in tasks {
                let _ = t.await;
            }

            if next_cursor.is_none() || next_cursor == cursor {
                break;
            }
            cursor = next_cursor;
        }

        if self.cancel_token.is_cancelled() {
            self.log(&app, "warn", ">>> 下载任务已被用户手动取消");
        } else {
            self.log(
                &app,
                "success",
                &format!(
                    ">>> 任务执行完毕！新增下载: {} 个，跳过已存在: {} 个。",
                    downloaded.load(Ordering::Relaxed),
                    skipped.load(Ordering::Relaxed)
                ),
            );
        }

        Ok(())
    }

    fn emit_progress(app: &AppHandle, current: usize, total: usize, downloaded: usize, skipped: usize) {
        let percent = if total > 0 { (current as f64 / total as f64) * 100.0 } else { 0.0 };
        let _ = app.emit_all("download-progress", ProgressPayload {
            current,
            total,
            downloaded,
            skipped,
            percent: percent.min(100.0),
        });
    }

    fn log(&self, app: &AppHandle, level: &str, msg: &str) {
        let _ = app.emit_all("download-log", LogPayload {
            level: level.to_string(),
            message: msg.to_string(),
            timestamp: Local::now().format("%H:%M:%S").to_string(),
        });
    }
}
