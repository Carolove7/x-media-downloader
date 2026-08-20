use crate::types::{MediaItem, UserInfo};
use chrono::{DateTime, NaiveDate, Utc};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, COOKIE, REFERER, USER_AGENT};
use reqwest::Client;
use serde_json::Value;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

const BEARER_TOKEN: &str = "Bearer AAAAAAAAAAAAAAAAAAAAANRILgAAAAAAnNwIzUejRCOuH5E6I8xnZz4puTs%3D1Zv7ttfk8LF81IUq16cHjhLTvJu4FA33AGWWjCpTnA";

// 预置候选 GraphQL Query IDs（按最新验证有效性排序，支持自动回退探测）
const USER_BY_SCREEN_NAME_CANDIDATES: &[&str] = &[
    "Gb-d6r0vxPOADdG62OEBpQ",
    "NimuplG1OB7Fd2btCLdBOw",
    "sLVLhk0bGj3MVFEKTdax1w",
    "6-O8jgJcxldwzgFwGbeX4A",
    "xc8f1g7BYqr6VTzTbvNlGw",
];

const USER_MEDIA_CANDIDATES: &[&str] = &[
    "2DC9TKrcUzwGC_QskSVl5w",
    "2tLOJWwGuCTytDrGBg8VwQ",
    "jCRhbOzdgOHp6u9H4g2tEg",
    "Le6KlbilFmSu-5VltFND-Q",
];

const USER_TWEETS_CANDIDATES: &[&str] = &[
    "eoJ5zbv51Z_KVl81v9PmLQ",
    "QWF3SzpHmykQHsQMixG0cg",
    "HuTx74BxAIF5GQ4DISn4Ug",
];

