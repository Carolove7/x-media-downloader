use crate::types::{MediaItem, UserInfo};
use chrono::{DateTime, NaiveDate, Utc};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, COOKIE, REFERER, USER_AGENT};
use reqwest::Client;
use serde_json::Value;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

const BEARER_TOKEN: &str = "Bearer AAAAAAAAAAAAAAAAAAAAANRILgAAAAAAnNwIzUejRCOuH5E6I8xnZz4puTs%3D1Zv7ttfk8LF81IUq16cHjhLTvJu4FA33AGWWjCpTnA";

pub struct TwitterClient {
    client: Client,
    screen_name: String,
    time_range: Option<(NaiveDate, NaiveDate)>,
    /// 实际使用的网络模式描述（直连 / 环境变量代理 / 系统代理），用于日志展示。
    pub proxy_desc: String,
}

impl TwitterClient {
    pub fn new(auth_token: &str, ct0: &str, screen_name: &str, time_range: &str) -> Result<Self, String> {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"));
        headers.insert(AUTHORIZATION, HeaderValue::from_static(BEARER_TOKEN));
        headers.insert("x-csrf-token", HeaderValue::from_str(ct0).map_err(|e| e.to_string())?);

        let cookie_str = format!("auth_token={auth_token}; ct0={ct0};");
        headers.insert(COOKIE, HeaderValue::from_str(&cookie_str).map_err(|e| e.to_string())?);
        headers.insert(REFERER, HeaderValue::from_str(&format!("https://twitter.com/{screen_name}")).map_err(|e| e.to_string())?);

        // 注意：不设置 reqwest 的 .timeout()（它是"整个请求含 body 读取"的总超时），
        // 否则下载大图/视频时超过 30 秒必然被掐断（Python 参考版 httpx 的 timeout
        // 只是"读空闲超时"，不限制总时长）。这里只限制连接建立时间，防挂起。
        // 请求级超时由调用方用 tokio::time::timeout 包裹（GraphQL 20s / 下载 600s）。
        //
        // 代理：reqwest 默认只读环境变量代理，不读 Windows 系统代理（WinINET 注册表）。
        // 大陆网络环境下 twitter.com 直连不可达，必须显式读取系统代理才能工作。
        let mut builder = Client::builder()
            .default_headers(headers)
            .connect_timeout(std::time::Duration::from_secs(15));
        let proxy_desc = match detect_system_proxy() {
            Some((proxy, desc)) => {
                builder = builder.proxy(proxy);
                desc
            }
            None => "直连（未检测到代理）".to_string(),
        };
        let client = builder.build().map_err(|e| e.to_string())?;

        let range = parse_time_range(time_range);

        Ok(Self {
            client,
            screen_name: screen_name.trim_start_matches('@').to_string(),
            time_range: range,
            proxy_desc,
        })
    }

    /// 发送 GraphQL 请求并把整个响应体读回为字符串。
    /// 关键：`.send()` 与 `read_body`（读 body 流）**都**包在超时里——
    /// 否则代理偶发截断大响应体后，reqwest 会一直等待更多数据，任务永久挂起，
    /// 表现为 UI 卡在"正在下载…"。超时后由调用方重试（对齐参考版 retry=3）。
    async fn send_and_read(&self, url: &str, secs: u64) -> Result<String, String> {
        let resp = match tokio::time::timeout(
            std::time::Duration::from_secs(secs),
            self.client.get(url).send(),
        )
        .await
        {
            Ok(Ok(r)) => r,
            Ok(Err(e)) => return Err(format!("请求失败: {e}")),
            Err(_) => return Err(format!("请求超时（{secs} 秒未响应），请检查网络或代理设置")),
        };
        let status = resp.status();
        if status == 401 {
            return Err("Cookie 无效或已过期，请重新填写 auth_token 和 ct0".into());
        }
        if status == 429 {
            return Err("已触发 Twitter API 请求速率限制 (429 Rate Limit)，请稍后再试".into());
        }
        match tokio::time::timeout(std::time::Duration::from_secs(secs), read_body(resp)).await {
            Ok(Ok(b)) => Ok(b),
            Ok(Err(e)) => Err(e),
            Err(_) => Err(format!(
                "读取响应超时（{secs} 秒），可能是网络/代理不稳定导致传输中断，将在重试后恢复"
            )),
        }
    }

