// X媒体下载器 - Rust/Tauri 重写版
// 1:1 移植自 twitter_downloader_gui.py (DownloaderWorker)
// 功能保持不变：Cookie 认证、GraphQL 媒体时间线分页、媒体唯一 ID 去重、
// 时间范围过滤、类型过滤、并发下载、断点安全(.part+原子改名)、速度/统计/进度上报。

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::{DateTime, Local, NaiveDate, TimeZone};
use futures_util::StreamExt;
use md5::{Digest, Md5};
use regex::Regex;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::AsyncWriteExt;
use tokio::sync::{Mutex as TokioMutex, Semaphore};

// ----------------- 常量（与 Python 版一致） -----------------

const BEARER_TOKEN: &str = "Bearer AAAAAAAAAAAAAAAAAAAAANRILgAAAAAAnNwIzUejRCOuH5E6I8xnZz4puTs%3D1Zv7ttfk8LF81IUq16cHjhLTvJu4FA33AGWWjCpTnA";
const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/114.0.0.0 Safari/537.36";
const MAX_PAGES: usize = 500;
const API_TIMEOUT: Duration = Duration::from_secs(10);
const CHUNK_TIMEOUT: Duration = Duration::from_secs(20);
const RETRY_LIMIT: usize = 3;
const SELFTEST_BATCH: usize = 8;

const USER_INFO_FEATURES: &str = r#"{"hidden_profile_likes_enabled":false,"hidden_profile_subscriptions_enabled":false,"responsive_web_graphql_exclude_directive_enabled":true,"verified_phone_label_enabled":false,"subscriptions_verification_info_verified_since_enabled":true,"highlights_tweets_tab_ui_enabled":true,"creator_subscriptions_tweet_preview_api_enabled":true,"responsive_web_graphql_timeline_navigation_enabled":true}"#;

const USER_MEDIA_FEATURES: &str = r#"{"responsive_web_graphql_exclude_directive_enabled":true,"verified_phone_label_enabled":false,"creator_subscriptions_tweet_preview_api_enabled":true,"responsive_web_graphql_timeline_navigation_enabled":true,"responsive_web_graphql_skip_user_profile_image_extensions_enabled":false,"tweetypie_unmention_optimization_enabled":true,"responsive_web_edit_tweet_api_enabled":true,"graphql_is_translatable_rweb_tweet_is_translatable_enabled":true,"view_counts_everywhere_api_enabled":true,"longform_notetweets_consumption_enabled":true,"responsive_web_twitter_article_tweet_consumption_enabled":false,"tweet_awards_web_tipping_enabled":false,"freedom_of_speech_not_reach_fetch_enabled":true,"standardized_nudges_misinfo":true,"tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled":true,"longform_notetweets_rich_text_read_enabled":true,"longform_notetweets_inline_media_enabled":true,"responsive_web_media_download_video_enabled":false,"responsive_web_enhance_cards_enabled":false}"#;

// ----------------- 配置持久化（config.json，与 exe 同目录） -----------------

fn default_concurrency() -> u32 {
    8
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub auth_token: String,
    pub ct0: String,
    pub user_id: String,
    pub save_path: String,
    #[serde(default = "default_concurrency")]
    pub concurrency: u32,
    pub media_filter_label: String,
    pub time_range: String,
    pub recent_user_ids: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            auth_token: String::new(),
            ct0: String::new(),
            user_id: String::new(),
            save_path: String::new(),
            concurrency: default_concurrency(),
            media_filter_label: String::new(),
            time_range: String::new(),
            recent_user_ids: Vec::new(),
        }
    }
}

fn config_file() -> PathBuf {
    let dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));
    dir.join("config.json")
}

