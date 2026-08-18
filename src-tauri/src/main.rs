#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod downloader;
mod twitter_client;
mod types;

use chrono::Local;
use downloader::DownloadManager;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{api::shell, AppHandle, State};
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
            let _ = app.emit_all(
                "download-log",
                LogPayload {
                    level: "error".into(),
                    message: format!(">>> 任务执行出错，已终止: {e}"),
                    timestamp: now.clone(),
                },
            );
            let _ = app.emit_all(
                "download-log",
                LogPayload {
                    level: "error".into(),
                    message: ">>> 请检查 Cookie 是否有效、网络能否访问 twitter.com".into(),
                    timestamp: now,
                },
            );
        }
    });

    Ok(())
}

#[tauri::command]
async fn cancel_download(state: State<'_, AppState>) -> Result<(), String> {
    let lock = state.downloader.lock().unwrap();
    if let Some(ref mgr) = *lock {
        mgr.cancel();
    }
    Ok(())
}

#[tauri::command]
async fn open_download_dir(app: AppHandle, config: AppConfig) -> Result<(), String> {
    let screen_name = config.user_id.trim_start_matches('@');
    let dir = PathBuf::from(&config.save_path).join(screen_name);
    // 目录不存在则尝试打开父目录
    let open_path = if dir.exists() { dir } else { PathBuf::from(&config.save_path) };
    let path_str = open_path.to_string_lossy().to_string();

    let scope = app.shell_scope();
    shell::open(&scope, &path_str, None).map_err(|e| format!("打开目录失败: {e}"))?;
    Ok(())
}

fn default_config() -> AppConfig {
    AppConfig {
        auth_token: "".into(),
        ct0: "".into(),
        user_id: "".into(),
        save_path: "".into(),
        concurrency: Some(8),
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