    pub async fn fetch_user_info(&self) -> Result<UserInfo, String> {
        let url = format!(
            r#"https://twitter.com/i/api/graphql/xc8f1g7BYqr6VTzTbvNlGw/UserByScreenName?variables={{"screen_name":"{}","withSafetyModeUserFields":false}}&features={{"hidden_profile_likes_enabled":false,"hidden_profile_subscriptions_enabled":false,"responsive_web_graphql_exclude_directive_enabled":true,"verified_phone_label_enabled":false,"subscriptions_verification_info_verified_since_enabled":true,"highlights_tweets_tab_ui_enabled":true,"creator_subscriptions_tweet_preview_api_enabled":true,"responsive_web_graphql_skip_user_profile_image_extensions_enabled":false,"responsive_web_graphql_timeline_navigation_enabled":true}}&fieldToggles={{"withAuxiliaryUserLabels":false}}"#,
            self.screen_name
        );

        let quoted = quote_url(&url);
        eprintln!("[DIAG] fetch_user_info URL length: {}, quoted length: {}", url.len(), quoted.len());
        let body = match self.send_and_read(&quoted, 20).await {
            Ok(b) => b,
            Err(e) => {
                crate::log_to_file("error", &format!("[DIAG] fetch_user_info FAILED: {e}"));
                return Err(e);
            }
        };
        crate::log_to_file("info", "[DIAG] fetch_user_info OK");
        let json: Value = serde_json::from_str(&body)
            .map_err(|e| format!("JSON 解析错误: {e} | 响应片段: {}", &body.chars().take(200).collect::<String>()))?;
        if let Some(msg) = get_graphql_error(&json) {
            return Err(format!("Twitter 返回错误: {msg}"));
        }
        let user_res = &json["data"]["user"]["result"];
        let legacy = &user_res["legacy"];

        let rest_id = user_res["rest_id"].as_str().ok_or("无法找到 rest_id")?.to_string();
        let name = legacy["name"].as_str().unwrap_or("未知").to_string();
        let statuses_count = legacy["statuses_count"].as_u64().unwrap_or(0);
        let media_count = legacy["media_count"].as_u64().unwrap_or(0);

        Ok(UserInfo {
            screen_name: self.screen_name.clone(),
            rest_id,
            name,
            statuses_count,
            media_count,
        })
    }

