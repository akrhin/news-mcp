//! Cache unit tests

use news_mcp::cache::{NewsArticle, NewsCache, NewsCategory};
use std::str::FromStr;

#[test]
fn test_news_category_from_str() {
    assert_eq!(
        NewsCategory::from_str("technology").unwrap(),
        NewsCategory::Technology
    );
    assert_eq!(
        NewsCategory::from_str("tech").unwrap(),
        NewsCategory::Technology
    );
    // Unknown keys become Custom (no longer an error)
    let custom = NewsCategory::from_str("invalid").unwrap();
    assert!(matches!(custom, NewsCategory::Custom(ref n) if n == "invalid"));
}

#[test]
fn test_news_category_builtin() {
    let categories = NewsCategory::builtin();
    assert_eq!(categories.len(), 34);
}

#[test]
fn test_news_category_display() {
    assert_eq!(NewsCategory::Technology.display_name(), "Technology");
    assert_eq!(NewsCategory::Science.display_name(), "Science");
}

#[test]
fn test_news_article_creation() {
    let article = NewsArticle::new(
        "Test Title".to_string(),
        Some("Test Description".to_string()),
        "https://example.com".to_string(),
        "Test Source".to_string(),
        NewsCategory::Technology,
        None,
        Some("Test Author".to_string()),
    );

    assert_eq!(article.title, "Test Title");
    assert_eq!(article.description, Some("Test Description".to_string()));
    assert_eq!(article.link, "https://example.com");
    assert_eq!(article.source, "Test Source");
    assert_eq!(article.category, NewsCategory::Technology);
    assert_eq!(article.author, Some("Test Author".to_string()));
}

#[test]
fn test_cache_operations() {
    let cache = NewsCache::new(100);

    let articles = vec![
        NewsArticle::new(
            "Article 1".to_string(),
            Some("Description 1".to_string()),
            "https://example.com/1".to_string(),
            "Source 1".to_string(),
            NewsCategory::Technology,
            None,
            None,
        ),
        NewsArticle::new(
            "Article 2".to_string(),
            Some("Description 2".to_string()),
            "https://example.com/2".to_string(),
            "Source 2".to_string(),
            NewsCategory::Technology,
            None,
            None,
        ),
    ];

    // Set articles
    cache
        .set_category_news(NewsCategory::Technology, articles)
        .unwrap();

    // Get articles
    let retrieved = cache.get_category_news(&NewsCategory::Technology).unwrap();
    assert_eq!(retrieved.len(), 2);

    // Get empty category
    let empty = cache.get_category_news(&NewsCategory::Science).unwrap();
    assert_eq!(empty.len(), 0);
}

#[test]
fn test_cache_search() {
    let cache = NewsCache::new(100);

    let articles = vec![
        NewsArticle::new(
            "Rust Programming".to_string(),
            Some("Learn Rust programming language".to_string()),
            "https://example.com/rust".to_string(),
            "Tech Source".to_string(),
            NewsCategory::Technology,
            None,
            None,
        ),
        NewsArticle::new(
            "Python Programming".to_string(),
            Some("Learn Python programming language".to_string()),
            "https://example.com/python".to_string(),
            "Tech Source".to_string(),
            NewsCategory::Technology,
            None,
            None,
        ),
    ];

    cache
        .set_category_news(NewsCategory::Technology, articles)
        .unwrap();

    // Search for "rust"
    let results = cache
        .search("rust", Some(&NewsCategory::Technology))
        .unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].title, "Rust Programming");

    // Search for "programming" (matches both)
    let results = cache
        .search("programming", Some(&NewsCategory::Technology))
        .unwrap();
    assert_eq!(results.len(), 2);

    // Search in all categories
    let results = cache.search("programming", None).unwrap();
    assert_eq!(results.len(), 2);
}

#[test]
fn test_cache_max_articles_limit() {
    let cache = NewsCache::new(5); // Limit to 5 articles

    let articles: Vec<NewsArticle> = (0..10)
        .map(|i| {
            NewsArticle::new(
                format!("Article {}", i),
                Some(format!("Description {}", i)),
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

    let retrieved = cache.get_category_news(&NewsCategory::Technology).unwrap();
    assert_eq!(retrieved.len(), 5); // Should be limited to 5
}

#[test]
fn test_cache_categories() {
    let cache = NewsCache::new(100);

    let categories = cache.get_all_categories().unwrap();
    assert_eq!(categories.len(), 34); // 27 categories after removing empty ones

    for (category, count) in categories {
        assert!(count == 0); // Empty cache
        let _ = category.display_name(); // Should work
    }
}

#[test]
fn test_cache_clear() {
    let cache = NewsCache::new(100);

    let articles = vec![NewsArticle::new(
        "Article".to_string(),
        None,
        "https://example.com".to_string(),
        "Source".to_string(),
        NewsCategory::Technology,
        None,
        None,
    )];

    cache
        .set_category_news(NewsCategory::Technology, articles)
        .unwrap();

    assert_eq!(cache.total_article_count().unwrap(), 1);

    cache.clear().unwrap();

    assert_eq!(cache.total_article_count().unwrap(), 0);
}
