//! Integration tests

use news_mcp::cache::{ArticleCache, NewsArticle, NewsCache, NewsCategory};
use news_mcp::config::{AppConfig, ArticleFetchConfig, FeedSourceConfig};
use news_mcp::service::NewsService;
use news_mcp::tools::create_default_registry;
use rust_mcp_sdk::schema::ContentBlock;
use std::collections::HashMap;
use std::sync::Arc;

fn get_text_content(result: &rust_mcp_sdk::schema::CallToolResult) -> &str {
    match &result.content[0] {
        ContentBlock::TextContent(text) => &text.text,
        _ => panic!("Expected text content"),
    }
}

#[test]
fn test_full_workflow() {
    // Create cache
    let cache = Arc::new(NewsCache::new(100));

    // Create and parse RSS feed
    let sample_feed = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
    <channel>
        <title>Test News</title>
        <item>
            <title>Breaking News</title>
            <link>https://example.com/breaking</link>
            <description>Important news story</description>
        </item>
    </channel>
</rss>"#;

    let service = NewsService::new();
    let articles = service
        .parse_feed(sample_feed, NewsCategory::Technology)
        .unwrap();

    // Store in cache
    cache
        .set_category_news(NewsCategory::Technology, articles)
        .unwrap();

    // Retrieve from cache
    let cached = cache.get_category_news(&NewsCategory::Technology).unwrap();
    assert_eq!(cached.len(), 1);
    assert_eq!(cached[0].title, "Breaking News");
}

#[tokio::test]
async fn test_tools_workflow() {
    let cache = Arc::new(NewsCache::new(100));

    // Add test article
    let article = NewsArticle::new(
        "Breaking News".to_string(),
        Some("Important story".to_string()),
        "https://example.com/breaking".to_string(),
        "Source".to_string(),
        NewsCategory::Technology,
        None,
        None,
    );
    cache
        .set_category_news(NewsCategory::Technology, vec![article])
        .unwrap();

    // Use tools
    let article_cache = Arc::new(ArticleCache::new(100));
    let article_fetch_config = ArticleFetchConfig::default();
    let feeds: HashMap<String, FeedSourceConfig> = HashMap::new();
    let registry = create_default_registry(cache, article_cache, article_fetch_config, feeds);
    let result = registry
        .get("get_news")
        .unwrap()
        .execute(serde_json::json!({"category": "technology"}))
        .await
        .unwrap();
    let text = get_text_content(&result);
    assert!(text.contains("Breaking News"));
}

#[test]
fn test_config_to_cache() {
    let config = AppConfig::default();

    // Create cache with config settings
    let cache = Arc::new(NewsCache::new(config.cache.max_articles_per_category));

    // Fill cache with articles exceeding limit
    let articles: Vec<NewsArticle> = (0..150)
        .map(|i| {
            NewsArticle::new(
                format!("Article {}", i),
                None,
                format!("https://example.com/{}", i),
                "Source".to_string(),
                NewsCategory::Technology,
                None,
                None,
            )
        })
        .collect();

    cache
        .set_category_news(NewsCategory::Technology, articles)
        .unwrap();

    // Should respect limit
    let cached = cache.get_category_news(&NewsCategory::Technology).unwrap();
    assert_eq!(cached.len(), config.cache.max_articles_per_category);
}

#[test]
fn test_multiple_categories_workflow() {
    let cache = Arc::new(NewsCache::new(100));

    // Add articles to multiple categories
    for category in NewsCategory::builtin() {
        let article = NewsArticle::new(
            format!("{} News", category.display_name()),
            None,
            "https://example.com".to_string(),
            "Source".to_string(),
            category.clone(),
            None,
            None,
        );

        cache.set_category_news(category, vec![article]).unwrap();
    }

    // Verify all categories have articles
    let categories = cache.get_all_categories().unwrap();
    for (category, count) in categories {
        assert_eq!(count, 1);

        let articles = cache.get_category_news(&category).unwrap();
        assert_eq!(articles.len(), 1);
        assert!(articles[0].title.contains(category.display_name().as_ref()));
    }
}

#[test]
fn test_search_across_categories() {
    let cache = Arc::new(NewsCache::new(100));

    // Add articles with common keyword
    let tech_article = NewsArticle::new(
        "Rust Programming Language".to_string(),
        Some("Learn Rust".to_string()),
        "https://rust-lang.org".to_string(),
        "Tech Source".to_string(),
        NewsCategory::Technology,
        None,
        None,
    );

    let science_article = NewsArticle::new(
        "Scientific Computing with Rust".to_string(),
        Some("Rust for science".to_string()),
        "https://science.org".to_string(),
        "Science Source".to_string(),
        NewsCategory::Science,
        None,
        None,
    );

    cache
        .set_category_news(NewsCategory::Technology, vec![tech_article])
        .unwrap();
    cache
        .set_category_news(NewsCategory::Science, vec![science_article])
        .unwrap();

    // Search across all categories
    let results = cache.search("Rust", None).unwrap();
    assert_eq!(results.len(), 2);

    // Search in specific category
    let tech_results = cache
        .search("Rust", Some(&NewsCategory::Technology))
        .unwrap();
    assert_eq!(tech_results.len(), 1);
    assert!(tech_results[0].source.contains("Tech"));
}
