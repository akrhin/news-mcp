//! Service unit tests

use news_mcp::cache::NewsCategory;
use news_mcp::service::NewsService;
use news_mcp::utils::get_feed_urls;

#[test]
fn test_news_service_creation() {
    let service = NewsService::new();
    // Service should be created successfully
    let _ = &service;
}

#[test]
fn test_feed_urls() {
    let tech_urls = get_feed_urls(&NewsCategory::Technology);
    assert!(!tech_urls.is_empty());
    assert!(tech_urls[0].contains("techcrunch"));

    let science_urls = get_feed_urls(&NewsCategory::Science);
    assert!(!science_urls.is_empty());
    assert!(science_urls[0].contains("sciencedaily"));

    // Test China News categories
    let instant_urls = get_feed_urls(&NewsCategory::Instant);
    assert!(!instant_urls.is_empty());
    assert!(instant_urls[0].contains("chinanews"));

    let finance_urls = get_feed_urls(&NewsCategory::Finance);
    assert!(!finance_urls.is_empty());
}

#[test]
fn test_parse_feed() {
    let sample_rss = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
    <channel>
        <title>Test Feed</title>
        <link>https://example.com</link>
        <description>Test feed description</description>
        <item>
            <title>Test Article 1</title>
            <link>https://example.com/article1</link>
            <description>Description of test article 1</description>
            <pubDate>Mon, 01 Jan 2024 00:00:00 UTC</pubDate>
        </item>
        <item>
            <title>Test Article 2</title>
            <link>https://example.com/article2</link>
            <description>Description of test article 2</description>
        </item>
    </channel>
</rss>"#;

    let service = NewsService::new();
    let articles = service
        .parse_feed(sample_rss.as_bytes(), NewsCategory::Technology)
        .unwrap();

    assert_eq!(articles.len(), 2);
    assert_eq!(articles[0].title, "Test Article 1");
    assert_eq!(articles[0].link, "https://example.com/article1");
    assert_eq!(articles[0].source, "Test Feed");
    assert_eq!(articles[0].category, NewsCategory::Technology);
    assert!(articles[0].description.is_some());
}

#[test]
fn test_parse_atom_feed() {
    let sample_atom = r#"<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
    <title>Atom Feed</title>
    <link href="https://example.com"/>
    <entry>
        <title>Atom Article</title>
        <link href="https://example.com/atom"/>
        <summary>Atom article summary</summary>
        <updated>2024-01-01T00:00:00Z</updated>
    </entry>
</feed>"#;

    let service = NewsService::new();
    let articles = service
        .parse_feed(sample_atom.as_bytes(), NewsCategory::Science)
        .unwrap();

    assert_eq!(articles.len(), 1);
    assert_eq!(articles[0].title, "Atom Article");
    assert_eq!(articles[0].source, "Atom Feed");
}
