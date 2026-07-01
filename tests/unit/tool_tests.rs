//! Tool unit tests

use news_mcp::cache::{ArticleCache, NewsArticle, NewsCache, NewsCategory};
use news_mcp::config::{ArticleFetchConfig, FeedSourceConfig};
use news_mcp::tools::{create_default_registry, GetCategoriesToolImpl, GetNewsToolImpl, Tool};
use rust_mcp_sdk::schema::ContentBlock;
use std::collections::HashMap;
use std::sync::Arc;

fn get_text_content(result: &rust_mcp_sdk::schema::CallToolResult) -> &str {
    match &result.content[0] {
        ContentBlock::TextContent(text) => &text.text,
        _ => panic!("Expected text content"),
    }
}

fn create_test_cache() -> Arc<NewsCache> {
    let cache = Arc::new(NewsCache::new(100));

    let articles = [
        NewsArticle::new(
            "Technology News".to_string(),
            Some("Latest tech updates".to_string()),
            "https://example.com/tech".to_string(),
            "Tech Source".to_string(),
            NewsCategory::Technology,
            None,
            None,
        ),
        NewsArticle::new(
            "Science News".to_string(),
            Some("Science updates".to_string()),
            "https://example.com/science".to_string(),
            "Science Source".to_string(),
            NewsCategory::Science,
            None,
            None,
        ),
    ];

    cache
        .set_category_news(NewsCategory::Technology, vec![articles[0].clone()])
        .unwrap();
    cache
        .set_category_news(NewsCategory::Science, vec![articles[1].clone()])
        .unwrap();

    cache
}

fn create_empty_cache() -> Arc<NewsCache> {
    Arc::new(NewsCache::new(100))
}

fn create_test_feeds() -> HashMap<String, FeedSourceConfig> {
    HashMap::new()
}

fn create_test_article_cache() -> Arc<ArticleCache> {
    Arc::new(ArticleCache::new(100))
}

fn create_article_fetch_config() -> ArticleFetchConfig {
    ArticleFetchConfig::default()
}

// ============================================================================
// Tool Registry Tests
// ============================================================================

#[test]
fn test_tool_registry() {
    let cache = create_test_cache();
    let article_cache = create_test_article_cache();
    let article_fetch_config = create_article_fetch_config();
    let feeds = create_test_feeds();
    let registry = create_default_registry(cache, article_cache, article_fetch_config, feeds);

    let tools = registry.get_tools();
    assert_eq!(tools.len(), 3);

    // Verify tool names
    let tool_names: Vec<String> = tools.iter().map(|t| t.name.clone()).collect();
    assert!(tool_names.contains(&"get_news".to_string()));
    assert!(tool_names.contains(&"get_categories".to_string()));
    assert!(tool_names.contains(&"get_article_content".to_string()));
}

#[test]
fn test_tool_registry_get() {
    let cache = create_test_cache();
    let article_cache = create_test_article_cache();
    let article_fetch_config = create_article_fetch_config();
    let feeds = create_test_feeds();
    let registry = create_default_registry(cache, article_cache, article_fetch_config, feeds);

    let tool = registry.get("get_news");
    assert!(tool.is_some());

    let tool = registry.get("invalid_tool");
    assert!(tool.is_none());
}

// ============================================================================
// get_news Tool Tests
// ============================================================================

#[test]
fn test_get_news_tool_definition() {
    let cache = create_test_cache();
    let feeds = create_test_feeds();
    let tool = GetNewsToolImpl::new(cache, feeds);

    let definition = tool.definition();
    assert_eq!(definition.name, "get_news");
    // description is optional in the schema
    if let Some(desc) = &definition.description {
        assert!(desc.contains("cache"));
    }
}

#[tokio::test]
async fn test_get_news_tool_execution() {
    let cache = create_test_cache();
    let feeds = create_test_feeds();
    let tool = GetNewsToolImpl::new(cache, feeds);

    let result = tool.execute(serde_json::json!({})).await.unwrap();
    assert!(get_text_content(&result).contains("Technology"));
}

#[tokio::test]
async fn test_get_news_tool_with_params() {
    let cache = create_test_cache();
    let feeds = create_test_feeds();
    let tool = GetNewsToolImpl::new(cache, feeds);

    let params = serde_json::json!({
        "category": "science",
        "limit": 1,
        "format": "text"
    });

    let result = tool.execute(params).await.unwrap();
    assert!(get_text_content(&result).contains("Science"));
}

#[tokio::test]
async fn test_get_news_all_categories() {
    let cache = create_test_cache();
    let feeds = create_test_feeds();
    let tool = GetNewsToolImpl::new(cache, feeds);

    // Test each category
    for category in &[
        "technology",
        "science",
        "hackernews",
        "instant",
        "headlines",
        "politics",
    ] {
        let params = serde_json::json!({
            "category": category
        });
        let result = tool.execute(params).await.unwrap();
        // Should succeed even for empty categories
        let text = get_text_content(&result);
        assert!(!text.is_empty());
    }
}

