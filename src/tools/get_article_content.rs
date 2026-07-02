//! get_article_content tool implementation
//!
//! Fetches and caches full article content by article ID.
//! AI uses this after browsing news headlines to deep-read selected articles.

use crate::cache::{ArticleCache, CachedArticle, NewsCache};
use crate::config::ArticleFetchConfig;
use crate::service::ArticleFetcher;
use crate::tools::Tool;
use async_trait::async_trait;
use rust_mcp_sdk::macros;
use rust_mcp_sdk::schema::{CallToolError, CallToolResult, Tool as McpTool};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Hot search sources that don't support content fetching
const HOT_SEARCH_SOURCES: &[&str] = &[
    "微博热搜",
    "百度热搜",
    "知乎热榜",
    "抖音热点",
    "B站热搜",
    "贴吧热议",
    "今日头条",
    "华尔街见闻",
    "财联社热门",
    "澎湃新闻",
    "凤凰网",
];

/// Get article content tool parameters
#[macros::mcp_tool(
    name = "get_article_content",
    title = "Get Article Content",
    description = "Fetches full article content by article ID. Use this after browsing news headlines with get_news to deep-read selected articles. NOTE: Hot search / trending topics do not support content fetching — they are social platform trends, not full articles. Only works for RSS-based news sources (Technology, Science, China News, etc.). First fetch performs HTTP request; subsequent calls return cached content instantly.",
    destructive_hint = false,
    idempotent_hint = true,
    open_world_hint = true,
    read_only_hint = true
)]
#[derive(Debug, Clone, Deserialize, Serialize, macros::JsonSchema)]
pub struct GetArticleContentTool {
    /// Article ID (shown in get_news output)
    #[json_schema(
        title = "Article ID",
        description = "The article ID shown in get_news output (e.g. 'breaking', 'techcrunch')"
    )]
    pub id: String,

    /// Output format
    #[json_schema(
        title = "Format",
        description = "Output format (markdown, json, text)",
        default = "markdown",
        enum_values = ["markdown", "json", "text"]
    )]
    pub format: Option<String>,
}

/// Get article content tool implementation
pub struct GetArticleContentToolImpl {
    news_cache: Arc<NewsCache>,
    article_cache: Arc<ArticleCache>,
    fetch_config: ArticleFetchConfig,
}

impl GetArticleContentToolImpl {
    /// Create a new get_article_content tool
    pub fn new(
        news_cache: Arc<NewsCache>,
        article_cache: Arc<ArticleCache>,
        fetch_config: ArticleFetchConfig,
    ) -> Self {
        Self {
            news_cache,
            article_cache,
            fetch_config,
        }
    }

    /// Format article with content as markdown
    fn format_as_markdown(
        article: &crate::cache::NewsArticle,
        cached_article: &CachedArticle,
        cached: bool,
    ) -> String {
        let mut output = String::new();

        output.push_str(&format!("## {}\n\n", article.title));
        output.push_str(&format!("{}\n\n", cached_article.content));
        output.push_str("---\n\n");
        output.push_str(&format!("- **Source**: {}\n", article.source));
        output.push_str(&format!("- **ID**: {}\n", article.id));
        output.push_str(&format!(
            "- **Fetched**: {}\n",
            cached_article.fetched_at.format("%Y-%m-%d %H:%M UTC")
        ));
        output.push_str(&format!(
            "- **Word Count**: {}\n",
            cached_article.word_count
        ));
        output.push_str(&format!("- **Cached**: {}\n", cached));

        output
    }

    /// Format article with content as JSON
    fn format_as_json(
        article: &crate::cache::NewsArticle,
        cached_article: &CachedArticle,
        cached: bool,
    ) -> String {
        let output = serde_json::json!({
            "id": article.id,
            "title": article.title,
            "content": cached_article.content,
            "source": article.source,
            "url": article.link,
            "fetched_at": cached_article.fetched_at.to_rfc3339(),
            "word_count": cached_article.word_count,
            "cached": cached
        });

        serde_json::to_string_pretty(&output).unwrap_or_else(|_| "{}".to_string())
    }

    /// Format article with content as plain text
    fn format_as_text(
        article: &crate::cache::NewsArticle,
        cached_article: &CachedArticle,
        cached: bool,
    ) -> String {
        let mut output = String::new();

        output.push_str(&format!("Title: {}\n\n", article.title));
        output.push_str(&format!("{}\n\n", cached_article.content));
        output.push_str(&format!("Source: {}\n", article.source));
        output.push_str(&format!("ID: {}\n", article.id));
        output.push_str(&format!(
            "Fetched: {}\n",
            cached_article.fetched_at.format("%Y-%m-%d %H:%M UTC")
        ));
        output.push_str(&format!("Word Count: {}\n", cached_article.word_count));
        output.push_str(&format!("Cached: {}\n", cached));

        output
    }
}

