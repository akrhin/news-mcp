//! News service implementation
//!
//! Handles fetching and parsing RSS feeds.

use crate::cache::{NewsArticle, NewsCategory};
use crate::config::AppConfig;
use crate::error::{Error, Result};
use crate::service::NewsSource;
use crate::utils::get_feed_urls;
use async_trait::async_trait;
use feed_rs::parser;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

/// News service for fetching RSS feeds
pub struct NewsService {
    /// HTTP client with retry middleware
    client: reqwest_middleware::ClientWithMiddleware,
    /// Optional config for feed URL lookups
    config: Option<Arc<AppConfig>>,
}

impl NewsService {
    /// Create a new news service
    pub fn new() -> Self {
        Self {
            client: crate::utils::build_http_client_with_retry(),
            config: None,
        }
    }

    /// Create a news service with configuration for dynamic feed URLs
    pub fn with_config(config: Arc<AppConfig>) -> Self {
        Self {
            client: crate::utils::build_http_client_with_retry(),
            config: Some(config),
        }
    }

    /// Get feed URLs for a category, using config if available
    fn get_feed_urls_for_category(&self, category: &NewsCategory) -> Vec<String> {
        if let Some(config) = &self.config {
            let urls = config.get_feed_urls(&category.to_string());
            if !urls.is_empty() {
                return urls;
            }
        }
        // Fallback to hardcoded defaults
        get_feed_urls(category)
            .into_iter()
            .map(|s| s.to_string())
            .collect()
    }

    /// Fetch RSS feed from URL and parse articles
    pub async fn fetch_rss_feed(
        &self,
        url: &str,
        category: NewsCategory,
    ) -> Result<Vec<NewsArticle>> {
        debug!("Fetching RSS feed from: {}", url);

        let response = self.client.get(url).send().await?;

        if !response.status().is_success() {
            return Err(Error::rss(format!(
                "Failed to fetch {}: status {}",
                url,
                response.status()
            )));
        }

        let content = response.bytes().await?;
        self.parse_feed(&content, category)
    }

    /// Parse RSS/Atom feed content into articles
    pub fn parse_feed(&self, content: &[u8], category: NewsCategory) -> Result<Vec<NewsArticle>> {
        let feed = parser::parse(content)
            .map_err(|e| Error::rss(format!("Failed to parse feed: {e}")))?;

        // Clone the feed title before the closure to avoid move issues
        let feed_title = feed.title.clone();

        let articles: Vec<NewsArticle> = feed
            .entries
            .into_iter()
            .filter_map(|entry| {
                let title = entry
                    .title
                    .map(|t| t.content)
                    .unwrap_or_else(|| "Untitled".to_string());

                let description = entry
                    .summary
                    .map(|s| s.content)
                    .or_else(|| entry.content.map(|c| c.body.unwrap_or_default()));

                let link = entry
                    .links
                    .first()
                    .map(|l| l.href.clone())
                    .unwrap_or_else(String::new);

                if link.is_empty() {
                    warn!("Article '{}' has no link, skipping", title);
                    return None;
                }

                let source = feed_title
                    .clone()
                    .map(|t| t.content)
                    .unwrap_or_else(|| "Unknown Source".to_string());

                let published_at = entry.published.or(entry.updated);

                let author = entry.authors.first().map(|a| a.name.clone());

                Some(NewsArticle::new(
                    title,
                    description,
                    link,
                    source,
                    category.clone(),
                    published_at,
                    author,
                ))
            })
            .collect();

        info!("Parsed {} articles from feed", articles.len());
        Ok(articles)
    }

    /// Fetch all feeds for a category concurrently
    pub async fn fetch_category(&self, category: NewsCategory) -> Result<Vec<NewsArticle>> {
        let urls = self.get_feed_urls_for_category(&category);

        if urls.is_empty() {
            debug!("No feed URLs configured for category {}", category);
            return Ok(vec![]);
        }

        // Fetch all URLs concurrently
        let cat = category.clone();
        let futures: Vec<_> = urls
            .iter()
            .map(|url| self.fetch_rss_feed(url, cat.clone()))
            .collect();

        let results = futures::future::join_all(futures).await;

        let mut all_articles = Vec::new();
        for (i, result) in results.into_iter().enumerate() {
            match result {
                Ok(articles) => {
                    all_articles.extend(articles);
                }
                Err(e) => {
                    error!("Failed to fetch feed {}: {}", urls[i], e);
                }
            }
        }

        // Sort by publication date (most recent first)
        all_articles.sort_by(|a, b| match (a.published_at, b.published_at) {
            (Some(a_date), Some(b_date)) => b_date.cmp(&a_date),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        });

        info!(
            "Fetched {} total articles for category {}",
            all_articles.len(),
            category
        );
        Ok(all_articles)
    }

    /// Fetch all categories (builtins + custom from config) concurrently
    pub async fn fetch_all_categories(
        &self,
    ) -> Result<std::collections::HashMap<NewsCategory, Vec<NewsArticle>>> {
        let mut categories = NewsCategory::builtin();

        // Add custom categories from config
        if let Some(config) = &self.config {
            for key in config.feeds.keys() {
                let cat = NewsCategory::from_config_key(key);
                if matches!(cat, NewsCategory::Custom(_)) && !categories.contains(&cat) {
                    categories.push(cat);
                }
            }
        }
        let mut results = std::collections::HashMap::new();

        // Use futures to fetch concurrently
        let futures: Vec<_> = categories
            .iter()
            .map(|category| {
                let cat = category.clone();
                let cat2 = cat.clone();
                async move {
                    let articles = self.fetch_category(cat).await?;
                    Ok::<_, Error>((cat2, articles))
                }
            })
            .collect();

        // Execute all futures concurrently — each category independently
        // (join_all, not try_join_all: a single bad feed must not kill all others)
        let results_vec = futures::future::join_all(futures).await;

        for result in results_vec {
            match result {
                Ok((category, articles)) => {
                    results.insert(category, articles);
                }
                Err(e) => {
                    error!("Failed to fetch category: {}", e);
                }
            }
        }

        Ok(results)
    }
}

impl Default for NewsService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NewsSource for NewsService {
    fn name(&self) -> &str {
        "RSS Feeds"
    }

    async fn fetch(&self) -> Result<HashMap<NewsCategory, Vec<NewsArticle>>> {
        self.fetch_all_categories().await
    }
}
