//! NewsNow hot list service implementation
//!
//! Fetches hot trending topics from NewsNow API (https://newsnow.busiyi.world/api/s)
//! Supports multiple platforms: weibo, baidu, zhihu, douyin, bilibili-hot-search, etc.

use crate::cache::{NewsArticle, NewsCategory};
use crate::error::{Error, Result};
use crate::service::NewsSource;
use async_trait::async_trait;
use serde::Deserialize;
use std::collections::HashMap;
use tracing::{debug, info, warn};

const NEWSNOW_API_BASE: &str = "https://newsnow.busiyi.world/api/s";

/// NewsNow API response structure
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct NewsNowResponse {
    status: String,
    id: String,
    #[serde(default)]
    updated_time: Option<i64>,
    items: Vec<NewsNowItem>,
}

/// Single hot item from NewsNow
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct NewsNowItem {
    #[serde(default, deserialize_with = "deserialize_id")]
    id: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    url: String,
    #[serde(default)]
    mobile_url: String,
    #[serde(default)]
    extra: Option<NewsNowExtra>,
}

/// Custom deserializer for id field (handles both string and number)
fn deserialize_id<'de, D>(deserializer: D) -> std::result::Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Debug, Deserialize)]
    #[serde(untagged)]
    enum IdValue {
        String(String),
        Number(i64),
    }

    match IdValue::deserialize(deserializer) {
        Ok(IdValue::String(s)) => Ok(s),
        Ok(IdValue::Number(n)) => Ok(n.to_string()),
        Err(_) => Ok(String::new()), // Default to empty string on error
    }
}

/// Icon value - can be a string URL or an object with url field
#[derive(Debug, Deserialize)]
#[serde(untagged)]
#[allow(dead_code)]
enum IconValue {
    String(String),
    Object(NewsNowIcon),
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct NewsNowIcon {
    #[serde(default)]
    url: Option<String>,
}

/// Extra metadata for hot item
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct NewsNowExtra {
    #[serde(default)]
    hover: Option<String>,
    #[serde(default)]
    icon: Option<IconValue>,
}

/// Platform configuration for NewsNow
#[derive(Debug, Clone)]
pub struct NewsNowPlatform {
    pub id: &'static str,
    pub name: &'static str,
    pub category: NewsCategory,
}

/// Available NewsNow platforms
pub const NEWSNOW_PLATFORMS: &[NewsNowPlatform] = &[
    NewsNowPlatform {
        id: "weibo",
        name: "微博热搜",
        category: NewsCategory::WeiboHot,
    },
    NewsNowPlatform {
        id: "baidu",
        name: "百度热搜",
        category: NewsCategory::BaiduHot,
    },
    NewsNowPlatform {
        id: "zhihu",
        name: "知乎热榜",
        category: NewsCategory::ZhihuHot,
    },
    NewsNowPlatform {
        id: "douyin",
        name: "抖音热点",
        category: NewsCategory::DouyinHot,
    },
    NewsNowPlatform {
        id: "bilibili-hot-search",
        name: "B站热搜",
        category: NewsCategory::BilibiliHot,
    },
    NewsNowPlatform {
        id: "tieba",
        name: "贴吧热议",
        category: NewsCategory::TiebaHot,
    },
    NewsNowPlatform {
        id: "toutiao",
        name: "今日头条",
        category: NewsCategory::ToutiaoHot,
    },
    NewsNowPlatform {
        id: "wallstreetcn-hot",
        name: "华尔街见闻",
        category: NewsCategory::WallstreetcnHot,
    },
    NewsNowPlatform {
        id: "cls-hot",
        name: "财联社热门",
        category: NewsCategory::ClsHot,
    },
    NewsNowPlatform {
        id: "thepaper",
        name: "澎湃新闻",
        category: NewsCategory::ThepaperHot,
    },
    NewsNowPlatform {
        id: "ifeng",
        name: "凤凰网",
        category: NewsCategory::IfengHot,
    },
];

/// NewsNow service for fetching hot trending topics
pub struct NewsNowService {
    client: reqwest::Client,
    /// Platforms to fetch (default: all)
    platforms: Vec<NewsNowPlatform>,
}

impl NewsNowService {
    /// Create a new NewsNow service with all platforms
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .connect_timeout(std::time::Duration::from_secs(5))
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
                .build()
                .expect("Failed to create HTTP client"),
            platforms: NEWSNOW_PLATFORMS.to_vec(),
        }
    }

    /// Create service with specific platforms
    pub fn with_platforms(platforms: Vec<NewsNowPlatform>) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .connect_timeout(std::time::Duration::from_secs(5))
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
                .build()
                .expect("Failed to create HTTP client"),
            platforms,
        }
    }

    /// Fetch hot list for a single platform
    pub async fn fetch_platform(&self, platform: &NewsNowPlatform) -> Result<Vec<NewsArticle>> {
        let url = format!("{}?id={}&latest", NEWSNOW_API_BASE, platform.id);
        debug!("Fetching {} hot list from {}", platform.name, url);

        let response = self
            .client
            .get(&url)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| Error::rss(format!("Failed to fetch {}: {}", platform.name, e)))?;

        if !response.status().is_success() {
            return Err(Error::rss(format!(
                "{} returned status {}",
                platform.name,
                response.status()
            )));
        }

        let data: NewsNowResponse = response
            .json()
            .await
            .map_err(|e| Error::rss(format!("Failed to parse {}: {}", platform.name, e)))?;

        if data.status != "success" && data.status != "cache" {
            warn!(
                "{} returned non-success status: {}",
                platform.name, data.status
            );
            return Ok(vec![]);
        }

        let articles: Vec<NewsArticle> = data
            .items
            .into_iter()
            .filter_map(|item| self.item_to_article(&item, platform))
            .collect();

        info!("Fetched {} articles from {}", articles.len(), platform.name);
        Ok(articles)
    }

    /// Convert NewsNow item to NewsArticle
    fn item_to_article(
        &self,
        item: &NewsNowItem,
        platform: &NewsNowPlatform,
    ) -> Option<NewsArticle> {
        if item.title.is_empty() {
            return None;
        }

        let title = clean_text(&item.title);

        let description = item
            .extra
            .as_ref()
            .and_then(|e| e.hover.as_ref().map(|h| clean_text(h)));

        let link = if !item.url.is_empty() {
            item.url.clone()
        } else if !item.mobile_url.is_empty() {
            item.mobile_url.clone()
        } else {
            // Fallback: construct a search URL if no link provided
            format!(
                "https://www.google.com/search?q={}",
                urlencoding::encode(&title)
            )
        };

        Some(NewsArticle::new(
            title,
            description,
            link,
            platform.name.to_string(),
            platform.category.clone(),
            None, // NewsNow doesn't provide individual timestamps
            None,
        ))
    }

    /// Fetch all platforms concurrently
    pub async fn fetch_all_platforms(&self) -> Result<HashMap<NewsCategory, Vec<NewsArticle>>> {
        let mut results = HashMap::new();

        // Fetch all platforms concurrently
        let futures: Vec<_> = self
            .platforms
            .iter()
            .map(|platform| {
                let platform = platform.clone();
                async move {
                    let articles = self.fetch_platform(&platform).await?;
                    Ok::<_, Error>((platform.category, articles))
                }
            })
            .collect();

        let results_vec = futures::future::join_all(futures).await;

        for result in results_vec {
            match result {
                Ok((category, articles)) => {
                    if !articles.is_empty() {
                        results.insert(category, articles);
                    }
                }
                Err(e) => {
                    warn!("Failed to fetch platform: {}", e);
                }
            }
        }

        info!(
            "Fetched {} platforms with {} total articles",
            results.len(),
            results.values().map(|v| v.len()).sum::<usize>()
        );
        Ok(results)
    }
}