fn load_config() -> Config {
    std::fs::read_to_string(config_file())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_config_to_disk(cfg: &Config) {
    if let Ok(json) = serde_json::to_string_pretty(cfg) {
        let _ = std::fs::write(config_file(), json);
    }
}

// ----------------- 工具函数（逐条对应 Python 版） -----------------

fn encode_uri_component(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

fn stamp2time(msecs: i64) -> String {
    match Local.timestamp_millis_opt(msecs).single() {
        Some(dt) => dt.format("%Y-%m-%d %H-%M").to_string(),
        None => Local::now().format("%Y-%m-%d %H-%M").to_string(),
    }
}

fn parse_tweet_msecs(created_at: &str) -> Option<i64> {
    // Twitter 标准时间格式: "Wed Jun 10 12:00:00 +0000 2020"（UTC）
    let fmts = [
        "%a %b %d %H:%M:%S %z %Y",
        "%a %b %e %H:%M:%S %z %Y", // 日为空格填充的变体
    ];
    for f in fmts {
        if let Ok(dt) = DateTime::parse_from_str(created_at, f) {
            return Some(dt.timestamp_millis());
        }
    }
    None
}

fn highest_video_quality(variants: &Vec<serde_json::Value>) -> String {
    if variants.is_empty() {
        return String::new();
    }
    if variants.len() == 1 {
        return variants[0].get("url").and_then(|v| v.as_str()).unwrap_or("").to_string();
    }
    let mut max_bitrate: i64 = 0;
    let mut best: Option<String> = None;
    for v in variants {
        if let Some(br) = v.get("bitrate").and_then(|b| b.as_i64()) {
            if br > max_bitrate {
                max_bitrate = br;
                best = v.get("url").and_then(|u| u.as_str()).map(|s| s.to_string());
            }
        }
    }
    best.unwrap_or_else(|| {
        variants[0].get("url").and_then(|u| u.as_str()).unwrap_or("").to_string()
    })
}

fn extract_media_id(url: &str, ext: &str) -> String {
    let base = url.split('?').next().unwrap_or(url);
    static RE_EXT_TW_VIDEO: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    static RE_LONG_DIGITS: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    static RE_MEDIA: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    if ext == "mp4" {
        let re = RE_EXT_TW_VIDEO.get_or_init(|| Regex::new(r"ext_tw_video/(\d+)").unwrap());
        if let Some(m) = re.captures(base) {
            return m[1].to_string();
        }
        let re = RE_LONG_DIGITS.get_or_init(|| Regex::new(r"/(\d{10,})").unwrap());
        if let Some(m) = re.captures(base) {
            return m[1].to_string();
        }
    } else {
        let re = RE_MEDIA.get_or_init(|| Regex::new(r"/media/([A-Za-z0-9_\-]+)").unwrap());
        if let Some(m) = re.captures(base) {
            return m[1].to_string();
        }
    }
    // 兜底：URL 路径(不含查询)的 md5 短哈希，保证同一媒体始终一致
    let mut h = Md5::new();
    h.update(base.as_bytes());
    let digest = format!("{:x}", h.finalize());
    digest[..12].to_string()
}

fn parse_media_id_from_filename(filename: &str) -> Option<String> {
    // 文件名格式: {timestr}-{img|vid}_{media_id}，取第一个下划线之后的全部内容
    let base = match filename.rsplit_once('.') {
        Some((b, _)) => b,
        None => filename,
    };
    match base.split_once('_') {
        Some((_, mid)) if !mid.is_empty() => Some(mid.to_string()),
        _ => None,
    }
}

fn parse_time_range(time_range: &str) -> Option<(NaiveDate, NaiveDate)> {
    // 格式: '1990-01-01:2030-01-01'；无效返回 None（无限制）
    let (s, e) = time_range.trim().split_once(':')?;
    let mut start = NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d").ok()?;
    let mut end = NaiveDate::parse_from_str(e.trim(), "%Y-%m-%d").ok()?;
    if start > end {
        std::mem::swap(&mut start, &mut end);
    }
    Some((start, end))
}

// ----------------- 下载任务参数与状态 -----------------

#[derive(Debug, Clone, Deserialize)]
pub struct DownloadParams {
    pub auth_token: String,
    pub ct0: String,
    pub user_id: String,
    pub save_path: String,
    pub concurrency: u32,
    pub media_filter: String, // all | image | video
    pub time_range: String,
}

struct UserInfo {
    screen_name: String,
    rest_id: String,
    name: String,
    #[allow(dead_code)]
    statuses_count: i64,
    media_count: i64,
    cursor: Option<String>,
}

#[derive(Clone)]
struct Target {
    url: String,
    prefix: String,
    ext: String,
    media_id: String,
}

struct Counters {
    stop: Arc<AtomicBool>,
    transfer_bytes: AtomicU64,
    down: AtomicU64,
    skip: AtomicU64,
    fail: AtomicU64,
    processed: AtomicU64,
    range_in: AtomicU64,
    range_out: AtomicU64,
    bytes_downloaded: AtomicU64,
}

impl Counters {
    fn new() -> Self {
        Self {
            stop: Arc::new(AtomicBool::new(false)),
            transfer_bytes: AtomicU64::new(0),
            down: AtomicU64::new(0),
            skip: AtomicU64::new(0),
            fail: AtomicU64::new(0),
            processed: AtomicU64::new(0),
            range_in: AtomicU64::new(0),
            range_out: AtomicU64::new(0),
            bytes_downloaded: AtomicU64::new(0),
        }
    }
    fn is_stopped(&self) -> bool {
        self.stop.load(Ordering::Relaxed)
    }
}

#[derive(Serialize, Clone)]
struct StatsPayload {
    down: u64,
    skip: u64,
    fail: u64,
}

#[derive(Serialize, Clone)]
struct ProgressPayload {
    current: u64,
    total: u64,
}

#[derive(Serialize, Clone)]
struct DonePayload {
    cancelled: bool,
}

// ----------------- 事件回调（GUI 事件推送 与 CLI 自检 共用核心逻辑） -----------------

#[derive(Clone)]
struct Callbacks {
    log_fn: Arc<dyn Fn(&str) + Send + Sync>,
    stats_fn: Arc<dyn Fn(u64, u64, u64) + Send + Sync>,
    progress_fn: Arc<dyn Fn(u64, u64) + Send + Sync>,
    speed_fn: Arc<dyn Fn(f64) + Send + Sync>,
}

impl Callbacks {
    /// GUI 模式：事件推送到前端
    fn gui(app: AppHandle) -> Self {
        let app1 = app.clone();
        let app2 = app.clone();
        let app3 = app.clone();
        Self {
            log_fn: Arc::new(move |msg| {
                let _ = app1.emit("xdl-log", msg);
            }),
            stats_fn: Arc::new(move |down, skip, fail| {
                let _ = app2.emit("xdl-stats", StatsPayload { down, skip, fail });
            }),
            progress_fn: Arc::new(move |cur, tot| {
                let _ = app3.emit("xdl-progress", ProgressPayload { current: cur, total: tot });
            }),
            speed_fn: Arc::new(move |s| {
                let _ = app.emit("xdl-speed", s);
            }),
        }
    }

    /// CLI 自检模式：日志直接打印到控制台
    fn console() -> Self {
        Self {
            log_fn: Arc::new(|msg| println!("{}", msg)),
            stats_fn: Arc::new(|_, _, _| {}),
            progress_fn: Arc::new(|_, _| {}),
            speed_fn: Arc::new(|_| {}),
        }
    }

    fn log(&self, msg: &str) {
        (self.log_fn)(msg);
    }

    fn speed(&self, s: f64) {
        (self.speed_fn)(s);
    }
}

fn emit_log(sink: &Callbacks, msg: &str) {
    sink.log(msg);
}

fn emit_stats(sink: &Callbacks, c: &Counters) {
    (sink.stats_fn)(
        c.down.load(Ordering::Relaxed),
        c.skip.load(Ordering::Relaxed),
        c.fail.load(Ordering::Relaxed),
    );
}

fn emit_progress(sink: &Callbacks, c: &Counters, total: u64) {
    (sink.progress_fn)(c.processed.load(Ordering::Relaxed), total);
}

// ----------------- 核心下载器（对应 DownloaderWorker） -----------------

struct Downloader {
    sink: Callbacks,
    client: reqwest::Client,
    screen_name: String,
    save_path: String,
    concurrency: usize,
    media_filter: String,
    time_range: String,
    time_bounds: Option<(NaiveDate, NaiveDate)>,
    user_info: UserInfo,
    counters: Arc<Counters>,
}

impl Downloader {
    fn new(sink: Callbacks, params: &DownloadParams, counters: Arc<Counters>) -> Self {
        let screen_name = params.user_id.trim().trim_start_matches('@').to_string();
        let cookie = format!("auth_token={}; ct0={};", params.auth_token.trim(), params.ct0.trim());
        let mut headers = reqwest::header::HeaderMap::new();
        let _ = headers.insert("user-agent", USER_AGENT.parse().unwrap());
        let _ = headers.insert("authorization", BEARER_TOKEN.parse().unwrap());
        let _ = headers.insert("cookie", cookie.parse().unwrap());
        let _ = headers.insert("x-csrf-token", params.ct0.trim().parse().unwrap());
        let _ = headers.insert("referer", format!("https://twitter.com/{}", screen_name).parse().unwrap());

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .expect("构建 HTTP 客户端失败");

        Self {
            sink,
            client,
            screen_name: screen_name.clone(),
            save_path: params.save_path.trim().to_string(),
            concurrency: params.concurrency.clamp(1, 16) as usize,
            media_filter: params.media_filter.clone(),
            time_range: params.time_range.clone(),
            time_bounds: parse_time_range(&params.time_range),
            user_info: UserInfo {
                screen_name,
                rest_id: String::new(),
                name: String::new(),
                statuses_count: 0,
                media_count: 0,
                cursor: None,
            },
            counters,
        }
    }

    fn log(&self, msg: &str) {
        emit_log(&self.sink, msg);
    }

    fn is_stopped(&self) -> bool {
        self.counters.is_stopped()
    }

    // 对应 fetch_user_info()
    async fn fetch_user_info(&mut self) -> bool {
        let variables = format!(
            r#"{{"screen_name":"{}","withSafetyModeUserFields":false}}"#,
            self.user_info.screen_name
        );
        let url = format!(
            "https://twitter.com/i/api/graphql/xc8f1g7BYqr6VTzTbvNlGw/UserByScreenName?variables={}&features={}&fieldToggles=%7B%22withAuxiliaryUserLabels%22%3Afalse%7D",
            encode_uri_component(&variables),
            encode_uri_component(USER_INFO_FEATURES),
        );

        for attempt in 1..=RETRY_LIMIT {
            if self.is_stopped() {
                return false;
            }
            match self.client.get(&url).timeout(API_TIMEOUT).send().await {
                Ok(resp) => {
                    let status = resp.status().as_u16();
                    if status == 401 {
                        self.log("[错误] Cookie 无效或已过期，请检查 auth_token 及 ct0。");
                        return false;
                    }
                    if status == 429 {
                        self.log("[错误] 请求过于频繁 (429 频率限制)，请稍后重试。");
                        return false;
                    }
                    let text = resp.text().await.unwrap_or_default();
                    match serde_json::from_str::<serde_json::Value>(&text) {
                        Ok(raw) => {
                            if let Some(errors) = raw.get("errors").and_then(|e| e.as_array()) {
                                if let Some(first) = errors.first() {
                                    let msg = first
                                        .get("message")
                                        .and_then(|m| m.as_str())
                                        .unwrap_or("未知错误");
                                    self.log(&format!("[错误] 接口返回错误: {}", msg));
                                    return false;
                                }
                            }
                            let user = raw
                                .pointer("/data/user/result")
                                .and_then(|v| v.as_object().cloned());
                            match user {
                                None => {
                                    self.log(&format!(
                                        "[错误] 未找到账户 @{}（可能不存在、被封禁或无权限访问）。",
                                        self.screen_name
                                    ));
                                    return false;
                                }
                                Some(user) => {
                                    self.user_info.rest_id = user
                                        .get("rest_id")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string();
                                    let legacy = user.get("legacy").cloned().unwrap_or_default();
                                    self.user_info.name = legacy
                                        .get("name")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string();
                                    self.user_info.statuses_count = legacy
                                        .get("statuses_count")
                                        .and_then(|v| v.as_i64())
                                        .unwrap_or(0);
                                    self.user_info.media_count = legacy
                                        .get("media_count")
                                        .and_then(|v| v.as_i64())
                                        .unwrap_or(0);
                                    return true;
                                }
                            }
                        }
                        Err(e) => {
                            if attempt < RETRY_LIMIT && !self.is_stopped() {
                                self.log(&format!(
                                    "[重试] 获取用户信息中，正在进行第 {} 次重试... ({})",
                                    attempt, e
                                ));
                                tokio::time::sleep(Duration::from_millis(1500)).await;
                            } else {
                                self.log(&format!("[错误] 获取用户信息失败 (超时10秒/重试用尽): {}", e));
                                return false;
                            }
                        }
                    }
                }
                Err(e) => {
                    if attempt < RETRY_LIMIT && !self.is_stopped() {
                        self.log(&format!(
                            "[重试] 获取用户信息中，正在进行第 {} 次重试... ({})",
                            attempt, e
                        ));
                        tokio::time::sleep(Duration::from_millis(1500)).await;
                    } else {
                        self.log(&format!("[错误] 获取用户信息失败 (超时10秒/重试用尽): {}", e));
                        return false;
                    }
                }
            }
        }
        false
    }

    // 对应 get_download_urls()
    async fn get_download_urls(&mut self) -> Option<Vec<Target>> {
        if self.is_stopped() {
            return None;
        }
        let variables = match &self.user_info.cursor {
            Some(c) => format!(
                r#"{{"userId":"{}","count":500,"cursor":"{}","includePromotedContent":false,"withClientEventToken":false,"withBirdwatchNotes":false,"withVoice":true,"withV2Timeline":true}}"#,
                self.user_info.rest_id, c
            ),
            None => format!(
                r#"{{"userId":"{}","count":500,"includePromotedContent":false,"withClientEventToken":false,"withBirdwatchNotes":false,"withVoice":true,"withV2Timeline":true}}"#,
                self.user_info.rest_id
            ),
        };
        let url = format!(
            "https://twitter.com/i/api/graphql/Le6KlbilFmSu-5VltFND-Q/UserMedia?variables={}&features={}",
            encode_uri_component(&variables),
            encode_uri_component(USER_MEDIA_FEATURES),
        );

        let mut resp = None;
        for attempt in 1..=RETRY_LIMIT {
            if self.is_stopped() {
                return None;
            }
            match self.client.get(&url).timeout(API_TIMEOUT).send().await {
                Ok(r) => {
                    resp = Some(r);
                    break;
                }
                Err(e) => {
                    if attempt < RETRY_LIMIT && !self.is_stopped() {
                        self.log(&format!("[重试] 获取推文列表中，正在第 {} 次重试...", attempt));
                        tokio::time::sleep(Duration::from_millis(1500)).await;
                    } else {
                        self.log(&format!("[错误] 请求推文列表失败 (超时10秒/重试用尽): {}", e));
                        return None;
                    }
                }
            }
        }
        let resp = resp?;
        let status = resp.status().as_u16();
        let text = resp.text().await.unwrap_or_default();

        if status == 429 || text.contains("Rate limit exceeded") {
            self.log("[提示] 触发 API 频率限制 (429 Rate limit)，已终止本次任务以防账号封禁。");
            return None;
        }

        let raw: serde_json::Value = match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(e) => {
                self.log(&format!("[错误] 解析推文列表出错: {}", e));
                return None;
            }
        };

        let timeline = raw
            .pointer("/data/user/result/timeline_v2/timeline")
            .or_else(|| raw.pointer("/data/user/result/timeline/timeline"))
            .cloned()
            .unwrap_or_default();
        let instructions = timeline
            .get("instructions")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        if instructions.is_empty() {
            return None;
        }

        let mut items: Vec<serde_json::Value> = Vec::new();
        let mut next_cursor: Option<String> = None;

        for instr in &instructions {
            let mut entries: Vec<serde_json::Value> = instr
                .get("entries")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            if entries.is_empty() {
                if let Some(entry) = instr.get("entry") {
                    entries.push(entry.clone());
                }
            }
            for entry in &entries {
                let entry_id = entry.get("entryId").and_then(|v| v.as_str()).unwrap_or("");
                if entry_id.contains("bottom") || entry_id.contains("cursor-bottom") {
                    let content = entry.get("content").cloned().unwrap_or_default();
                    let v = content
                        .get("value")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                        .or_else(|| {
                            content
                                .pointer("/cursor/value")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string())
                        });
                    if v.is_some() {
                        next_cursor = v;
                    }
                }
                let content = entry.get("content").cloned().unwrap_or_default();
                if let Some(list) = content.get("items").and_then(|v| v.as_array()) {
                    items.extend(list.iter().cloned());
                } else if let Some(list) = content.get("moduleItems").and_then(|v| v.as_array()) {
                    items.extend(list.iter().cloned());
                } else if content.get("itemContent").is_some() {
                    items.push(entry.clone());
                }
            }
            if let Some(list) = instr.get("moduleItems").and_then(|v| v.as_array()) {
                items.extend(list.iter().cloned());
            }
        }

        if next_cursor.as_deref() != self.user_info.cursor.as_deref() {
            self.user_info.cursor = next_cursor;
        } else {
            self.user_info.cursor = None;
        }

        let mut targets: Vec<Target> = Vec::new();
        for item in &items {
            if self.is_stopped() {
                break;
            }
            // 每条目独立容错：解析失败仅跳过该条
            if let Some(t) = self.parse_item(item) {
                targets.extend(t);
            }
        }

        // 按下载类型筛选
        if self.media_filter == "image" {
            targets.retain(|t| t.ext == "jpg");
        } else if self.media_filter == "video" {
            targets.retain(|t| t.ext == "mp4");
        }

        Some(targets)
    }

    // 对应 get_download_urls() 内的单条目解析（try/except 包裹的部分）
    fn parse_item(&self, item: &serde_json::Value) -> Option<Vec<Target>> {
        let entry_obj = item.get("item").unwrap_or(item);
        let item_content = entry_obj.get("itemContent")?;
        if item_content.get("tweet_results").is_none() {
            return None;
        }
        let tweet_result = item_content.pointer("/tweet_results/result")?;
        let tweet_obj = tweet_result.get("tweet").unwrap_or(tweet_result);
        let legacy = tweet_obj.get("legacy")?;

        // 推文发布时间（UTC）
        let created_at = legacy.get("created_at").and_then(|v| v.as_str());
        let tweet_msecs = created_at.and_then(parse_tweet_msecs);
        let timestr = match tweet_msecs {
            Some(ms) => stamp2time(ms),
            None => Local::now().format("%Y-%m-%d %H-%M").to_string(),
        };

        // 媒体列表（图片/视频）
        let media_list = legacy
            .pointer("/extended_entities/media")
            .or_else(|| legacy.pointer("/entities/media"))
            .and_then(|v| v.as_array())?
            .clone();

        // 时间范围筛选：按 UTC 日期比较
        if let Some((start, end)) = self.time_bounds {
            let mut in_range = false;
            if let Some(ms) = tweet_msecs {
                if let Some(dt) = DateTime::from_timestamp_millis(ms) {
                    let utc_date = dt.naive_utc().date();
                    in_range = start <= utc_date && utc_date <= end;
                }
            }
            if !in_range {
                self.counters.range_out.fetch_add(media_list.len() as u64, Ordering::Relaxed);
                return None;
            }
        }

        let mut out = Vec::new();
        for media in &media_list {
            if let Some(video_info) = media.get("video_info") {
                let variants = video_info
                    .get("variants")
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default();
                let vid_url = highest_video_quality(&variants);
                if !vid_url.is_empty() {
                    let media_id = extract_media_id(&vid_url, "mp4");
                    out.push(Target {
                        url: vid_url,
                        prefix: format!("{}-vid", timestr),
                        ext: "mp4".into(),
                        media_id,
                    });
                    self.counters.range_in.fetch_add(1, Ordering::Relaxed);
                }
            } else if let Some(img_url) = media.get("media_url_https").and_then(|v| v.as_str()) {
                if !img_url.is_empty() {
                    let media_id = extract_media_id(img_url, "jpg");
                    out.push(Target {
                        url: img_url.to_string(),
                        prefix: format!("{}-img", timestr),
                        ext: "jpg".into(),
                        media_id,
                    });
                    self.counters.range_in.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
        Some(out)
    }

    // 对应 _scan_existing_files()
    fn scan_existing_files(&self, user_folder: &Path) -> std::collections::HashSet<String> {
        let mut seen = std::collections::HashSet::new();
        let dir = match std::fs::read_dir(user_folder) {
            Ok(d) => d,
            Err(_) => return seen,
        };
        for entry in dir.flatten() {
            let filename = entry.file_name().to_string_lossy().into_owned();
            if let Some(mid) = parse_media_id_from_filename(&filename) {
                if let Ok(md) = entry.metadata() {
                    if md.len() > 0 {
                        seen.insert(mid);
                    }
                }
            }
        }
        seen
    }

    // 对应 _collect_targets()
    async fn collect_targets(&mut self, seen_ids: &std::collections::HashSet<String>) -> Vec<Target> {
        let mut targets: Vec<Target> = Vec::new();
        let mut seen_this_run: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut pages = 0;
        while !self.is_stopped() && pages < MAX_PAGES {
            let page = match self.get_download_urls().await {
                Some(p) => p,
                None => break,
            };
            if page.is_empty() {
                break;
            }
            pages += 1;
            for t in page {
                if seen_ids.contains(&t.media_id) {
                    self.counters.skip.fetch_add(1, Ordering::Relaxed);
                } else if seen_this_run.contains(&t.media_id) {
                    self.counters.skip.fetch_add(1, Ordering::Relaxed);
                } else {
                    seen_this_run.insert(t.media_id.clone());
                    targets.push(t);
                }
            }
            if self.user_info.cursor.is_none() {
                break;
            }
        }
        targets
    }

    // 对应 run() 主流程
    async fn run(&mut self) -> bool {
        let start_time = Instant::now();
        let sep = "=".repeat(40);
        self.log(&sep);
        self.log(&format!(
            "开始 | @{} | 类型 {} | 并发 {}",
            self.screen_name, self.media_filter, self.concurrency
        ));
        let user_folder = Path::new(&self.save_path).join(&self.screen_name);
        self.log(&format!("保存至: {}", user_folder.display()));
        if self.time_bounds.is_some() {
            self.log(&format!("时间范围: {}", self.time_range));
        }
        self.log(&sep);
        self.log(&format!("获取用户 @{} 信息…", self.screen_name));

        if !self.fetch_user_info().await {
            emit_stats(&self.sink, &self.counters);
            return false;
        }

        self.log(&format!("用户: {} (ID: {})", self.user_info.name, self.user_info.rest_id));
        self.log(&format!("媒体总数(估算): {}", self.user_info.media_count));

        if let Err(e) = std::fs::create_dir_all(&user_folder) {
            self.log(&format!("[错误] 无法创建保存目录: {}", e));
            emit_stats(&self.sink, &self.counters);
            return false;
        }

        // 扫描磁盘已有文件，构建去重集合（仅完整文件计入）
        let seen_ids = self.scan_existing_files(&user_folder);
        if !seen_ids.is_empty() {
            self.log(&format!("[去重] 本地已有 {} 个完整文件", seen_ids.len()));
        }

        // 阶段一：一次性遍历全部媒体时间线，构建全局去重后的下载队列
        let targets = self.collect_targets(&seen_ids).await;

        if self.time_bounds.is_some() {
            self.log(&format!(
                "时间范围 [{}] 内 {} 个，已排除 {} 个",
                self.time_range,
                self.counters.range_in.load(Ordering::Relaxed),
                self.counters.range_out.load(Ordering::Relaxed)
            ));
        }

        if targets.is_empty() {
            if !seen_ids.is_empty() {
                self.log(&format!("[信息] 共 {} 个本地媒体均已存在，无需重复下载。", seen_ids.len()));
            } else {
                self.log("[信息] 未发现任何可下载的媒体。");
            }
            emit_stats(&self.sink, &self.counters);
            self.log(&sep);
            return false;
        }

        let discovered_total = targets.len() as u64;
        self.log(&format!(
            "[信息] 队列就绪：待下载 {} · 跳过 {} · 开始下载",
            discovered_total,
            self.counters.skip.load(Ordering::Relaxed)
        ));
        emit_progress(&self.sink, &self.counters, discovered_total);
        emit_stats(&self.sink, &self.counters);
        self.sink.speed(0.0);

        // 速度监控：0.4s 汇总一次全部下载线程吞吐
        let monitor_sink = self.sink.clone();
        let monitor_counters = self.counters.clone();
        let speed_monitor = tokio::spawn(async move {
            let mut last_bytes = 0u64;
            let mut last_time = Instant::now();
            loop {
                tokio::time::sleep(Duration::from_millis(400)).await;
                let current = monitor_counters.transfer_bytes.load(Ordering::Relaxed);
                let now = Instant::now();
                let elapsed = now.duration_since(last_time).as_secs_f64();
                let speed = if elapsed > 0.0 {
                    (current.saturating_sub(last_bytes)) as f64 / elapsed
                } else {
                    0.0
                };
                monitor_sink.speed(speed);
                last_bytes = current;
                last_time = now;
            }
        });

        // 阶段二：信号量并发下载去重后的队列
        let sem = Arc::new(Semaphore::new(self.concurrency));
        let mut set = tokio::task::JoinSet::new();
        let client = self.client.clone();
        let sink = self.sink.clone();
        let counters = self.counters.clone();
        let folder = user_folder.clone();

        for target in targets {
            let client = client.clone();
            let sink = sink.clone();
            let counters = counters.clone();
            let sem = sem.clone();
            let folder = folder.clone();
            set.spawn(async move {
                download_one(&sink, &client, sem, target, &folder, &counters, discovered_total).await;
            });
        }
        while set.join_next().await.is_some() {}

        speed_monitor.abort();
        self.sink.speed(0.0);
        emit_stats(&self.sink, &self.counters);

        let cancelled = self.is_stopped();
        if cancelled {
            self.log("\n>>> 已取消！");
        } else {
            let elapsed = start_time.elapsed().as_secs_f64();
            let mb = self.counters.bytes_downloaded.load(Ordering::Relaxed) as f64 / (1024.0 * 1024.0);
            let rate = if elapsed > 0.0 { mb / elapsed } else { 0.0 };
            self.log(&format!(
                "\n>>> 完成：新下载 {} · 跳过 {} · 失败 {}",
                self.counters.down.load(Ordering::Relaxed),
                self.counters.skip.load(Ordering::Relaxed),
                self.counters.fail.load(Ordering::Relaxed)
            ));
            self.log(&format!(">>> 耗时 {:.1}s · {:.2}MB · {:.2}MB/s", elapsed, mb, rate));
            self.log(&sep);
        }
        cancelled
    }
}

// 对应 download_one()
async fn download_one(
    sink: &Callbacks,
    client: &reqwest::Client,
    sem: Arc<Semaphore>,
    target: Target,
    user_folder: &Path,
    c: &Arc<Counters>,
    discovered_total: u64,
) {
    if c.is_stopped() {
        return;
    }
    let file_name = format!("{}_{}.{}", target.prefix, target.media_id, target.ext);
    let file_path = user_folder.join(&file_name);

    // 最终兜底：磁盘已有完整文件则跳过
    if let Ok(md) = tokio::fs::metadata(&file_path).await {
        if md.len() > 0 {
            c.skip.fetch_add(1, Ordering::Relaxed);
            c.processed.fetch_add(1, Ordering::Relaxed);
            emit_progress(sink, c, discovered_total);
            emit_stats(sink, c);
            return;
        }
    }

    let req_url = if target.ext == "mp4" {
        target.url.clone()
    } else {
        format!("{}?name=orig", target.url)
    };
    let partial_path = PathBuf::from(format!("{}.part", file_path.display()));
    let _ = tokio::fs::remove_file(&partial_path).await;

    for attempt in 0..RETRY_LIMIT {
        if c.is_stopped() {
            return;
        }
        let _permit = match sem.clone().acquire_owned().await {
            Ok(p) => p,
            Err(_) => return,
        };
        if c.is_stopped() {
            return;
        }
        match stream_to_file(client, &req_url, &partial_path, c).await {
            Ok(file_bytes) => {
                // 原子改名（Windows 上 rename 不覆盖，先移除旧目标）
                if file_path.exists() {
                    let _ = tokio::fs::remove_file(&file_path).await;
                }
                match tokio::fs::rename(&partial_path, &file_path).await {
                    Ok(_) => {
                        c.bytes_downloaded.fetch_add(file_bytes, Ordering::Relaxed);
                        if c.is_stopped() {
                            return;
                        }
                        c.down.fetch_add(1, Ordering::Relaxed);
                        c.processed.fetch_add(1, Ordering::Relaxed);
                        emit_progress(sink, c, discovered_total);
                        emit_stats(sink, c);
                        emit_log(sink, &format!("[成功] {}", file_name));
                        return;
                    }
                    Err(e) => {
                        let _ = tokio::fs::remove_file(&partial_path).await;
                        if attempt == RETRY_LIMIT - 1 && !c.is_stopped() {
                            c.fail.fetch_add(1, Ordering::Relaxed);
                            c.processed.fetch_add(1, Ordering::Relaxed);
                            emit_progress(sink, c, discovered_total);
                            emit_stats(sink, c);
                            emit_log(sink, &format!("[失败] {} 重命名失败: {}", file_name, e));
                            return;
                        }
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                }
            }
            Err(err) => {
                let _ = tokio::fs::remove_file(&partial_path).await;
                if err == "__cancelled__" {
                    return; // 用户取消：静默退出
                }
                if attempt == RETRY_LIMIT - 1 && !c.is_stopped() {
                    c.fail.fetch_add(1, Ordering::Relaxed);
                    c.processed.fetch_add(1, Ordering::Relaxed);
                    emit_progress(sink, c, discovered_total);
                    emit_stats(sink, c);
                    emit_log(sink, &format!("[失败] {} {}", file_name, err));
                    return;
                }
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }
}

// 流式下载到 .part 文件；成功返回字节数；特殊错误 "__cancelled__" 表示用户取消
async fn stream_to_file(
    client: &reqwest::Client,
    url: &str,
    partial: &Path,
    c: &Arc<Counters>,
) -> Result<u64, String> {
    let resp = client.get(url).send().await.map_err(|e| e.to_string())?;
    let status = resp.status().as_u16();
    if status != 200 {
        return Err(format!("HTTP {}", status));
    }
    let mut file = tokio::fs::File::create(partial).await.map_err(|e| e.to_string())?;
    let mut stream = resp.bytes_stream();
    let mut total: u64 = 0;
    loop {
        if c.is_stopped() {
            let _ = tokio::fs::remove_file(partial).await;
            return Err("__cancelled__".to_string());
        }
        let chunk = match tokio::time::timeout(CHUNK_TIMEOUT, stream.next()).await {
            Err(_) => return Err("读取超时(20秒)".to_string()),
            Ok(None) => break,
            Ok(Some(Err(e))) => return Err(e.to_string()),
            Ok(Some(Ok(bytes))) => bytes,
        };
        if chunk.is_empty() {
            continue;
        }
        file.write_all(&chunk).await.map_err(|e| e.to_string())?;
        let n = chunk.len() as u64;
        total += n;
        c.transfer_bytes.fetch_add(n, Ordering::Relaxed);
    }
    file.flush().await.map_err(|e| e.to_string())?;
    Ok(total)
}

// ----------------- 任务生命周期管理 -----------------

struct AppState {
    current_stop: TokioMutex<Option<Arc<AtomicBool>>>,
    running: AtomicBool,
}

// ----------------- Tauri 命令 -----------------

#[tauri::command]
fn get_config() -> Config {
    load_config()
}

#[tauri::command]
fn save_config(cfg: Config) {
    save_config_to_disk(&cfg);
}

#[tauri::command]
async fn start_download(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    params: DownloadParams,
) -> Result<(), String> {
    if state.running.swap(true, Ordering::SeqCst) {
        return Err("已有下载任务正在运行".into());
    }

    let counters = Arc::new(Counters::new());
    // 把取消标记挂到全局状态，cancel_download 才能实时中止下载器
    *state.current_stop.lock().await = Some(counters.stop.clone());

    let app_handle = app.clone();
    let params = DownloadParams {
        concurrency: params.concurrency.clamp(1, 16),
        ..params
    };

    tauri::async_runtime::spawn(async move {
        let sink = Callbacks::gui(app_handle.clone());
        let mut downloader = Downloader::new(sink, &params, counters);
        let cancelled = downloader.run().await;
        let _ = app_handle.emit("xdl-done", DonePayload { cancelled });

        // 复位运行状态，允许下一次任务
        let st = app_handle.state::<AppState>();
        st.running.store(false, Ordering::SeqCst);
        *st.current_stop.lock().await = None;
    });

    Ok(())
}

#[tauri::command]
async fn cancel_download(state: tauri::State<'_, AppState>) -> Result<bool, String> {
    let guard = state.current_stop.lock().await;
    if let Some(stop) = guard.as_ref() {
        stop.store(true, Ordering::SeqCst);
        Ok(true)
    } else {
        Ok(false)
    }
}

#[tauri::command]
async fn browse_folder(default_path: String) -> Option<String> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut dialog = rfd::FileDialog::new();
        if !default_path.is_empty() {
            if let Some(dir) = PathBuf::from(&default_path).parent() {
                dialog = dialog.set_directory(dir);
            }
        }
        dialog
            .pick_folder()
            .map(|p| p.to_string_lossy().into_owned())
    })
    .await
    .ok()
    .flatten()
}

#[tauri::command]
fn open_folder(path: String) -> Result<(), String> {
    // 规整路径：统一为反斜杠、去掉末尾分隔符。
    // 否则 explorer 在路径以反斜杠结尾且被加引号（含空格）时，结尾的反斜杠会把
    // 闭合引号"吃掉"，explorer 拿到无效路径后回退打开"文档"目录。
    let raw = path.replace('/', "\\");
    let target = raw.trim_end_matches('\\').to_string();

    let p = Path::new(&target);
    if p.is_dir() {
        std::process::Command::new("explorer")
            .arg(&target)
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("无法打开目录: {}", e))
    } else if let Some(parent) = p.parent() {
        // 账户子目录尚不存在：回退打开其上级保存位置，并提示先执行下载
        if parent.is_dir() {
            std::process::Command::new("explorer")
                .arg(parent)
                .spawn()
                .map(|_| ())
                .map_err(|e| format!("无法打开目录: {}", e))?;
            Err("该账户目录尚不存在，已为你打开保存位置。请先执行下载。".into())
        } else {
            Err("该目录尚不存在，请先执行下载。".into())
        }
    } else {
        Err("该目录尚不存在，请先执行下载。".into())
    }
}

// ----------------- CLI 自检模式（对应 Python 版 --selftest） -----------------
// 用法: x-media-downloader.exe --selftest
// 结果写入 exe 同目录 selftest_result.txt（release 版无控制台，凭文件核对）。

async fn selftest_inner(lines: &mut Vec<String>) -> bool {
    let cfg = load_config();
    lines.push(format!(
        "config.json: user_id={:?} save_path={:?} concurrency={}",
        cfg.user_id, cfg.save_path, cfg.concurrency
    ));
    if cfg.auth_token.is_empty() || cfg.ct0.is_empty() || cfg.user_id.is_empty() || cfg.save_path.is_empty() {
        lines.push("[失败] config.json 缺少 auth_token / ct0 / user_id / save_path".into());
        return false;
    }

    let params = DownloadParams {
        auth_token: cfg.auth_token.clone(),
        ct0: cfg.ct0.clone(),
        user_id: cfg.user_id.clone(),
        save_path: cfg.save_path.clone(),
        concurrency: cfg.concurrency.max(1),
        media_filter: "all".into(),
        time_range: String::new(),
    };
    let counters = Arc::new(Counters::new());
    let mut dl = Downloader::new(Callbacks::console(), &params, counters);

    lines.push("正在获取用户信息 (UserByScreenName)...".into());
    if !dl.fetch_user_info().await {
        lines.push("[失败] fetch_user_info 未通过（Cookie 无效或网络问题）".into());
        return false;
    }
    lines.push(format!(
        "[OK] 用户: {} (ID: {}) 媒体总数: {}",
        dl.user_info.name, dl.user_info.rest_id, dl.user_info.media_count
    ));

    lines.push("正在获取媒体时间线首页 (UserMedia)...".into());
    let page = match dl.get_download_urls().await {
        Some(p) => p,
        None => {
            lines.push("[失败] UserMedia 首页获取失败".into());
            return false;
        }
    };
    lines.push(format!("[OK] 首页解析到 {} 个媒体条目", page.len()));
    for t in page.iter().take(3) {
        lines.push(format!("  样例: {}_{}.{}", t.prefix, t.media_id, t.ext));
    }

    // 完整链路验证 1：单文件流式下载 + 原子改名
    let tmp_dir = std::env::temp_dir().join("xdl_selftest");
    if let Err(e) = std::fs::create_dir_all(&tmp_dir) {
        lines.push(format!("[警告] 无法创建临时目录，跳过下载验证: {}", e));
        return true;
    }
    if let Some(first) = page.first().cloned() {
        let req_url = if first.ext == "mp4" {
            first.url.clone()
        } else {
            format!("{}?name=orig", first.url)
        };
        let partial = tmp_dir.join("download.part");
        let _ = std::fs::remove_file(&partial);
        match stream_to_file(&dl.client, &req_url, &partial, &dl.counters).await {
            Ok(n) => {
                let final_path = tmp_dir.join(format!("{}_{}.{}", first.prefix, first.media_id, first.ext));
                match std::fs::rename(&partial, &final_path) {
                    Ok(_) => lines.push(format!(
                        "[OK] 单文件下载验证: {} ({:.2} MB)",
                        final_path.display(),
                        n as f64 / 1048576.0
                    )),
                    Err(e) => {
                        lines.push(format!("[失败] 重命名失败: {}", e));
                        return false;
                    }
                }
            }
            Err(e) => {
                lines.push(format!("[失败] 下载验证失败: {}", e));
                return false;
            }
        }
    }

    // 完整链路验证 2：走正式并发路径下载前 N 个，验证并发/统计/重命名
    let batch: Vec<Target> = page.iter().take(SELFTEST_BATCH).cloned().collect();
    if !batch.is_empty() {
        let batch_total = batch.len() as u64;
        let client = dl.client.clone();
        let counters = dl.counters.clone();
        let sink = Callbacks::console();
        let sem = Arc::new(Semaphore::new(4));
        let mut set = tokio::task::JoinSet::new();
        for target in batch.iter().cloned() {
            let client = client.clone();
            let counters = counters.clone();
            let sink = sink.clone();
            let sem = sem.clone();
            let folder = tmp_dir.clone();
            set.spawn(async move {
                download_one(&sink, &client, sem, target, &folder, &counters, batch_total).await;
            });
        }
        while set.join_next().await.is_some() {}

        let down = counters.down.load(Ordering::Relaxed);
        let skip = counters.skip.load(Ordering::Relaxed);
        let fail = counters.fail.load(Ordering::Relaxed);
        lines.push(format!(
            "[{}] 并发下载验证 (并发 4): 新下载 {} · 跳过 {} · 失败 {} / 共 {}",
            if fail == 0 { "OK" } else { "警告" },
            down,
            skip,
            fail,
            batch_total
        ));
        if fail > 0 {
            lines.push("[失败] 并发下载出现失败项".into());
            return false;
        }

        // 去重回环：重新扫描目录，应识别全部已下载文件的媒体 ID
        let seen = dl.scan_existing_files(&tmp_dir);
        let hit = batch.iter().filter(|t| seen.contains(&t.media_id)).count();
        let ok = hit == batch.len();
        lines.push(format!(
            "[{}] 去重回环验证: 扫描识别 {}/{} 个媒体 ID（再次运行不会重复下载）",
            if ok { "OK" } else { "失败" },
            hit,
            batch.len()
        ));
        if !ok {
            return false;
        }
    }
    true
}

/// 原生消息弹窗（对应 Python 版 messagebox.showwarning / showinfo / showerror）
#[tauri::command]
async fn show_message(title: String, message: String, level: String) {
    let level = match level.as_str() {
        "error" => rfd::MessageLevel::Error,
        "info" => rfd::MessageLevel::Info,
        _ => rfd::MessageLevel::Warning,
    };
    let _ = tauri::async_runtime::spawn_blocking(move || {
        let _ = rfd::MessageDialog::new()
            .set_title(&title)
            .set_description(&message)
            .set_level(level)
            .show();
    })
    .await;
}

/// 与 Python 版一致：固定 AppUserModelID，确保任务栏显示本程序图标而非宿主进程图标。
/// 直接用原始 FFI 调用 shell32，避免为此引入 windows 系列依赖。
#[cfg(windows)]
fn set_app_user_model_id() {
    #[link(name = "shell32")]
    extern "system" {
        fn SetCurrentProcessExplicitAppUserModelID(app_id: *const u16) -> i32;
    }
    let id: Vec<u16> = "x.media.downloader"
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    unsafe {
        SetCurrentProcessExplicitAppUserModelID(id.as_ptr());
    }
}

fn run_selftest() -> i32 {
    let out_path = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("selftest_result.txt")))
        .unwrap_or_else(|| PathBuf::from("selftest_result.txt"));

    let rt = match tokio::runtime::Runtime::new() {
        Ok(r) => r,
        Err(e) => {
            let _ = std::fs::write(&out_path, format!("SELFTEST_FAILED\n无法创建异步运行时: {}", e));
            return 1;
        }
    };

    let mut lines: Vec<String> = Vec::new();
    let passed = rt.block_on(selftest_inner(&mut lines));
    let status = if passed { "SELFTEST_PASSED" } else { "SELFTEST_FAILED" };
    lines.insert(0, status.to_string());
    let _ = std::fs::write(&out_path, lines.join("\n") + "\n");
    if passed { 0 } else { 1 }
}

fn main() {
    // CLI 自检模式：不打开窗口，跑真实链路并把结果写入 selftest_result.txt
    if std::env::args().any(|a| a == "--selftest") {
        std::process::exit(run_selftest());
    }

    #[cfg(windows)]
    set_app_user_model_id();

    tauri::Builder::default()
        .manage(AppState {
            current_stop: TokioMutex::new(None),
            running: AtomicBool::new(false),
        })
        .invoke_handler(tauri::generate_handler![
            get_config,
            save_config,
            start_download,
            cancel_download,
            browse_folder,
            open_folder,
            show_message
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