    pub async fn fetch_media_page(&self, rest_id: &str, cursor: Option<&str>) -> Result<(Vec<MediaItem>, Option<String>), String> {
        let cursor_query = match cursor {
            Some(c) => format!(r#","cursor":"{}""#, c),
            None => "".to_string(),
        };

        let url = format!(
            r#"https://twitter.com/i/api/graphql/Le6KlbilFmSu-5VltFND-Q/UserMedia?variables={{"userId":"{}","count":500{},"includePromotedContent":false,"withClientEventToken":false,"withBirdwatchNotes":false,"withVoice":true,"withV2Timeline":true}}&features={{"responsive_web_graphql_exclude_directive_enabled":true,"verified_phone_label_enabled":false,"creator_subscriptions_tweet_preview_api_enabled":true,"responsive_web_graphql_timeline_navigation_enabled":true,"responsive_web_graphql_skip_user_profile_image_extensions_enabled":false,"tweetypie_unmention_optimization_enabled":true,"responsive_web_edit_tweet_api_enabled":true,"graphql_is_translatable_rweb_tweet_is_translatable_enabled":true,"view_counts_everywhere_api_enabled":true,"longform_notetweets_consumption_enabled":true,"responsive_web_twitter_article_tweet_consumption_enabled":false,"tweet_awards_web_tipping_enabled":false,"freedom_of_speech_not_reach_fetch_enabled":true,"standardized_nudges_misinfo":true,"tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled":true,"longform_notetweets_rich_text_read_enabled":true,"longform_notetweets_inline_media_enabled":true,"responsive_web_media_download_video_enabled":false,"responsive_web_enhance_cards_enabled":false}}"#,
            rest_id, cursor_query
        );

        let quoted = quote_url(&url);
        eprintln!("[DIAG] fetch_media_page URL length: {}, cursor: {}", url.len(), cursor.is_some());
        let body = match self.send_and_read(&quoted, 20).await {
            Ok(b) => b,
            Err(e) => {
                crate::log_to_file("error", &format!("[DIAG] fetch_media_page FAILED: {e}"));
                return Err(e);
            }
        };
        crate::log_to_file("info", &format!("[DIAG] fetch_media_page 收到响应 {}B", body.len()));
        let json: Value = serde_json::from_str(&body)
            .map_err(|e| format!("解析媒体数据错误: {e} | 响应片段: {}", &body.chars().take(200).collect::<String>()))?;
        if let Some(msg) = get_graphql_error(&json) {
            return Err(format!("Twitter 返回错误: {msg}"));
        }

        // —— 以下解析严格对齐 Python 参考版 main.py 的 UserMedia 默认模式 ——
        let instructions = json["data"]["user"]["result"]["timeline_v2"]["timeline"]["instructions"]
            .as_array()
            .ok_or("未找到时间线 instructions")?;

        let is_first = cursor.is_none();
        let mut content_items: Vec<Value> = Vec::new();
        let mut next_cursor: Option<String> = None;

        if is_first {
            // 第一页：instructions 最后一个 instruction 的 entries[0].content.items
            if let Some(last_instr) = instructions.last() {
                if let Some(entries) = last_instr["entries"].as_array() {
                    if let Some(first_entry) = entries.first() {
                        if let Some(items) = first_entry["content"]["items"].as_array() {
                            content_items = items.iter().cloned().collect();
                        }
                    }
                    // 第一页的下一页 cursor 在 instructions[-1].entries 的 bottom 里
                    for e in entries {
                        let eid = e["entryId"].as_str().unwrap_or("");
                        if eid.contains("bottom") {
                            next_cursor = e["content"]["value"]
                                .as_str()
                                .or_else(|| e["content"]["cursor"]["value"].as_str())
                                .map(|s| s.to_string());
                        }
                    }
                }
            }
        } else if let Some(first_instr) = instructions.first() {
            // 后续页：instructions[0].moduleItems
            if let Some(mod_items) = first_instr["moduleItems"].as_array() {
                content_items = mod_items.iter().cloned().collect();
                for i in mod_items {
                    let eid = i["entryId"].as_str().unwrap_or("");
                    if eid.contains("bottom") {
                        next_cursor = i["content"]["value"]
                            .as_str()
                            .or_else(|| i["content"]["cursor"]["value"].as_str())
                            .map(|s| s.to_string());
                    }
                }
            } else {
                // 后续页无 moduleItems = 全部媒体已拉取完毕
                return Ok((Vec::new(), None));
            }
        }

        // 对齐参考版 get_url_from_content（x_label='item'）：逐项提取媒体并记录 cursor-bottom
        let mut items: Vec<MediaItem> = Vec::new();
        for i in &content_items {
            let entry_id = i["entryId"].as_str().unwrap_or("");
            if entry_id.contains("promoted-tweet") {
                continue;
            }
            if entry_id.contains("cursor-bottom") || entry_id.contains("bottom") {
                next_cursor = i["content"]["value"]
                    .as_str()
                    .or_else(|| i["content"]["cursor"]["value"].as_str())
                    .map(|s| s.to_string());
                continue;
            }

            // 取 tweet_result：普通推文走 i['item']['itemContent']，对话线走 item.items[0]
            let item_content = i["item"]["itemContent"].clone();
            let tweet_result = if entry_id.contains("profile-conversation") {
                i["item"]["items"][0]["item"]["itemContent"]["tweet_results"]["result"].clone()
            } else {
                item_content["tweet_results"]["result"].clone()
            };
            if tweet_result.is_null() {
                continue;
            }
            let tweet = if tweet_result.get("tweet").map_or(false, |v| v.is_object()) {
                tweet_result["tweet"].clone()
            } else {
                tweet_result
            };
            let legacy = &tweet["legacy"];
            if legacy.is_null() {
                continue;
            }

            // 时间范围过滤（按 UTC 日期比较）
            if let Some((start, end)) = self.time_range {
                let in_range = parse_tweet_date(legacy.get("created_at"))
                    .map(|date| date >= start && date <= end)
                    .unwrap_or(false);
                if !in_range {
                    continue;
                }
            }

            let timestr = parse_tweet_date(legacy.get("created_at"))
                .map(|date| date.format("%Y-%m-%d %H-%M").to_string())
                .unwrap_or_else(|| chrono::Local::now().format("%Y-%m-%d %H-%M").to_string());

            let media_arr = legacy["extended_entities"]["media"]
                .as_array()
                .or_else(|| legacy["entities"]["media"].as_array());

            if let Some(media_list) = media_arr {
                for m in media_list {
                    if let Some(variants) = m["video_info"]["variants"].as_array() {
                        let mut best_url: Option<String> = None;
                        let mut max_bitrate: i64 = -1;
                        for v in variants {
                            let bitrate = v["bitrate"].as_i64().unwrap_or(-1);
                            if let Some(u) = v["url"].as_str() {
                                if bitrate > max_bitrate {
                                    max_bitrate = bitrate;
                                    best_url = Some(u.to_string());
                                }
                            }
                        }
                        if let Some(url) = best_url {
                            let media_id = extract_media_id(&url, "mp4");
                            items.push(MediaItem {
                                url,
                                filename: format!("{timestr}-vid"),
                                ext: "mp4".into(),
                                media_id,
                            });
                        }
                    } else if let Some(img_url) = m["media_url_https"].as_str() {
                        let media_id = extract_media_id(img_url, "jpg");
                        items.push(MediaItem {
                            url: img_url.to_string(),
                            filename: format!("{timestr}-img"),
                            ext: "jpg".into(),
                            media_id,
                        });
                    }
                }
            }
        }

        Ok((items, next_cursor))
    }

    /// 暴露底层 reqwest client，供 downloader 复用（携带完整鉴权头）。
    pub fn client(&self) -> &Client {
        &self.client
    }

}

/// 解析时间范围字符串 "YYYY-MM-DD:YYYY-MM-DD"，返回 (start, end) 本地日期。
fn parse_time_range(s: &str) -> Option<(NaiveDate, NaiveDate)> {
    if s.is_empty() || !s.contains(':') {
        return None;
    }
    let parts: Vec<&str> = s.splitn(2, ':').collect();
    let start = NaiveDate::parse_from_str(parts[0].trim(), "%Y-%m-%d").ok()?;
    let end = NaiveDate::parse_from_str(parts[1].trim(), "%Y-%m-%d").ok()?;
    let (start, end) = if start > end { (end, start) } else { (start, end) };
    Some((start, end))
}

/// 对 URL 中的花括号做百分号编码（GraphQL 查询与媒体下载 URL 均需使用）。
/// Python 参考版对每个请求 URL（含下载 URL）都做同样的处理。
pub fn quote_url(url: &str) -> String {
    url.replace('{', "%7B").replace('}', "%7D")
}

/// 检测应使用的代理。
/// 优先级：环境变量（reqwest 默认已自动使用，直接返回 None 让其生效）→
/// Windows 系统代理（WinINET 注册表，浏览器"系统代理"设置）→ 无。
/// 返回 (代理, 描述)。
fn detect_system_proxy() -> Option<(reqwest::Proxy, String)> {
    const ENV_KEYS: [&str; 6] = [
        "http_proxy", "HTTP_PROXY", "https_proxy", "HTTPS_PROXY", "all_proxy", "ALL_PROXY",
    ];
    let env_set = ENV_KEYS
        .iter()
        .any(|k| std::env::var(k).map(|v| !v.trim().is_empty()).unwrap_or(false));
    if env_set {
        // reqwest 已自动使用环境变量代理，无需显式设置
        return None;
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(server) = read_wininet_proxy_server() {
            let (scheme, addr) = parse_proxy_server(&server);
            let url = format!("{scheme}://{addr}");
            match reqwest::Proxy::all(&url) {
                Ok(p) => return Some((p, format!("系统代理 {url}"))),
                Err(_) => return None,
            }
        }
    }

    None
}

/// 读取 Windows 注册表中的 WinINET 代理设置（即浏览器/系统"使用代理服务器"配置）。
#[cfg(target_os = "windows")]
fn read_wininet_proxy_server() -> Option<String> {
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ};
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu
        .open_subkey_with_flags(
            r"Software\Microsoft\Windows\CurrentVersion\Internet Settings",
            KEY_READ,
        )
        .ok()?;
    let enabled: u32 = key.get_value("ProxyEnable").ok()?;
    if enabled == 0 {
        return None;
    }
    let server: String = key.get_value("ProxyServer").ok()?;
    if server.trim().is_empty() {
        return None;
    }
    Some(server)
}