#[tokio::test]
async fn test_get_news_invalid_category() {
    let cache = create_test_cache();
    let feeds = create_test_feeds();
    let tool = GetNewsToolImpl::new(cache, feeds);

    // "invalid_category" is now accepted as Custom category (never fails)
    let params = serde_json::json!({
        "category": "invalid_category"
    });

    let result = tool.execute(params).await.unwrap();
    assert!(!get_text_content(&result).is_empty());
}

#[tokio::test]
async fn test_get_news_limit_boundaries() {
    let cache = create_test_cache();
    let feeds = create_test_feeds();
    let tool = GetNewsToolImpl::new(cache, feeds);

    // Test minimum limit
    let params = serde_json::json!({
        "limit": 1
    });
    let result = tool.execute(params).await.unwrap();
    let text = get_text_content(&result);
    assert!(text.contains("Technology"));

    // Test maximum limit (should clamp to 50)
    let params = serde_json::json!({
        "limit": 100
    });
    let result = tool.execute(params).await.unwrap();
    assert!(!get_text_content(&result).is_empty());

    // Test limit of 0 (should clamp to 1)
    let params = serde_json::json!({
        "limit": 0
    });
    let result = tool.execute(params).await.unwrap();
    assert!(!get_text_content(&result).is_empty());
}

#[tokio::test]
async fn test_get_news_formats() {
    let cache = create_test_cache();
    let feeds = create_test_feeds();
    let tool = GetNewsToolImpl::new(cache, feeds);

    // Markdown format
    let params = serde_json::json!({
        "format": "markdown"
    });
    let result = tool.execute(params).await.unwrap();
    let text = get_text_content(&result);
    assert!(text.contains("# News Articles"));

    // JSON format
    let params = serde_json::json!({
        "format": "json"
    });
    let result = tool.execute(params).await.unwrap();
    let text = get_text_content(&result);
    assert!(text.starts_with('['));

    // Text format
    let params = serde_json::json!({
        "format": "text"
    });
    let result = tool.execute(params).await.unwrap();
    let text = get_text_content(&result);
    assert!(text.contains("1."));
}

#[tokio::test]
async fn test_get_news_invalid_format() {
    let cache = create_test_cache();
    let feeds = create_test_feeds();
    let tool = GetNewsToolImpl::new(cache, feeds);

    let params = serde_json::json!({
        "format": "invalid_format"
    });

    let result = tool.execute(params).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_get_news_empty_cache() {
    let cache = create_empty_cache();
    let feeds = create_test_feeds();
    let tool = GetNewsToolImpl::new(cache, feeds);

    let result = tool.execute(serde_json::json!({})).await.unwrap();
    let text = get_text_content(&result);
    assert!(text.contains("No articles found"));
}

// ============================================================================
// get_categories Tool Tests
// ============================================================================

#[test]
fn test_get_categories_tool_definition() {
    let cache = create_test_cache();
    let tool = GetCategoriesToolImpl::new(cache, HashMap::new());

    let definition = tool.definition();
    assert_eq!(definition.name, "get_categories");
}

#[tokio::test]
async fn test_get_categories_tool() {
    let cache = create_test_cache();
    let tool = GetCategoriesToolImpl::new(cache, HashMap::new());

    let result = tool.execute(serde_json::json!({})).await.unwrap();
    let text = get_text_content(&result);
    assert!(text.contains("Technology"));
    assert!(text.contains("Science"));
    assert!(text.contains("article"));
}

#[tokio::test]
async fn test_get_categories_all_present() {
    let cache = create_test_cache();
    let tool = GetCategoriesToolImpl::new(cache, HashMap::new());

    let result = tool.execute(serde_json::json!({})).await.unwrap();
    let text = get_text_content(&result);

    // All categories should be present
    for category in &[
        "Technology",
        "Science",
        "Hacker News",
        "即时新闻",
        "要闻导读",
    ] {
        assert!(text.contains(category));
    }
}

#[tokio::test]
async fn test_get_categories_empty_cache() {
    let cache = create_empty_cache();
    let tool = GetCategoriesToolImpl::new(cache, HashMap::new());

    let result = tool.execute(serde_json::json!({})).await.unwrap();
    let text = get_text_content(&result);

    // Should still show all categories with 0 articles
    assert!(text.contains("0 articles"));
}

// ============================================================================
// Integration Tests
// ============================================================================

#[tokio::test]
async fn test_multiple_tools_workflow() {
    let cache = create_test_cache();

    // Get categories
    let categories_tool = GetCategoriesToolImpl::new(cache.clone(), HashMap::new());
    let result = categories_tool
        .execute(serde_json::json!({}))
        .await
        .unwrap();
    assert!(get_text_content(&result).contains("Technology"));

    // Get news
    let get_news_tool = GetNewsToolImpl::new(cache.clone(), create_test_feeds());
    let result = get_news_tool
        .execute(serde_json::json!({"category": "technology"}))
        .await
        .unwrap();
    assert!(get_text_content(&result).contains("Technology"));
}
