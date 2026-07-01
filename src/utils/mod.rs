//! Utilities module
//!
//! Provides helper functions and constants for the news-mcp server.

use crate::cache::NewsCategory;
use encoding_rs::{Encoding, UTF_8};
use reqwest_middleware::ClientBuilder;
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};

/// Detect encoding from XML declaration (<?xml encoding="..."?>) and
/// re-encode raw bytes to UTF-8 String. Searches at byte level so non-UTF-8
/// encodings (KOI8-R, Windows-1251, GB2312, etc.) don't corrupt the search.
pub fn decode_xml_bytes(raw: &[u8]) -> String {
    // Search for b"encoding=" in the raw bytes (before any UTF-8 conversion)
    if let Some(enc_start) = raw.windows(9).position(|w| w == b"encoding=") {
        let after = &raw[enc_start + 9..];
        if after.is_empty() {
            return String::from_utf8_lossy(raw).into_owned();
        }
        // Determine the quote character
        let quote = after[0];
        if quote == b'"' || quote == b'\'' {
            if let Some(enc_len) = after[1..].iter().position(|&b| b == quote) {
                let enc_name = &after[1..=enc_len];
                if let Some(encoding) = Encoding::for_label(enc_name) {
                    if encoding != UTF_8 {
                        let (decoded, _, _) = encoding.decode(raw);
                        return decoded.into_owned();
                    }
                }
            }
        }
    }
    // Fallback: attempt UTF-8, then fall back to lossy
    String::from_utf8_lossy(raw).into_owned()
}

/// Get fallback RSS feed URLs for a category when no config is available
pub fn get_feed_urls(category: &NewsCategory) -> Vec<&'static str> {
    match category {
        NewsCategory::Technology => vec![
            "https://techcrunch.com/feed/",
            "https://feeds.arstechnica.com/arstechnica/index",
            "https://www.theverge.com/rss/index.xml",
        ],
        NewsCategory::Science => vec!["https://www.sciencedaily.com/rss/all.xml"],
        NewsCategory::HackerNews => vec![],
        // China News categories
        NewsCategory::Instant => vec!["https://www.chinanews.com.cn/rss/scroll-news.xml"],
        NewsCategory::Headlines => vec!["https://www.chinanews.com.cn/rss/importnews.xml"],
        NewsCategory::Politics => vec!["https://www.chinanews.com.cn/rss/china.xml"],
        NewsCategory::EastWest => vec!["https://www.chinanews.com.cn/rss/dxw.xml"],
        NewsCategory::Society => vec!["https://www.chinanews.com.cn/rss/society.xml"],
        NewsCategory::Finance => vec!["https://www.chinanews.com.cn/rss/finance.xml"],
        NewsCategory::Life => vec!["https://www.chinanews.com.cn/rss/life.xml"],
        NewsCategory::Wellness => vec!["https://www.chinanews.com.cn/rss/jk.xml"],
        NewsCategory::GreaterBayArea => vec!["https://www.chinanews.com.cn/rss/dwq.xml"],
        NewsCategory::Chinese => vec!["https://www.chinanews.com.cn/rss/chinese.xml"],
        NewsCategory::Video => vec!["https://www.chinanews.com.cn/rss/sp.xml"],
        NewsCategory::Photo => vec!["https://www.chinanews.com.cn/rss/photo.xml"],
        NewsCategory::Creative => vec!["https://www.chinanews.com.cn/rss/chuangyi.xml"],
        NewsCategory::Live => vec!["https://www.chinanews.com.cn/rss/zhibo.xml"],
        NewsCategory::Education => vec!["https://www.chinanews.com.cn/rss/edu.xml"],
        NewsCategory::Law => vec!["https://www.chinanews.com.cn/rss/fz.xml"],
        NewsCategory::UnitedFront => vec!["https://www.chinanews.com.cn/rss/tx.xml"],
        NewsCategory::EthnicUnity => vec!["https://www.chinanews.com.cn/rss/mz.xml"],
        NewsCategory::Theory => vec!["https://www.chinanews.com.cn/rss/theory.xml"],
        NewsCategory::Asean => vec!["https://www.chinanews.com.cn/rss/aseaninfo.xml"],
        _ => vec![],
    }
}

/// Build HTTP client with retry middleware
pub fn build_http_client_with_retry() -> reqwest_middleware::ClientWithMiddleware {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .connect_timeout(std::time::Duration::from_secs(10))
        .user_agent("news-mcp/0.1.0")
        .build()
        .expect("Failed to create HTTP client");

    // Create retry policy with exponential backoff
    let retry_policy = ExponentialBackoff::builder().build_with_max_retries(3);

    ClientBuilder::new(client)
        .with(RetryTransientMiddleware::new_with_policy(retry_policy))
        .build()
}

/// Initialize logging based on configuration
pub fn init_logging(level: &str, enable_console: bool) {
    if enable_console {
        let filter = tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(level));

        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_target(false)
            .with_thread_ids(false)
            .with_writer(std::io::stderr)
            .init();
    }
}

/// Format articles for output
pub fn format_articles_as_markdown(articles: &[crate::cache::NewsArticle]) -> String {
    if articles.is_empty() {
        return "No articles found.".to_string();
    }

    let mut output = String::new();
    output.push_str("# News Articles\n\n");

    for article in articles {
        output.push_str(&format!("## {}\n\n", article.title));
        output.push_str(&format!("- **ID**: {}\n\n", article.id));

        if let Some(content) = &article.content {
            output.push_str(&format!("{}\n\n", content));
        } else if let Some(desc) = &article.description {
            output.push_str(&format!("{}\n\n", desc));
        }

        output.push_str(&format!("- **Source**: {}\n", article.source));
        output.push_str(&format!("- **Link**: {}\n", article.link));

        if let Some(date) = &article.published_at {
            output.push_str(&format!(
                "- **Published**: {}\n",
                date.format("%Y-%m-%d %H:%M UTC")
            ));
        }

        if let Some(author) = &article.author {
            output.push_str(&format!("- **Author**: {}\n", author));
        }

        output.push_str("\n---\n\n");
    }

    output
}

/// Format articles as JSON
pub fn format_articles_as_json(articles: &[crate::cache::NewsArticle]) -> String {
    serde_json::to_string_pretty(articles).unwrap_or_else(|_| "[]".to_string())
}

/// Format articles as plain text
pub fn format_articles_as_text(articles: &[crate::cache::NewsArticle]) -> String {
    if articles.is_empty() {
        return "No articles found.".to_string();
    }

    let mut output = String::new();

    for (i, article) in articles.iter().enumerate() {
        output.push_str(&format!("{}. [{}] {}\n", i + 1, article.id, article.title));

        if let Some(content) = &article.content {
            output.push_str(&format!("   {}\n", content));
        } else if let Some(desc) = &article.description {
            output.push_str(&format!("   {}\n", desc));
        }

        output.push_str(&format!(
            "   Source: {} | Link: {}\n",
            article.source, article.link
        ));

        if let Some(date) = &article.published_at {
            output.push_str(&format!(
                "   Published: {}\n",
                date.format("%Y-%m-%d %H:%M UTC")
            ));
        }

        output.push('\n');
    }

    output
}
