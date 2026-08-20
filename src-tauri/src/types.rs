use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppConfig {
    pub auth_token: String,
    pub ct0: String,
    pub user_id: String,
    pub save_path: String,
    pub concurrency: Option<usize>,
    #[serde(default)]
    pub proxy: Option<String>,
    #[serde(default)]
    pub recent_user_ids: Vec<String>,
    #[serde(default = "default_media_filter")]
    pub media_filter: String,
    #[serde(default = "default_unlimited_time")]
    pub unlimited_time: bool,
    #[serde(default)]
    pub start_year: u32,
    #[serde(default = "default_start_month")]
    pub start_month: u32,
    #[serde(default = "default_start_day")]
    pub start_day: u32,
    #[serde(default)]
    pub end_year: u32,
    #[serde(default = "default_end_month")]
    pub end_month: u32,
    #[serde(default = "default_end_day")]
    pub end_day: u32,
}

fn default_media_filter() -> String { "all".into() }
fn default_unlimited_time() -> bool { true }
fn default_start_month() -> u32 { 1 }
fn default_start_day() -> u32 { 1 }
fn default_end_month() -> u32 { 12 }
fn default_end_day() -> u32 { 31 }

impl AppConfig {
    /// 根据起止年月日生成 "YYYY-MM-DD:YYYY-MM-DD" 时间范围字符串。
    /// 若勾选“不限时间”或日期无效，返回空字符串。
    pub fn time_range(&self) -> String {
        if self.unlimited_time {
            return "".into();
        }
        let (sy, sm, sd) = normalize_date(self.start_year, self.start_month, self.start_day);
        let (ey, em, ed) = normalize_date(self.end_year, self.end_month, self.end_day);
        format!("{:04}-{:02}-{:02}:{:04}-{:02}-{:02}", sy, sm, sd, ey, em, ed)
    }
}

/// 把年月日收敛到合法日期（越界天数按当月最大天数处理）。
fn normalize_date(y: u32, m: u32, d: u32) -> (u32, u32, u32) {
    let y = y.max(1990).min(2100);
    let m = m.clamp(1, 12);
    let dim = days_in_month(y, m);
    let d = d.clamp(1, dim);
    (y, m, d)
}

fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0) {
                29
            } else {
                28
            }
        }
        _ => 31,
    }
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
    pub media_id: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct ProgressPayload {
    pub current: usize,
    pub total: usize,
    pub downloaded: usize,
    pub skipped: usize,
    pub failed: usize,
    pub percent: f64,
    pub speed: f64, // bytes per second
}

#[derive(Debug, Serialize, Clone)]
pub struct LogPayload {
    pub level: String,
    pub message: String,
    pub timestamp: String,
}
