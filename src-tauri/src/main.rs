#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod downloader;
mod twitter_client;
mod types;

use downloader::DownloadManager;
use std::sync::Mutex;
use tauri::{AppHandle, State};
use types::AppConfig;

struct AppState {
    downloader: Mutex<Option<DownloadManager>>,
}

#[tauri::command]
async fn load_config() -> Result<AppConfig, String> {
    let path = dirs::config_dir()
        .unwrap_or_default()
        .join("twitter_downloader_config.json");
    if path.exists() {
        let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        serde_json::from_str(&content).map_err(|e| e.to_string())
    } else {
        Ok(AppConfig {
            auth_token: "".into(),
            ct0: "".into(),
            user_id: "".into(),
            save_path: "".into(),
            concurrency: Some(8),
        })
    }
}

#[tauri::command]
async fn save_config(config: AppConfig) -> Result<(), String> {
    let path = dirs::config_dir()
        .unwrap_or_default()
        .join("twitter_downloader_config.json");
    let content = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
    std::fs::write(path, content).map_err(|e| e.to_string())
}

#[tauri::command]
async fn start_download(app: AppHandle, state: State<'_', AppState>, config: AppConfig) -> Result<(), String> {
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
        let _ = mgr.run(app, config).await;
    });

    Ok(())
}

#[tauri::command]
async fn cancel_download(state: State<'_', AppState>) -> Result<(), String> {
    let lock = state.downloader.lock().unwrap();
    if let Some(ref mgr) = *lock {
        mgr.cancel();
    }
    Ok(())
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
            cancel_download
        ])
        .run(tauri::generate_context!())
        .expect("运行 Tauri 应用程序出错");
}