/// 解析 ProxyServer 的两种格式：
/// - "127.0.0.1:10808"（所有协议共用）
/// - "http=127.0.0.1:10809;https=127.0.0.1:10809;socks=127.0.0.1:10808"（按协议）
/// 返回 (scheme, host:port)。
fn parse_proxy_server(server: &str) -> (&'static str, String) {
    if server.contains('=') {
        let mut https = None;
        let mut http = None;
        let mut socks = None;
        for part in server.split(';') {
            if let Some((k, v)) = part.split_once('=') {
                let v = v.trim().to_string();
                if v.is_empty() {
                    continue;
                }
                match k.trim().to_ascii_lowercase().as_str() {
                    "https" => https = Some(v),
                    "http" => http = Some(v),
                    "socks" | "socks5" => socks = Some(v),
                    _ => {}
                }
            }
        }
        if let Some(v) = https.or(http) {
            return ("http", v);
        }
        if let Some(v) = socks {
            return ("socks5", v);
        }
    }
    ("http", server.trim().to_string())
}

/// 读取响应体并做健壮化校验：空响应 / 非 JSON 响应都会给出可诊断的错误。
async fn read_body(resp: reqwest::Response) -> Result<String, String> {
    let status = resp.status();
    let body = resp.text().await.map_err(|e| format!("读取响应失败: {e}"))?;
    if body.trim().is_empty() {
        return Err(format!(
            "Twitter 返回空响应 (HTTP {status})，可能是网络被拦截、账号被风控或 Cookie 已失效"
        ));
    }
    if !body.trim_start().starts_with('{') && !body.trim_start().starts_with('[') {
        return Err(format!(
            "Twitter 返回了非 JSON 响应 (HTTP {status})，片段: {}",
            &body.chars().take(200).collect::<String>()
        ));
    }
    Ok(body)
}

