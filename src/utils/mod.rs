//! Utilities module
//!
//! Provides helper functions and constants for the news-mcp server.

use crate::cache::NewsCategory;
use reqwest_middleware::ClientBuilder;
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};

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

/// Format articles as compact one-liner (minimal context cost)
pub fn format_articles_as_compact(articles: &[crate::cache::NewsArticle]) -> String {
    if articles.is_empty() {
        return "No articles found.".to_string();
    }

    let mut output = String::new();
    output.push_str(&format!(
        "_{}_ — {} articles_\n\n",
        articles[0].category.display_name(),
        articles.len(),
    ));

    for article in articles {
        output.push_str(&format!("- **{}**", article.title));
        output.push_str(&format!(" | {}", article.source));
        if let Some(date) = &article.published_at {
            output.push_str(&format!(" | {}", date.format("%Y-%m-%d")));
        }
        output.push_str(&format!(" | ID: {}", article.id));
        output.push_str(&format!(" — [link]({})", article.link));
        output.push('\n');
        // One-line description if present (truncated to 120 chars)
        if let Some(desc) = &article.description {
            let first_line = desc.lines().next().unwrap_or(desc);
            let truncated: String = first_line.chars().take(120).collect();
            let suffix = if first_line.len() > 120 { "…" } else { "" };
            output.push_str(&format!("  _{}{}_\n", truncated, suffix));
        }
    }

    output
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