#[async_trait]
impl Tool for GetArticleContentToolImpl {
    fn definition(&self) -> McpTool {
        GetArticleContentTool::tool()
    }

    async fn execute(
        &self,
        arguments: serde_json::Value,
    ) -> std::result::Result<CallToolResult, CallToolError> {
        // Parse parameters
        let params: GetArticleContentTool = serde_json::from_value(arguments).map_err(|e| {
            CallToolError::invalid_arguments(
                "get_article_content",
                Some(format!("Invalid parameters: {}", e)),
            )
        })?;

        let id = params.id.trim();
        if id.is_empty() {
            return Err(CallToolError::from_message(
                "Article ID parameter is required",
            ));
        }

        let format = params.format.unwrap_or_else(|| "markdown".to_string());

        // Resolve max chars from config
        let max_chars = self.fetch_config.max_chars;

        // Look up article by ID in news cache
        let article = self
            .news_cache
            .get_article_by_id(id)
            .map_err(|e| CallToolError::from_message(format!("Cache error: {}", e)))?;

        let article = match article {
            Some(a) => a,
            None => {
                return Err(CallToolError::from_message(format!(
                    "Article with ID '{}' not found. Use get_news to browse available articles.",
                    id
                )))
            }
        };

        // Check if article is from a hot search platform (these don't support content fetching)
        if HOT_SEARCH_SOURCES.contains(&article.source.as_str()) {
            return Err(CallToolError::from_message(format!(
                "Hot search articles from '{}' do not support full content fetching. These are trending topics from social platforms, not traditional news articles.",
                article.source
            )));
        }

        // Check article content cache first
        if let Ok(Some(cached_article)) = self.article_cache.get(&article.link) {
            debug!("Returning cached content for article ID: {}", id);

            // Truncate content if needed
            let mut truncated_article = cached_article.clone();
            if truncated_article.content.len() > max_chars {
                truncated_article.content = format!(
                    "{}…\n\n_[truncated — {} chars; full article: {} chars total]_",
                    &truncated_article.content[..max_chars],
                    max_chars,
                    cached_article.content.len(),
                );
            }

            let output = match format.as_str() {
                "json" => Self::format_as_json(&article, &truncated_article, true),
                "text" => Self::format_as_text(&article, &truncated_article, true),
                _ => Self::format_as_markdown(&article, &truncated_article, true),
            };

            return Ok(CallToolResult::text_content(vec![output.into()]));
        }

        // Fetch content from URL using ArticleFetcher
        let fetcher = ArticleFetcher::new(self.fetch_config.fetch_timeout_secs, 1);
        let content = fetcher.fetch_content(&article.link).await.map_err(|e| {
            CallToolError::from_message(format!("Failed to fetch article content: {}", e))
        })?;

        let content = match content {
            Some(c) => c,
            None => {
                return Err(CallToolError::from_message(format!(
                    "Failed to extract content from article '{}'",
                    article.title
                )))
            }
        };

        // Store in article cache
        let cached_article = CachedArticle::new(content);
        if let Err(e) = self
            .article_cache
            .insert(article.link.clone(), cached_article.clone())
        {
            warn!("Failed to cache article content: {}", e);
        }

        // Also update the news cache article with content
        if let Err(e) = self
            .news_cache
            .update_article_content(&article.link, cached_article.content.clone())
        {
            warn!("Failed to update article content in news cache: {}", e);
        }

        info!(
            "Successfully fetched article '{}' ({} words, ID: {})",
            article.title, cached_article.word_count, id
        );

        // Format output
        let mut display_article = cached_article.clone();
        if display_article.content.len() > max_chars {
            display_article.content = format!(
                "{}…\n\n_[truncated — {} chars; full article: {} chars total]_",
                &display_article.content[..max_chars],
                max_chars,
                cached_article.content.len(),
            );
        }

        let output = match format.as_str() {
            "json" => Self::format_as_json(&article, &display_article, false),
            "text" => Self::format_as_text(&article, &display_article, false),
            _ => Self::format_as_markdown(&article, &display_article, false),
        };

        Ok(CallToolResult::text_content(vec![output.into()]))
    }
}