/// 提取 GraphQL 响应中的首个 errors 消息（若有），便于快速定位风控/限流/账号问题。
fn get_graphql_error(json: &Value) -> Option<String> {
    json.get("errors")
        .and_then(|e| e.as_array())
        .and_then(|arr| arr.first())
        .and_then(|err| err.get("message").and_then(|m| m.as_str()))
        .map(|s| s.to_string())
}

fn parse_tweet_date(created_at: Option<&Value>) -> Option<NaiveDate> {
    let s = created_at?.as_str()?;
    let dt = DateTime::parse_from_str(s, "%a %b %d %H:%M:%S %z %Y").ok()?;
    Some(dt.with_timezone(&Utc).date_naive())
}

/// 从媒体 URL 提取稳定且唯一的标识，用于文件名与去重。
fn extract_media_id(url: &str, ext: &str) -> String {
    let base = url.split('?').next().unwrap_or(url);

    if ext == "mp4" {
        // 优先匹配 ext_tw_video/<id>
        if let Some(pos) = base.find("ext_tw_video/") {
            let rest = &base[pos + 13..];
            let id: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            if !id.is_empty() {
                return id;
            }
        }
        // 兜底：连续 10 位以上数字 ID
        let digits: Vec<&str> = base.rsplit('/').collect();
        for part in digits {
            let num: String = part.chars().take_while(|c| c.is_ascii_digit()).collect();
            if num.len() >= 10 {
                return num;
            }
        }
    } else {
        // 图片：/media/<id>
        if let Some(pos) = base.find("/media/") {
            let rest = &base[pos + 7..];
            let id: String = rest
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
                .collect();
            if !id.is_empty() {
                return id;
            }
        }
    }

    // 最终兜底：URL 路径（不含查询）生成稳定短哈希
    let mut hasher = DefaultHasher::new();
    base.hash(&mut hasher);
    format!("{:x}", hasher.finish())[..12].to_string()
}