pub struct TwitterClient {
    client: Client,
    media_client: Client,
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
        custom_proxy: Option<&str>,
    ) -> Result<Self, String> {
        let auth_token = auth_token.trim();
        let ct0 = ct0.trim();
        let screen_name = screen_name.trim().trim_start_matches('@').to_string();

        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
            ),
        );
        headers.insert(AUTHORIZATION, HeaderValue::from_static(BEARER_TOKEN));
        headers.insert("x-csrf-token", HeaderValue::from_str(ct0).map_err(|e| e.to_string())?);
        headers.insert("x-twitter-active-user", HeaderValue::from_static("yes"));
        headers.insert("x-twitter-auth-type", HeaderValue::from_static("OAuth2Session"));
        headers.insert("x-twitter-client-language", HeaderValue::from_static("en"));
        headers.insert(
            "accept-language",
            HeaderValue::from_static("en-US,en;q=0.9,zh-CN;q=0.8,zh;q=0.7"),
        );
        headers.insert(
            "sec-ch-ua",
            HeaderValue::from_static(r#""Google Chrome";v="131", "Chromium";v="131", "Not_A Brand";v="24""#),
        );
        headers.insert("sec-ch-ua-mobile", HeaderValue::from_static("?0"));
        headers.insert("sec-ch-ua-platform", HeaderValue::from_static(r#""Windows""#));
        headers.insert("sec-fetch-dest", HeaderValue::from_static("empty"));
        headers.insert("sec-fetch-mode", HeaderValue::from_static("cors"));
        headers.insert("sec-fetch-site", HeaderValue::from_static("same-origin"));

        let cookie_str = format!("auth_token={auth_token}; ct0={ct0};");
        headers.insert(COOKIE, HeaderValue::from_str(&cookie_str).map_err(|e| e.to_string())?);
        headers.insert(
            REFERER,
            HeaderValue::from_str(&format!("https://x.com/{screen_name}")).map_err(|e| e.to_string())?,
        );

        let mut builder = Client::builder()
            .default_headers(headers)
            .connect_timeout(std::time::Duration::from_secs(15));

        let mut media_builder = Client::builder()
            .connect_timeout(std::time::Duration::from_secs(15));

        let proxy_desc = match resolve_proxy(custom_proxy) {
            Some((proxy_obj, desc)) => {
                builder = builder.proxy(proxy_obj.clone());
                media_builder = media_builder.proxy(proxy_obj);
                desc
            }
            None => "直连 / 系统默认".to_string(),
        };

        let client = builder.build().map_err(|e| e.to_string())?;
        let media_client = media_builder.build().map_err(|e| e.to_string())?;
        let range = parse_time_range(time_range);

        Ok(Self {
            client,
            media_client,
            screen_name,
            time_range: range,
            proxy_desc,
        })
    }

    /// 发送 GraphQL 请求并解析响应体
    async fn send_and_read(&self, url: reqwest::Url, secs: u64) -> Result<String, String> {
        let resp = match tokio::time::timeout(
            std::time::Duration::from_secs(secs),
            self.client.get(url).send(),
        )
        .await
        {
            Ok(Ok(r)) => r,
            Ok(Err(e)) => return Err(format!("网络请求失败: {e}")),
            Err(_) => return Err(format!("请求超时（{secs} 秒未响应），请检查代理与网络连接")),
        };

        let status = resp.status();
        if status == 401 {
            return Err("Cookie 无效或已过期，请重新获取并填写 auth_token 与 ct0".into());
        }
        if status == 429 {
            return Err("触发 X (Twitter) API 速率限制 (429 Rate Limit)，请稍后再试".into());
        }

        match tokio::time::timeout(std::time::Duration::from_secs(secs), read_body(resp)).await {
            Ok(Ok(b)) => Ok(b),
            Ok(Err(e)) => Err(e),
            Err(_) => Err(format!("读取响应超时（{secs} 秒），可能网络中断")),
        }
    }

    /// 获取用户信息（自动尝试候选 queryId，失败时自动尝试动态提取）
    pub async fn fetch_user_info(&self) -> Result<UserInfo, String> {
        // 先尝试动态嗅探最新 queryId（非阻塞）
        let dynamic_ops = self.try_extract_dynamic_ops().await;

        let mut candidate_ids: Vec<String> = Vec::new();
        if let Some(ref ops) = dynamic_ops {
            if let Some(id) = ops.get("UserByScreenName") {
                candidate_ids.push(id.clone());
            }
        }
        for &id in USER_BY_SCREEN_NAME_CANDIDATES {
            if !candidate_ids.iter().any(|c| c == id) {
                candidate_ids.push(id.to_string());
            }
        }

        let mut last_err = String::new();

        for qid in &candidate_ids {
            let mut url = match reqwest::Url::parse(&format!("https://x.com/i/api/graphql/{qid}/UserByScreenName")) {
                Ok(u) => u,
                Err(_) => continue,
            };

            let variables = serde_json::json!({
                "screen_name": self.screen_name,
                "withSafetyModeUserFields": true
            });

            let features = get_default_features();
            let field_toggles = serde_json::json!({
                "withAuxiliaryUserLabels": false
            });

            url.query_pairs_mut()
                .append_pair("variables", &variables.to_string())
                .append_pair("features", &features.to_string())
                .append_pair("fieldToggles", &field_toggles.to_string());

            match self.send_and_read(url, 20).await {
                Ok(body) => {
                    let json: Value = match serde_json::from_str(&body) {
                        Ok(j) => j,
                        Err(e) => {
                            last_err = format!("JSON 解析错误: {e}");
                            continue;
                        }
                    };

                    if let Some(msg) = get_graphql_error(&json) {
                        last_err = format!("Twitter 返回错误: {msg}");
                        continue;
                    }

                    let user_res = &json["data"]["user"]["result"];
                    if user_res.is_null() {
                        last_err = "无法找到用户数据，请检查账户 ID 是否正确".into();
                        continue;
                    }

                    if user_res.get("__typename").and_then(|v| v.as_str()) == Some("UserUnavailable") {
                        let reason = user_res.get("reason").and_then(|v| v.as_str()).unwrap_or("用户不可用（可能已被封禁或注销）");
                        return Err(format!("账户不可用: {reason}"));
                    }

                    let rest_id = user_res["rest_id"]
                        .as_str()
                        .or_else(|| user_res["id"].as_str())
                        .ok_or_else(|| "无法获取 rest_id".to_string())?
                        .to_string();

                    let legacy = &user_res["legacy"];
                    let name = legacy["name"]
                        .as_str()
                        .or_else(|| user_res["name"].as_str())
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
                    last_err = e;
                }
            }
        }

        Err(format!("获取用户信息失败（已尝试所有接口）: {last_err}"))
    }

    /// 获取媒体列表分页（优先 UserMedia，若不可用自动降级至 UserTweets 全量媒体提取）
    pub async fn fetch_media_page(
        &self,
        rest_id: &str,
        cursor: Option<&str>,
    ) -> Result<(Vec<MediaItem>, Option<String>), String> {
        // 1. 尝试 UserMedia
        match self.fetch_user_media_internal(rest_id, cursor).await {
            Ok(res) => return Ok(res),
            Err(e) => {
                crate::log_to_file("warn", &format!("[DIAG] UserMedia 请求未成功 ({e})，尝试 UserTweets 备用通道"));
            }
        }

        // 2. 备用通道：UserTweets
        self.fetch_user_tweets_internal(rest_id, cursor).await
    }

    async fn fetch_user_media_internal(
        &self,
        rest_id: &str,
        cursor: Option<&str>,
    ) -> Result<(Vec<MediaItem>, Option<String>), String> {
        let mut candidate_ids: Vec<String> = USER_MEDIA_CANDIDATES.iter().map(|s| s.to_string()).collect();
        let mut last_err = String::new();

        for qid in candidate_ids.drain(..) {
            let mut url = match reqwest::Url::parse(&format!("https://x.com/i/api/graphql/{qid}/UserMedia")) {
                Ok(u) => u,
                Err(_) => continue,
            };

            let mut vars_obj = serde_json::json!({
                "userId": rest_id,
                "count": 40,
                "includePromotedContent": false,
                "withClientEventToken": false,
                "withBirdwatchNotes": false,
                "withVoice": true,
                "withV2Timeline": true
            });

            if let Some(c) = cursor {
                vars_obj["cursor"] = serde_json::Value::String(c.to_string());
            }

            let features = get_default_features();

            url.query_pairs_mut()
                .append_pair("variables", &vars_obj.to_string())
                .append_pair("features", &features.to_string());

            match self.send_and_read(url, 25).await {
                Ok(body) => {
                    let json: Value = match serde_json::from_str(&body) {
                        Ok(j) => j,
                        Err(e) => {
                            last_err = format!("JSON 解析错误: {e}");
                            continue;
                        }
                    };

                    if let Some(msg) = get_graphql_error(&json) {
                        last_err = format!("Twitter 返回错误: {msg}");
                        continue;
                    }

                    return self.parse_timeline_response(&json);
                }
                Err(e) => {
                    last_err = e;
                }
            }
        }

        Err(last_err)
    }

    async fn fetch_user_tweets_internal(
        &self,
        rest_id: &str,
        cursor: Option<&str>,
    ) -> Result<(Vec<MediaItem>, Option<String>), String> {
        let candidate_ids: Vec<String> = USER_TWEETS_CANDIDATES.iter().map(|s| s.to_string()).collect();
        let mut last_err = String::new();

        for qid in candidate_ids {
            let mut url = match reqwest::Url::parse(&format!("https://x.com/i/api/graphql/{qid}/UserTweets")) {
                Ok(u) => u,
                Err(_) => continue,
            };

            let mut vars_obj = serde_json::json!({
                "userId": rest_id,
                "count": 40,
                "includePromotedContent": false,
                "withQuickPromoteEligibilityTweetFields": true,
                "withVoice": true,
                "withV2Timeline": true
            });

            if let Some(c) = cursor {
                vars_obj["cursor"] = serde_json::Value::String(c.to_string());
            }

            let features = get_default_features();

            url.query_pairs_mut()
                .append_pair("variables", &vars_obj.to_string())
                .append_pair("features", &features.to_string());

            match self.send_and_read(url, 25).await {
                Ok(body) => {
                    let json: Value = match serde_json::from_str(&body) {
                        Ok(j) => j,
                        Err(e) => {
                            last_err = format!("JSON 解析错误: {e}");
                            continue;
                        }
                    };

                    if let Some(msg) = get_graphql_error(&json) {
                        last_err = format!("Twitter 返回错误: {msg}");
                        continue;
                    }

                    return self.parse_timeline_response(&json);
                }
                Err(e) => {
                    last_err = e;
                }
            }
        }

        Err(format!("备用接口获取推文失败: {last_err}"))
    }

    /// 全量解析时间线 JSON 结构（兼容所有类型的 instructions / entries / modules / items）
    fn parse_timeline_response(&self, json: &Value) -> Result<(Vec<MediaItem>, Option<String>), String> {
        let instructions = json
            .pointer("/data/user/result/timeline_v2/timeline/instructions")
            .or_else(|| json.pointer("/data/user/result/timeline/timeline/instructions"))
            .or_else(|| json.pointer("/data/user/result/timeline/instructions"))
            .and_then(|v| v.as_array())
            .ok_or("未找到时间线 instructions 字段")?;

        let mut items = Vec::new();
        let mut next_cursor: Option<String> = None;

        for instr in instructions {
            let instr_type = instr.get("type").and_then(|v| v.as_str()).unwrap_or("");

            // 1. 处理 entries 列表（TimelineAddEntries 等）
            if let Some(entries) = instr.get("entries").and_then(|v| v.as_array()) {
                for entry in entries {
                    self.parse_entry_or_module(entry, &mut items, &mut next_cursor);
                }
            }

            // 2. 处理 moduleItems / items（TimelineAddToModule 等）
            if let Some(mod_items) = instr.get("moduleItems").or_else(|| instr.get("items")).and_then(|v| v.as_array()) {
                for item_val in mod_items {
                    self.parse_entry_or_module(item_val, &mut items, &mut next_cursor);
                }
            }

            // 3. 处理单 entry（TimelinePinEntry 等）
            if let Some(entry) = instr.get("entry") {
                self.parse_entry_or_module(entry, &mut items, &mut next_cursor);
            }

            // 4. 处理指令自带的 cursor
            if instr_type.contains("Cursor") || instr_type.contains("Bottom") {
                if let Some(c) = instr.get("value").or_else(|| instr.get("cursor")).and_then(|v| v.as_str()) {
                    next_cursor = Some(c.to_string());
                }
            }
        }

        Ok((items, next_cursor))
    }

    fn parse_entry_or_module(&self, obj: &Value, items: &mut Vec<MediaItem>, next_cursor: &mut Option<String>) {
        let entry_id = obj.get("entryId").and_then(|v| v.as_str()).unwrap_or("");

        // 游标识别
        if entry_id.contains("cursor-bottom") || entry_id.contains("bottom") {
            if let Some(val) = obj.pointer("/content/value")
                .or_else(|| obj.pointer("/content/cursor/value"))
                .or_else(|| obj.pointer("/content/itemContent/value"))
                .or_else(|| obj.get("value"))
                .and_then(|v| v.as_str())
            {
                *next_cursor = Some(val.to_string());
                return;
            }
        }

        if entry_id.contains("cursor-top") || entry_id.contains("top") || entry_id.contains("promoted") {
            return;
        }

        // 检查是否包含子 items 模块（如 profile-conversation, grid, TimelineTimelineModule）
        if let Some(sub_items) = obj.pointer("/content/items").and_then(|v| v.as_array()) {
            for sub in sub_items {
                self.extract_tweet_media_from_item(sub, items);
            }
            return;
        }

        // 单推文条目
        self.extract_tweet_media_from_item(obj, items);
    }

    fn extract_tweet_media_from_item(&self, obj: &Value, items: &mut Vec<MediaItem>) {
        // 从各种可能的 JSON 嵌套路径提取 tweet_results
        let tweet_result = obj.pointer("/content/itemContent/tweet_results/result")
            .or_else(|| obj.pointer("/item/itemContent/tweet_results/result"))
            .or_else(|| obj.pointer("/content/tweet_results/result"))
            .or_else(|| obj.pointer("/itemContent/tweet_results/result"))
            .or_else(|| obj.pointer("/item/items/0/item/itemContent/tweet_results/result"));

        let tweet_result = match tweet_result {
            Some(r) if !r.is_null() => r,
            _ => return,
        };

        // 解包 TweetWithVisibilityResults
        let tweet = if tweet_result.get("__typename").and_then(|v| v.as_str()) == Some("TweetWithVisibilityResults") {
            tweet_result.get("tweet").unwrap_or(tweet_result)
        } else if tweet_result.get("tweet").map_or(false, |v| v.is_object()) {
            &tweet_result["tweet"]
        } else {
            tweet_result
        };

        let legacy = if tweet.get("legacy").map_or(false, |v| v.is_object()) {
            &tweet["legacy"]
        } else {
            tweet
        };

        // 时间过滤
        let created_at = legacy.get("created_at").or_else(|| tweet.get("created_at"));
        let tweet_date = parse_tweet_date(created_at);

        if let Some((start, end)) = self.time_range {
            if let Some(date) = tweet_date {
                if date < start || date > end {
                    return;
                }
            }
        }

        let timestr = tweet_date
            .map(|d| d.format("%Y-%m-%d %H-%M").to_string())
            .unwrap_or_else(|| chrono::Local::now().format("%Y-%m-%d %H-%M").to_string());

        // 提取媒体（优先 extended_entities，再 entities）
        let media_arr = legacy.pointer("/extended_entities/media")
            .or_else(|| legacy.pointer("/entities/media"))
            .or_else(|| tweet.pointer("/extended_entities/media"))
            .or_else(|| tweet.pointer("/entities/media"))
            .and_then(|v| v.as_array());

        if let Some(media_list) = media_arr {
            for m in media_list {
                // 1. 视频 / GIF
                if let Some(variants) = m.pointer("/video_info/variants").and_then(|v| v.as_array()) {
                    let mut best_url: Option<String> = None;
                    let mut max_bitrate: i64 = -1;
                    for v in variants {
                        let bitrate = v.get("bitrate").and_then(|b| b.as_i64()).unwrap_or(-1);
                        if let Some(u) = v.get("url").and_then(|u| u.as_str()) {
                            if bitrate > max_bitrate {
                                max_bitrate = bitrate;
                                best_url = Some(u.to_string());
                            } else if best_url.is_none() && u.contains(".mp4") {
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
                        continue;
                    }
                }

                // 2. 图片
                if let Some(img_url) = m.get("media_url_https").or_else(|| m.get("media_url")).and_then(|v| v.as_str()) {
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

    /// 尝试动态从 x.com 前端资源嗅探最新的 GraphQL queryId
    async fn try_extract_dynamic_ops(&self) -> Option<HashMap<String, String>> {
        let home_html = match tokio::time::timeout(
            std::time::Duration::from_secs(6),
            self.client.get("https://x.com").send(),
        )
        .await
        {
            Ok(Ok(resp)) => resp.text().await.ok()?,
            _ => return None,
        };

        let mut script_urls = Vec::new();
        for line in home_html.split("src=\"") {
            if let Some(url_part) = line.split('"').next() {
                if (url_part.contains("main.") || url_part.contains("api.") || url_part.contains("vendor")) && url_part.ends_with(".js") {
                    script_urls.push(url_part.to_string());
                }
            }
        }

        let mut ops = HashMap::new();
        for js_url in script_urls.iter().take(3) {
            if let Ok(Ok(resp)) = tokio::time::timeout(std::time::Duration::from_secs(5), self.client.get(js_url).send()).await {
                if let Ok(js_text) = resp.text().await {
                    extract_ops_from_js(&js_text, &mut ops);
                    if ops.contains_key("UserByScreenName") && ops.contains_key("UserMedia") {
                        break;
                    }
                }
            }
        }

        if !ops.is_empty() {
            Some(ops)
        } else {
            None
        }
    }

    /// 媒体文件下载 Client（无鉴权头，防止 CDN 边缘节点 400 校验拦截）
    pub fn media_client(&self) -> &Client {
        &self.media_client
    }
}

fn extract_ops_from_js(js: &str, map: &mut HashMap<String, String>) {
    // 匹配类似 queryId:"2DC9TKrcUzwGC_QskSVl5w",operationName:"UserMedia"
    for part in js.split("operationName:\"") {
        if let Some((op_name, rest)) = part.split_once('"') {
            if let Some(pos) = rest.find("queryId:\"") {
                let qid_part = &rest[pos + 9..];
                if let Some(qid) = qid_part.split('"').next() {
                    if qid.len() >= 20 && qid.len() <= 24 {
                        map.insert(op_name.to_string(), qid.to_string());
                    }
                }
            }
        }
    }
}

fn get_default_features() -> Value {
    serde_json::json!({
        "responsive_web_graphql_exclude_directive_enabled": true,
        "verified_phone_label_enabled": false,
        "creator_subscriptions_tweet_preview_api_enabled": true,
        "responsive_web_graphql_timeline_navigation_enabled": true,
        "responsive_web_graphql_skip_user_profile_image_extensions_enabled": false,
        "tweetypie_unmention_optimization_enabled": true,
        "responsive_web_edit_tweet_api_enabled": true,
        "graphql_is_translatable_rweb_tweet_is_translatable_enabled": true,
        "view_counts_everywhere_api_enabled": true,
        "longform_notetweets_consumption_enabled": true,
        "responsive_web_twitter_article_tweet_consumption_enabled": true,
        "tweet_awards_web_tipping_enabled": false,
        "freedom_of_speech_not_reach_fetch_enabled": true,
        "standardized_nudges_misinfo": true,
        "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled": true,
        "longform_notetweets_rich_text_read_enabled": true,
        "longform_notetweets_inline_media_enabled": true,
        "responsive_web_media_download_video_enabled": false,
        "responsive_web_enhance_cards_enabled": false,
        "articles_preview_enabled": false,
        "c9s_tweet_anatomy_moderator_badge_enabled": true,
        "communities_web_enable_tweet_community_results_fetch": true,
        "creator_subscriptions_quote_tweet_preview_enabled": false,
        "rweb_video_timestamps_enabled": true
    })
}

fn resolve_proxy(custom_proxy: Option<&str>) -> Option<(reqwest::Proxy, String)> {
    if let Some(p) = custom_proxy {
        let p = p.trim();
        if !p.is_empty() {
            let proxy_url = if !p.contains("://") {
                format!("http://{p}")
            } else {
                p.to_string()
            };
            if let Ok(proxy_obj) = reqwest::Proxy::all(&proxy_url) {
                return Some((proxy_obj, format!("自定义代理 {proxy_url}")));
            }
        }
    }

    const ENV_KEYS: [&str; 6] = [
        "http_proxy", "HTTP_PROXY", "https_proxy", "HTTPS_PROXY", "all_proxy", "ALL_PROXY",
    ];
    let env_set = ENV_KEYS
        .iter()
        .any(|k| std::env::var(k).map(|v| !v.trim().is_empty()).unwrap_or(false));
    if env_set {
        return None;
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(server) = read_wininet_proxy_server() {
            let (scheme, addr) = parse_proxy_server(&server);
            let url = format!("{scheme}://{addr}");
            if let Ok(proxy_obj) = reqwest::Proxy::all(&url) {
                return Some((proxy_obj, format!("系统代理 {url}")));
            }
        }
    }

    None
}

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

async fn read_body(resp: reqwest::Response) -> Result<String, String> {
    let status = resp.status();
    let body = resp.text().await.map_err(|e| format!("读取响应失败: {e}"))?;
    if body.trim().is_empty() {
        return Err(format!(
            "X (Twitter) 返回空响应 (HTTP {status})，请检查网络或 Cookie 是否失效"
        ));
    }
    if !body.trim_start().starts_with('{') && !body.trim_start().starts_with('[') {
        return Err(format!(
            "X (Twitter) 返回了非 JSON 响应 (HTTP {status})，片段: {}",
            &body.chars().take(200).collect::<String>()
        ));
    }
    Ok(body)
}

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

fn extract_media_id(url: &str, ext: &str) -> String {
    let base = url.split('?').next().unwrap_or(url);

    if ext == "mp4" {
        if let Some(pos) = base.find("ext_tw_video/") {
            let rest = &base[pos + 13..];
            let id: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            if !id.is_empty() {
                return id;
            }
        }
        let digits: Vec<&str> = base.rsplit('/').collect();
        for part in digits {
            let num: String = part.chars().take_while(|c| c.is_ascii_digit()).collect();
            if num.len() >= 10 {
                return num;
            }
        }
    } else {
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

    let mut hasher = DefaultHasher::new();
    base.hash(&mut hasher);
    format!("{:x}", hasher.finish())[..12].to_string()
}