#[async_trait]
impl NewsSource for NewsNowService {
    fn name(&self) -> &str {
        "NewsNow Hot Lists"
    }

    async fn fetch(&self) -> Result<HashMap<NewsCategory, Vec<NewsArticle>>> {
        self.fetch_all_platforms().await
    }
}

impl Default for NewsNowService {
    fn default() -> Self {
        Self::new()
    }
}

/// Clean text by removing HTML entities and unwanted content
fn clean_text(text: &str) -> String {
    let mut cleaned = text.to_string();

    // Remove common HTML entities
    cleaned = cleaned.replace("&lt;", "<");
    cleaned = cleaned.replace("&gt;", ">");
    cleaned = cleaned.replace("&amp;", "&");
    cleaned = cleaned.replace("&quot;", "\"");
    cleaned = cleaned.replace("&apos;", "'");
    cleaned = cleaned.replace("&nbsp;", " ");
    cleaned = cleaned.replace("&mdash;", "—");
    cleaned = cleaned.replace("&ndash;", "-");
    cleaned = cleaned.replace("&#x27;", "'");
    cleaned = cleaned.replace("&#x2F;", "/");

    // Remove HTML tags (basic)
    let mut result = String::new();
    let mut in_tag = false;
    for c in cleaned.chars() {
        if c == '<' {
            in_tag = true;
        } else if c == '>' {
            in_tag = false;
        } else if !in_tag {
            result.push(c);
        }
    }

    // Trim and normalize whitespace
    result = result
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ");

    // Remove multiple consecutive spaces
    while result.contains("  ") {
        result = result.replace("  ", " ");
    }

    result.trim().to_string()
}

/// URL encoding helper (simple implementation)
mod urlencoding {
    pub fn encode(s: &str) -> String {
        s.chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '~' {
                    c.to_string()
                } else {
                    format!("%{:02X}", c as u32)
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_text_basic() {
        let input = "Hello &amp; World";
        let output = clean_text(input);
        assert_eq!(output, "Hello & World");
    }

    #[test]
    fn test_clean_text_html_tags() {
        let input = "<p>Hello World</p>";
        let output = clean_text(input);
        assert_eq!(output, "Hello World");
    }

    #[test]
    fn test_urlencoding() {
        let input = "测试标题";
        let output = urlencoding::encode(input);
        assert!(output.contains("%"));
    }

    #[test]
    fn test_platforms_available() {
        assert!(!NEWSNOW_PLATFORMS.is_empty());
        assert_eq!(NEWSNOW_PLATFORMS.len(), 11);
    }
}
