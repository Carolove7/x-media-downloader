use crate::types::{MediaItem, UserInfo};
use chrono::Local;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, COOKIE, REFERER, USER_AGENT};
use reqwest::Client;
use serde_json::Value;

const BEARER_TOKEN: &str = "Bearer AAAAAAAAAAAAAAAAAAAAANRILgAAAAAAnNwIzUejRCOuH5E6I8xnZz4puTs%3D1Zv7ttfk8LF81IUq16cHjhLTvJu4FA33AGWWjCpTnA";

pub struct TwitterClient {
    client: Client,
    screen_name: String,
}

impl TwitterClient {
    pub fn new(auth_token: &str, ct0: &str, screen_name: &str) -> Result<Self, String> {
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

        Ok(Self {
            client,
            screen_name: screen_name.trim_start_matches('@').to_string(),
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
            if let Some(entries) = instr["entries"].as_array() {
                for entry in entries {
                    let entry_id = entry["entryId"].as_str().unwrap_or("");
                    if entry_id.contains("bottom") || entry_id.contains("cursor-bottom") {
                        next_cursor = entry["content"]["value"]
                            .as_str()
                            .or_else(|| entry["content"]["cursor"]["value"].as_str())
                            .map(|s| s.to_string());
                    }

                    if let Some(tweet_res) = entry["content"]["itemContent"]["tweet_results"]["result"].as_object() {
                        let tweet = tweet_res.get("tweet").and_then(|v| v.as_object()).unwrap_or(tweet_res);
                        let legacy = &tweet["legacy"];
                        
                        let time_str = tweet.get("edit_control")
                            .and_then(|e| e["editable_until_msecs"].as_str())
                            .and_then(|ms| ms.parse::<i64>().ok())
                            .map(|msecs| {
                                let dt = chrono::DateTime::from_timestamp_millis(msecs - 3600000).unwrap_or_else(|| Local::now().into());
                                dt.format("%Y-%m-%d %H-%M").to_string()
                            })
                            .unwrap_or_else(|| Local::now().format("%Y-%m-%d %H-%M").to_string());

                        let media_arr = legacy["extended_entities"]["media"]
                            .as_array()
                            .or_else(|| legacy["entities"]["media"].as_array());

                        if let Some(media_list) = media_arr {
                            for m in media_list {
                                if let Some(variants) = m["video_info"]["variants"].as_array() {
                                    let mut max_bitrate = 0;
                                    let mut best_url = String::new();
                                    for v in variants {
                                        let bitrate = v["bitrate"].as_u64().unwrap_or(0);
                                        if bitrate >= max_bitrate {
                                            max_bitrate = bitrate;
                                            if let Some(u) = v["url"].as_str() {
                                                best_url = u.to_string();
                                            }
                                        }
                                    }
                                    if !best_url.is_empty() {
                                        items.push(MediaItem {
                                            url: best_url,
                                            filename: format!("{time_str}-vid"),
                                            ext: "mp4".into(),
                                        });
                                    }
                                } else if let Some(img_url) = m["media_url_https"].as_str() {
                                    items.push(MediaItem {
                                        url: format!("{img_url}?name=orig"),
                                        filename: format!("{time_str}-img"),
                                        ext: "jpg".into(),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok((items, next_cursor))
    }
}
