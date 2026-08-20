use crate::types::{MediaItem, UserInfo};
use chrono::{DateTime, NaiveDate, Utc};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, COOKIE, REFERER, USER_AGENT};
use reqwest::Client;
use serde_json::Value;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

const BEARER_TOKEN: &str = "Bearer AAAAAAAAAAAAAAAAAAAAANRILgAAAAAAnNwIzUejRCOuH5E6I8xnZz4puTs%3D1Zv7ttfk8LF81IUq16cHjhLTvJu4FA33AGWWjCpTnA";

// 与参考 Python 完全一致的 Query ID
const USER_BY_SCREEN_NAME_QID: &str = "xc8f1g7BYqr6VTzTbvNlGw";
const USER_MEDIA_QID: &str = "Le6KlbilFmSu-5VltFND-Q";

pub fn quote_url(url: &str) -> String {
    url.replace('{', "%7B").replace('}', "%7D")
}

pub struct TwitterClient {
    client: Client,
    screen_name: String,
    time_range: Option<(NaiveDate, NaiveDate)>,
    pub proxy_desc: String,
}

impl TwitterClient {
    pub fn new(
        auth_token: &str,
        ct0: &str,
        screen_name: &str,
        time_range: &str,
    ) -> Result<Self, String> {
        let auth_token = auth_token.trim();
        let ct0 = ct0.trim();
        let screen_name = screen_name.trim().trim_start_matches('@').to_string();

        let cookie_str = format!("auth_token={auth_token}; ct0={ct0};");

        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/114.0.0.0 Safari/537.36",
            ),
        );
        headers.insert(AUTHORIZATION, HeaderValue::from_static(BEARER_TOKEN));
        headers.insert(COOKIE, HeaderValue::from_str(&cookie_str).map_err(|e| e.to_string())?);
        headers.insert("x-csrf-token", HeaderValue::from_str(ct0).map_err(|e| e.to_string())?);
        headers.insert(
            REFERER,
            HeaderValue::from_str(&format!("https://twitter.com/{screen_name}"))
                .map_err(|e| e.to_string())?,
        );

        let client = Client::builder()
            .default_headers(headers)
            .connect_timeout(std::time::Duration::from_secs(15))
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| e.to_string())?;

        let range = parse_time_range(time_range);

        Ok(Self {
            client,
            screen_name,
            time_range: range,
            proxy_desc: "系统默认".to_string(),
        })
    }

    pub fn media_client(&self) -> &Client {
        &self.client
    }

    /// 获取用户信息 — 与 Python fetch_user_info 完全对齐
    pub async fn fetch_user_info(&self) -> Result<UserInfo, String> {
        let url = format!(
            "https://twitter.com/i/api/graphql/{}/UserByScreenName\
             ?variables={{\"screen_name\":\"{}\",\"withSafetyModeUserFields\":false}}\
             &features={{\"hidden_profile_likes_enabled\":false,\"hidden_profile_subscriptions_enabled\":false,\
             \"responsive_web_graphql_exclude_directive_enabled\":true,\"verified_phone_label_enabled\":false,\
             \"subscriptions_verification_info_verified_since_enabled\":true,\"highlights_tweets_tab_ui_enabled\":true,\
             \"creator_subscriptions_tweet_preview_api_enabled\":true,\
             \"responsive_web_graphql_timeline_navigation_enabled\":true}}\
             &fieldToggles={{\"withAuxiliaryUserLabels\":false}}",
            USER_BY_SCREEN_NAME_QID,
            self.screen_name
        );

        let mut last_err = String::new();
        for attempt in 1..=3 {
            match self.get_json(&quote_url(&url), 10).await {
                Ok(json) => {
                    if let Some(errs) = json.get("errors").and_then(|e| e.as_array()) {
                        if !errs.is_empty() {
                            let msg = errs[0].get("message").and_then(|m| m.as_str()).unwrap_or("未知错误");
                            last_err = format!("接口返回错误: {msg}");
                            continue;
                        }
                    }

                    let user_res = &json["data"]["user"]["result"];
                    if user_res.is_null() {
                        return Err(format!("未找到账户 @{}（可能不存在、被封禁或无权限访问）", self.screen_name));
                    }

                    let rest_id = user_res["rest_id"]
                        .as_str()
                        .ok_or_else(|| "无法获取 rest_id".to_string())?
                        .to_string();

                    let legacy = &user_res["legacy"];
                    let name = legacy["name"]
                        .as_str()
                        .unwrap_or(&self.screen_name)
                        .to_string();
                    let statuses_count = legacy["statuses_count"].as_u64().unwrap_or(0);
                    let media_count = legacy["media_count"].as_u64().unwrap_or(0);

                    return Ok(UserInfo {
                        screen_name: self.screen_name.clone(),
                        rest_id,
                        name,
                        statuses_count,
                        media_count,
                    });
                }
                Err(e) => {
                    last_err = e.clone();
                    crate::log_to_file("warn", &format!("[DIAG] fetch_user_info attempt {attempt} failed: {e}"));
                    if attempt < 3 {
                        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
                    }
                }
            }
        }

        Err(format!("获取用户信息失败: {last_err}"))
    }

    /// 获取媒体列表分页 — 与 Python get_download_urls 完全对齐
    pub async fn fetch_media_page(
        &self,
        rest_id: &str,
        cursor: Option<&str>,
    ) -> Result<(Vec<MediaItem>, Option<String>), String> {
        let url_top = format!(
            "https://twitter.com/i/api/graphql/{}/UserMedia?variables={{\"userId\":\"{}\",\"count\":500,",
            USER_MEDIA_QID, rest_id
        );
        let url_bottom = "\"includePromotedContent\":false,\"withClientEventToken\":false,\"withBirdwatchNotes\":false,\"withVoice\":true,\"withV2Timeline\":true}\
            &features={\"responsive_web_graphql_exclude_directive_enabled\":true,\"verified_phone_label_enabled\":false,\
            \"creator_subscriptions_tweet_preview_api_enabled\":true,\"responsive_web_graphql_timeline_navigation_enabled\":true,\
            \"responsive_web_graphql_skip_user_profile_image_extensions_enabled\":false,\"tweetypie_unmention_optimization_enabled\":true,\
            \"responsive_web_edit_tweet_api_enabled\":true,\"graphql_is_translatable_rweb_tweet_is_translatable_enabled\":true,\
            \"view_counts_everywhere_api_enabled\":true,\"longform_notetweets_consumption_enabled\":true,\
            \"responsive_web_twitter_article_tweet_consumption_enabled\":false,\"tweet_awards_web_tipping_enabled\":false,\
            \"freedom_of_speech_not_reach_fetch_enabled\":true,\"standardized_nudges_misinfo\":true,\
            \"tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled\":true,\
            \"longform_notetweets_rich_text_read_enabled\":true,\"longform_notetweets_inline_media_enabled\":true,\
            \"responsive_web_media_download_video_enabled\":false,\"responsive_web_enhance_cards_enabled\":false}";

        let url = if let Some(c) = cursor {
            format!("{url_top}\"cursor\":\"{c}\",{url_bottom}")
        } else {
            format!("{url_top}{url_bottom}")
        };

        for attempt in 1..=3 {
            match self.get_json(&quote_url(&url), 10).await {
                Ok(json) => {
                    if json.get("errors").and_then(|e| e.as_array()).map_or(false, |e| !e.is_empty()) {
                        if let Some(e) = json["errors"][0].get("message").and_then(|m| m.as_str()) {
                            if e.contains("Rate limit") || e.contains("429") {
                                return Err("触发 API 频率限制 (429)，请稍后重试".to_string());
                            }
                        }
                    }
                    return self.parse_timeline_response(&json);
                }
                Err(e) => {
                    crate::log_to_file("warn", &format!("[DIAG] fetch_media_page attempt {attempt} failed: {e}"));
                    if attempt < 3 {
                        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
                    } else {
                        return Err(format!("获取媒体列表失败: {e}"));
                    }
                }
            }
        }

        Err("获取媒体列表失败（重试用尽）".to_string())
    }

    /// 解析时间线响应 — 与 Python get_download_urls 中的解析逻辑完全对齐
    fn parse_timeline_response(&self, json: &Value) -> Result<(Vec<MediaItem>, Option<String>), String> {
        let user_res = json.pointer("/data/user/result")
            .ok_or_else(|| "未找到 data.user.result".to_string())?;

        let timeline = user_res
            .pointer("/timeline_v2/timeline")
            .or_else(|| user_res.pointer("/timeline/timeline"))
            .ok_or_else(|| "未找到 timeline 字段".to_string())?;

        let instructions = timeline["instructions"]
            .as_array()
            .ok_or_else(|| "未找到 instructions".to_string())?;

        let mut items: Vec<&Value> = Vec::new();
        let mut next_cursor: Option<String> = None;

        for instr in instructions {
            let entries = instr.get("entries").and_then(|e| e.as_array());
            let single_entry = if entries.is_none() { instr.get("entry") } else { None };

            let all_entries: Vec<&Value> = entries
                .map(|e| e.iter().collect())
                .or_else(|| single_entry.map(|e| vec![e]))
                .unwrap_or_default();

            for entry in &all_entries {
                let entry_id = entry.get("entryId").and_then(|v| v.as_str()).unwrap_or("");

                // 识别 bottom cursor（分页游标）
                if entry_id.contains("bottom") || entry_id.contains("cursor-bottom") {
                    let content = &entry["content"];
                    let cursor_val = content.get("value")
                        .or_else(|| content.pointer("/cursor/value"))
                        .and_then(|v| v.as_str());
                    if let Some(cv) = cursor_val {
                        next_cursor = Some(cv.to_string());
                    }
                }

                let content = &entry["content"];
                if let Some(sub_items) = content.get("items").and_then(|v| v.as_array()) {
                    items.extend(sub_items.iter());
                } else if let Some(mod_items) = content.get("moduleItems").and_then(|v| v.as_array()) {
                    items.extend(mod_items.iter());
                } else if content.get("itemContent").is_some() {
                    items.push(entry);
                }
            }

            // 处理 moduleItems 在 instruction 级别
            if let Some(mod_items) = instr.get("moduleItems").and_then(|v| v.as_array()) {
                items.extend(mod_items.iter());
            }
        }

        let mut media_items: Vec<MediaItem> = Vec::new();

        for item in items {
            // 取 entry_obj：兼容 item/items 嵌套格式
            let entry_obj = item.get("item").unwrap_or(item);
            let item_content = match entry_obj.get("itemContent") {
                Some(ic) => ic,
                None => continue,
            };

            if item_content.get("tweet_results").is_none() {
                continue;
            }

            let tweet_result = match item_content["tweet_results"].get("result") {
                Some(r) if !r.is_null() => r,
                _ => continue,
            };

            // 解包 TweetWithVisibilityResults 或普通 tweet 字段
            let tweet_obj = tweet_result.get("tweet").unwrap_or(tweet_result);
            let legacy = match tweet_obj.get("legacy") {
                Some(l) if l.is_object() => l,
                _ => continue,
            };

            // 解析发布时间
            let created_at = legacy.get("created_at").and_then(|v| v.as_str());
            let tweet_date = created_at.and_then(|s| {
                DateTime::parse_from_str(s, "%a %b %d %H:%M:%S +0000 %Y")
                    .ok()
                    .map(|dt| dt.with_timezone(&Utc).date_naive())
            });

            // 时间范围过滤
            if let Some((start, end)) = self.time_range {
                match tweet_date {
                    Some(d) if d >= start && d <= end => {}
                    Some(_) => continue,
                    None => continue,
                }
            }

            let timestr = tweet_date
                .map(|d| d.format("%Y-%m-%d %H-%M").to_string())
                .unwrap_or_else(|| chrono::Local::now().format("%Y-%m-%d %H-%M").to_string());

            // 提取媒体列表
            let media_list = legacy
                .pointer("/extended_entities/media")
                .or_else(|| legacy.pointer("/entities/media"))
                .and_then(|v| v.as_array());

            let media_list = match media_list {
                Some(m) => m,
                None => continue,
            };

            for media in media_list {
                if let Some(video_info) = media.get("video_info") {
                    // 视频：取最高码率
                    let variants = video_info.get("variants").and_then(|v| v.as_array());
                    if let Some(vid_url) = get_highest_video_quality(variants) {
                        let media_id = extract_media_id(&vid_url, "mp4");
                        media_items.push(MediaItem {
                            url: vid_url,
                            filename: format!("{timestr}-vid"),
                            ext: "mp4".to_string(),
                            media_id,
                        });
                    }
                } else if let Some(img_url) = media.get("media_url_https").and_then(|v| v.as_str()) {
                    let media_id = extract_media_id(img_url, "jpg");
                    media_items.push(MediaItem {
                        url: img_url.to_string(),
                        filename: format!("{timestr}-img"),
                        ext: "jpg".to_string(),
                        media_id,
                    });
                }
            }
        }

        Ok((media_items, next_cursor))
    }

    /// 发送 GET 请求并解析为 JSON
    async fn get_json(&self, url: &str, timeout_secs: u64) -> Result<Value, String> {
        let resp = match tokio::time::timeout(
            std::time::Duration::from_secs(timeout_secs),
            self.client.get(url).send(),
        )
        .await
        {
            Ok(Ok(r)) => r,
            Ok(Err(e)) => return Err(format!("网络请求失败: {e}")),
            Err(_) => return Err(format!("请求超时（{timeout_secs} 秒）")),
        };

        let status = resp.status();
        if status.as_u16() == 401 {
            return Err("Cookie 无效或已过期，请检查 auth_token 及 ct0".to_string());
        }
        if status.as_u16() == 429 {
            return Err("触发 API 频率限制 (429 Rate limit)，请稍后重试".to_string());
        }

        let body = match tokio::time::timeout(
            std::time::Duration::from_secs(timeout_secs),
            resp.text(),
        )
        .await
        {
            Ok(Ok(b)) => b,
            Ok(Err(e)) => return Err(format!("读取响应失败: {e}")),
            Err(_) => return Err("读取响应超时".to_string()),
        };

        if body.trim().is_empty() {
            return Err(format!("返回空响应 (HTTP {status})，请检查 Cookie 是否有效"));
        }

        serde_json::from_str(&body).map_err(|e| {
            let preview = &body.chars().take(200).collect::<String>();
            format!("JSON 解析错误: {e}，响应片段: {preview}")
        })
    }
}

/// 取最高码率的视频 URL（对齐 Python get_heighest_video_quality）
fn get_highest_video_quality(variants: Option<&Vec<Value>>) -> Option<String> {
    let variants = variants?;
    if variants.is_empty() {
        return None;
    }
    if variants.len() == 1 {
        return variants[0].get("url").and_then(|v| v.as_str()).map(|s| s.to_string());
    }

    let mut max_bitrate = -1i64;
    let mut best_url: Option<String> = None;

    for v in variants {
        if let Some(bitrate) = v.get("bitrate").and_then(|b| b.as_i64()) {
            if bitrate > max_bitrate {
                max_bitrate = bitrate;
                best_url = v.get("url").and_then(|u| u.as_str()).map(|s| s.to_string());
            }
        }
    }

    best_url.or_else(|| {
        variants[0].get("url").and_then(|v| v.as_str()).map(|s| s.to_string())
    })
}

/// 解析时间范围字符串
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

/// 从媒体 URL 提取唯一标识（对齐 Python extract_media_id）
pub fn extract_media_id(url: &str, ext: &str) -> String {
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
        let parts: Vec<&str> = base.rsplit('/').collect();
        for part in parts {
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
