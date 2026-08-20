#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod downloader;
mod twitter_client;
mod types;

use chrono::{Datelike, Local};
use downloader::DownloadManager;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Manager, State};
use types::{AppConfig, LogPayload};

const RECENT_USER_ID_LIMIT: usize = 7;

struct AppState {
    downloader: Mutex<Option<DownloadManager>>,
}

fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_default()
        .join("twitter_downloader_config.json")
}

/// 日志落盘：GUI 模式下 eprintln 完全不可见，写文件便于事后诊断。
/// 文件位置：%APPDATA%\twitter_downloader_log.txt
pub fn log_to_file(level: &str, msg: &str) {
    use std::io::Write;
    let path = dirs::config_dir()
        .unwrap_or_default()
        .join("twitter_downloader_log.txt");
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        let now = Local::now().format("%Y-%m-%d %H:%M:%S");
        let _ = writeln!(f, "[{now}] [{level}] {msg}");
    }
}

#[tauri::command]
async fn load_config() -> Result<AppConfig, String> {
    let path = config_path();
    if path.exists() {
        let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        serde_json::from_str(&content).map_err(|e| e.to_string())
    } else {
        Ok(default_config())
    }
}

#[tauri::command]
async fn save_config(mut config: AppConfig) -> Result<(), String> {
    let path = config_path();

    // 维护最近账户 ID 列表（最多 7 个，去重，当前排到最前）
    let user_id = config.user_id.trim().to_string();
    let mut recent: Vec<String> = Vec::new();
    if !user_id.is_empty() {
        recent.push(user_id);
    }
    for value in config.recent_user_ids.iter() {
        let v = value.trim().to_string();
        if !v.is_empty() && !recent.contains(&v) {
            recent.push(v);
            if recent.len() >= RECENT_USER_ID_LIMIT {
                break;
            }
        }
    }
    config.recent_user_ids = recent;

    let content = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
    std::fs::write(path, content).map_err(|e| e.to_string())
}

#[tauri::command]
async fn start_download(app: AppHandle, state: State<'_, AppState>, config: AppConfig) -> Result<(), String> {
    let manager = DownloadManager::new();
    {
        let mut lock = state.downloader.lock().unwrap();
        *lock = Some(manager);
    }

    let mgr = {
        let lock = state.downloader.lock().unwrap();
        lock.as_ref().unwrap().clone()
    };

    tokio::spawn(async move {
        if let Err(e) = mgr.run(app.clone(), config).await {
            let now = Local::now().format("%H:%M:%S").to_string();
            let msg = format!(">>> 任务执行出错，已终止: {e}");
            log_to_file("error", &msg);
            let _ = app.emit_all(
                "download-log",
                LogPayload {
                    level: "error".into(),
                    message: msg,
                    timestamp: now.clone(),
                },
            );
            let hint = ">>> 请检查 Cookie 是否有效、网络能否访问 twitter.com".to_string();
            log_to_file("error", &hint);
            let _ = app.emit_all(
                "download-log",
                LogPayload {
                    level: "error".into(),
                    message: hint,
                    timestamp: now,
                },
            );
        }
    });

    Ok(())
}

#[tauri::command]
async fn cancel_download(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let lock = state.downloader.lock().unwrap();
    if let Some(ref mgr) = *lock {
        mgr.cancel();
    }
    drop(lock);
    // 立即给出取消反馈，避免用户点击后“毫无反应”的错觉
    let now = Local::now().format("%H:%M:%S").to_string();
    let _ = app.emit_all(
        "download-log",
        LogPayload {
            level: "warn".into(),
            message: ">>> 收到取消指令，正在中断队列…".into(),
            timestamp: now,
        },
    );
    Ok(())
}

#[tauri::command]
async fn open_download_dir(_app: AppHandle, config: AppConfig) -> Result<(), String> {
    let screen_name = config.user_id.trim_start_matches('@');
    let dir = PathBuf::from(&config.save_path).join(screen_name);
    // 目录不存在则尝试打开父目录
    let open_path = if dir.exists() { dir } else { PathBuf::from(&config.save_path) };
    let path_str = open_path.to_string_lossy().to_string();

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&path_str)
            .spawn()
            .map_err(|e| format!("打开目录失败: {e}"))?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&path_str)
            .spawn()
            .map_err(|e| format!("打开目录失败: {e}"))?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&path_str)
            .spawn()
            .map_err(|e| format!("打开目录失败: {e}"))?;
    }
    Ok(())
}

fn default_config() -> AppConfig {
    AppConfig {
        auth_token: "".into(),
        ct0: "".into(),
        user_id: "".into(),
        save_path: "".into(),
        concurrency: Some(8),
        proxy: None,
        recent_user_ids: Vec::new(),
        media_filter: "all".into(),
        unlimited_time: true,
        start_year: 1990,
        start_month: 1,
        start_day: 1,
        end_year: chrono::Local::now().year() as u32,
        end_month: chrono::Local::now().month(),
        end_day: chrono::Local::now().day(),
    }
}

fn main() {
    tauri::Builder::default()
        .manage(AppState {
            downloader: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            load_config,
            save_config,
            start_download,
            cancel_download,
            open_download_dir
        ])
        .run(tauri::generate_context!())
        .expect("运行 Tauri 应用程序出错");
}
