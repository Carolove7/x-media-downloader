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

        let client = Client::builder()
            .default_headers(headers)
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .map_err(|e| e.to_string())?;

        let range = parse_time_range(time_range);

        Ok(Self {
            client,
            screen_name: screen_name.trim_start_matches('@').to_string(),
            time_range: range,
        })
    }

    pub async fn fetch_user_info(&self) -> Result<UserInfo, String> {
        let url = format!(
            r#"https://twitter.com/i/api/graphql/xc8f1g7BYqr6VTzTbvNlGw/UserByScreenName?variables={{"screen_name":"{}","withSafetyModeUserFields":false}}&features={{"hidden_profile_likes_enabled":false,"hidden_profile_subscriptions_enabled":false,"responsive_web_graphql_exclude_directive_enabled":true,"verified_phone_label_enabled":false,"subscriptions_verification_info_verified_since_enabled":true,"highlights_tweets_tab_ui_enabled":true,"creator_subscriptions_tweet_preview_api_enabled":true,"responsive_web_graphql_skip_user_profile_image_extensions_enabled":false,"responsive_web_graphql_timeline_navigation_enabled":true}}&fieldToggles={{"withAuxiliaryUserLabels":false}}"#,
            self.screen_name
        );

        let resp = self.client.get(&url).send().await.map_err(|e| format!("请求失败: {e}"))?;
        if resp.status() == 401 {
            return Err("Cookie 无效或已过期，请重新填写 auth_token 和 ct0".into());
        }

        let json: Value = resp.json().await.map_err(|e| format!("JSON 解析错误: {e}"))?;
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

        let resp = self.client.get(&url).send().await.map_err(|e| format!("获取媒体列表失败: {e}"))?;
        if resp.status() == 429 {
            return Err("已触发 Twitter API 请求速率限制 (429 Rate Limit)，请稍后再试".into());
        }

        let json: Value = resp.json().await.map_err(|e| format!("解析媒体数据错误: {e}"))?;
        let instructions = json["data"]["user"]["result"]["timeline_v2"]["timeline"]["instructions"]
            .as_array()
            .or_else(|| json["data"]["user"]["result"]["timeline"]["timeline"]["instructions"].as_array())
            .ok_or("未找到时间线数据")?;

        let mut items = Vec::new();
        let mut next_cursor = None;

        for instr in instructions {
            // 某些响应把条目放在 instruction.moduleItems 里
            if let Some(module_items) = instr["moduleItems"].as_array() {
                for item in module_items {
                    Self::extract_medias(item, &mut items, &mut next_cursor, self.time_range);
                }
            }

            if let Some(entries) = instr["entries"].as_array() {
                for entry in entries {
                    let entry_id = entry["entryId"].as_str().unwrap_or("");
                    if entry_id.contains("bottom") || entry_id.contains("cursor-bottom") {
                        next_cursor = entry["content"]["value"]
                            .as_str()
                            .or_else(|| entry["content"]["cursor"]["value"].as_str())
                            .map(|s| s.to_string());
                        continue;
                    }

                    // 处理 content.items / content.moduleItems / content.itemContent 三种形态
                    if let Some(nested) = entry["content"]["items"].as_array() {
                        for item in nested {
                            Self::extract_medias(item, &mut items, &mut next_cursor, self.time_range);
                        }
                    } else if let Some(nested) = entry["content"]["moduleItems"].as_array() {
                        for item in nested {
                            Self::extract_medias(item, &mut items, &mut next_cursor, self.time_range);
                        }
                    } else {
                        Self::extract_medias(entry, &mut items, &mut next_cursor, self.time_range);
                    }
                }
            }
        }

        Ok((items, next_cursor))
    }

    fn extract_medias(entry: &Value, items: &mut Vec<MediaItem>, _cursor: &mut Option<String>, time_range: Option<(NaiveDate, NaiveDate)>) {
        let entry_id = entry["entryId"].as_str().unwrap_or("");
        if entry_id.contains("bottom") || entry_id.contains("cursor-bottom") {
            return;
        }

        // 先尝试 entry.item.itemContent，再尝试 entry.itemContent
        let item_content = entry["item"]["itemContent"]
            .as_object()
            .map(|_| &entry["item"]["itemContent"])
            .unwrap_or(&entry["itemContent"]);

        if item_content.is_null() || item_content["tweet_results"].is_null() {
            return;
        }

        let tweet_result = match item_content["tweet_results"]["result"].as_object() {
            Some(o) => o,
            None => return,
        };
        let tweet = tweet_result.get("tweet").and_then(|v| v.as_object()).unwrap_or(tweet_result);
        let legacy = &tweet["legacy"];
        if legacy.is_null() {
            return;
        }

        // 时间范围过滤（按 UTC 日期比较）
        if let Some((start, end)) = time_range {
            let in_range = parse_tweet_date(legacy.get("created_at"))
                .map(|date| date >= start && date <= end)
                .unwrap_or(false);
            if !in_range {
                return;
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
                    let mut max_bitrate: i64 = -1;
                    let mut best_url = String::new();
                    for v in variants {
                        let bitrate = v["bitrate"].as_i64().unwrap_or(-1);
                        if let Some(u) = v["url"].as_str() {
                            if bitrate > max_bitrate {
                                max_bitrate = bitrate;
                                best_url = u.to_string();
                            }
                        }
                    }
                    if !best_url.is_empty() {
                        let media_id = extract_media_id(&best_url, "mp4");
                        items.push(MediaItem {
                            url: best_url,
                            filename: format!("{timestr}-vid"),
                            ext: "mp4".into(),
                            media_id,
                        });
                    }
                } else if let Some(img_url) = m["media_url_https"].as_str() {
                    let media_id = extract_media_id(img_url, "jpg");
                    items.push(MediaItem {
                        url: format!("{img_url}?name=orig"),
                        filename: format!("{timestr}-img"),
                        ext: "jpg".into(),
                        media_id,
                    });
                }
            }
        }
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

/// 从推文 created_at 字段（如 "Wed Jun 10 12:00:00 +0000 2020"）解析 UTC 日期。
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
