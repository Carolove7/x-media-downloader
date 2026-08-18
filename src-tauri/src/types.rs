use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppConfig {
    pub auth_token: String,
    pub ct0: String,
    pub user_id: String,
    pub save_path: String,
    pub concurrency: Option<usize>,
}

#[derive(Debug, Serialize, Clone)]
pub struct UserInfo {
    pub screen_name: String,
    pub rest_id: String,
    pub name: String,
    pub statuses_count: u64,
    pub media_count: u64,
}

#[derive(Debug, Clone)]
pub struct MediaItem {
    pub url: String,
    pub filename: String,
    pub ext: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct ProgressPayload {
    pub current: usize,
    pub total: usize,
    pub downloaded: usize,
    pub skipped: usize,
    pub percent: f64,
}

#[derive(Debug, Serialize, Clone)]
pub struct LogPayload {
    pub level: String,
    pub message: String,
    pub timestamp: String,
}
