//! Configuration module
//!
//! Provides configuration structures for the news-mcp server.
//! Supports TOML config files, environment variable overrides, and
//! configurable feed sources.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Main application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Server configuration
    pub server: ServerConfig,
    /// Poller configuration
    pub poller: PollerConfig,
    /// Cache configuration
    pub cache: CacheConfig,
    /// Article content fetch configuration
    #[serde(default)]
    pub article_fetch: ArticleFetchConfig,
    /// Logging configuration
    pub logging: LoggingConfig,
    /// Feed source definitions (category -> list of feed sources)
    #[serde(default)]
    pub feeds: HashMap<String, FeedSourceConfig>,
}

impl AppConfig {
    /// Load configuration from a TOML file
    pub fn from_file(path: &Path) -> crate::error::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: AppConfig = toml::from_str(&content)?;
        Ok(config)
    }

    /// Load configuration from a file path string
    pub fn from_path(path: impl AsRef<Path>) -> crate::error::Result<Self> {
        Self::from_file(path.as_ref())
    }

    /// Create default configuration with built-in feed sources
    pub fn default_config() -> Self {
        Self::default()
    }

    /// Apply environment variable overrides to the configuration
    pub fn apply_env_overrides(&mut self) {
        if let Ok(port) = std::env::var("NEWS_MCP_PORT") {
            if let Ok(port) = port.parse::<u16>() {
                self.server.port = port;
            }
        }
        if let Ok(host) = std::env::var("NEWS_MCP_HOST") {
            self.server.host = host;
        }
        if let Ok(mode) = std::env::var("NEWS_MCP_TRANSPORT") {
            self.server.transport_mode = mode;
        }
        if let Ok(interval) = std::env::var("NEWS_MCP_INTERVAL") {
            if let Ok(secs) = interval.parse::<u64>() {
                self.poller.interval_secs = secs;
            }
        }
        if let Ok(level) = std::env::var("NEWS_MCP_LOG_LEVEL") {
            self.logging.level = level;
        }
    }

    /// Get feed URLs for a category, falling back to defaults if not configured
    pub fn get_feed_urls(&self, category: &str) -> Vec<String> {
        let key = category.to_lowercase();
        self.feeds
            .get(&key)
            .map(|fc| fc.urls.clone())
            .unwrap_or_default()
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            poller: PollerConfig::default(),
            cache: CacheConfig::default(),
            article_fetch: ArticleFetchConfig::default(),
            logging: LoggingConfig::default(),
            feeds: default_feed_sources(),
        }
    }
}

/// Configuration for a single feed source category
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedSourceConfig {
    /// Display name for the category
    #[serde(default)]
    pub display_name: Option<String>,
    /// Description of the category
    #[serde(default)]
    pub description: Option<String>,
    /// RSS/Atom feed URLs
    #[serde(default)]
    pub urls: Vec<String>,
    /// Whether this category is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

/// Default feed sources with built-in URLs
fn default_feed_sources() -> HashMap<String, FeedSourceConfig> {
    let mut feeds = HashMap::new();

    feeds.insert(
        "technology".into(),
        FeedSourceConfig {
            display_name: Some("Technology".into()),
            description: Some("Technology news from TechCrunch, Ars Technica, The Verge".into()),
            urls: vec![
                "https://techcrunch.com/feed/".into(),
                "https://feeds.arstechnica.com/arstechnica/index".into(),
                "https://www.theverge.com/rss/index.xml".into(),
            ],
            enabled: true,
        },
    );

    feeds.insert(
        "science".into(),
        FeedSourceConfig {
            display_name: Some("Science".into()),
            description: Some("Science news from ScienceDaily".into()),
            urls: vec!["https://www.sciencedaily.com/rss/all.xml".into()],
            enabled: true,
        },
    );

    feeds.insert(
        "hackernews".into(),
        FeedSourceConfig {
            display_name: Some("Hacker News".into()),
            description: Some("Top stories from Hacker News (via API)".into()),
            urls: vec![], // Uses HN API, not RSS
            enabled: true,
        },
    );

    // China News categories
    let china_feeds = [
        (
            "instant",
            "即时新闻",
            "即时新闻 - 中国新闻网滚动新闻",
            "https://www.chinanews.com.cn/rss/scroll-news.xml",
        ),
        (
            "headlines",
            "要闻导读",
            "要闻导读 - 中国新闻网重要新闻",
            "https://www.chinanews.com.cn/rss/importnews.xml",
        ),
        (
            "politics",
            "时政新闻",
            "时政新闻 - 中国新闻网时政要闻",
            "https://www.chinanews.com.cn/rss/china.xml",
        ),
        (
            "eastwest",
            "东西问",
            "东西问 - 中国新闻网文化对话",
            "https://www.chinanews.com.cn/rss/dxw.xml",
        ),
        (
            "society",
            "社会新闻",
            "社会新闻 - 中国新闻网社会百态",
            "https://www.chinanews.com.cn/rss/society.xml",
        ),
        (
            "finance",
            "财经新闻",
            "财经新闻 - 中国新闻网财经资讯",
            "https://www.chinanews.com.cn/rss/finance.xml",
        ),
        (
            "life",
            "生活",
            "生活 - 中国新闻网生活服务",
            "https://www.chinanews.com.cn/rss/life.xml",
        ),
        (
            "wellness",
            "健康",
            "健康 - 中国新闻网健康资讯",
            "https://www.chinanews.com.cn/rss/jk.xml",
        ),
        (
            "greaterbayarea",
            "大湾区",
            "大湾区 - 中国新闻网粤港澳大湾区",
            "https://www.chinanews.com.cn/rss/dwq.xml",
        ),
        (
            "chinese",
            "华人",
            "华人 - 中国新闻网海外华人",
            "https://www.chinanews.com.cn/rss/chinese.xml",
        ),
        (
            "video",
            "视频",
            "视频 - 中国新闻网视频新闻",
            "https://www.chinanews.com.cn/rss/sp.xml",
        ),
        (
            "photo",
            "图片",
            "图片 - 中国新闻网图片新闻",
            "https://www.chinanews.com.cn/rss/photo.xml",
        ),
        (
            "creative",
            "创意",
            "创意 - 中国新闻网创意产业",
            "https://www.chinanews.com.cn/rss/chuangyi.xml",
        ),
        (
            "live",
            "直播",
            "直播 - 中国新闻网直播报道",
            "https://www.chinanews.com.cn/rss/zhibo.xml",
        ),
        (
            "education",
            "教育",
            "教育 - 中国新闻网教育资讯",
            "https://www.chinanews.com.cn/rss/edu.xml",
        ),
        (
            "law",
            "法治",
            "法治 - 中国新闻网法治新闻",
            "https://www.chinanews.com.cn/rss/fz.xml",
        ),
        (
            "unitedfront",
            "同心",
            "同心 - 中国新闻网统战新闻",
            "https://www.chinanews.com.cn/rss/tx.xml",
        ),
        (
            "ethnicunity",
            "铸牢中华民族共同体意识",
            "铸牢中华民族共同体意识 - 中国新闻网民族新闻",
            "https://www.chinanews.com.cn/rss/mz.xml",
        ),
        (
            "theory",
            "理论",
            "理论 - 中国新闻网理论动态",
            "https://www.chinanews.com.cn/rss/theory.xml",
        ),
        (
            "asean",
            "中国—东盟商贸资讯平台",
            "中国—东盟商贸资讯平台 - 中国新闻网东盟资讯",
            "https://www.chinanews.com.cn/rss/aseaninfo.xml",
        ),
    ];

    for (key, name, desc, url) in china_feeds {
        feeds.insert(
            key.into(),
            FeedSourceConfig {
                display_name: Some(name.into()),
                description: Some(desc.into()),
                urls: vec![url.into()],
                enabled: true,
            },
        );
    }

    // NewsNow Hot List categories
    let newsnow_feeds = [
        ("weibohot", "微博热搜", "微博热搜 - 实时热搜榜"),
        ("baiduhot", "百度热搜", "百度热搜 - 百度实时热搜"),
        ("zhihuhot", "知乎热榜", "知乎热榜 - 知乎热门话题"),
        ("douyinhot", "抖音热点", "抖音热点 - 抖音热门视频"),
        ("bilibilihot", "B站热搜", "B站热搜 - 哔哩哔哩热搜榜"),
        ("tiebahot", "贴吧热议", "贴吧热议 - 百度贴吧热议话题"),
        ("toutiaohot", "今日头条热点", "今日头条热点 - 头条热门资讯"),
        (
            "wallstreetcnhot",
            "华尔街见闻热门",
            "华尔街见闻热门 - 财经资讯",
        ),
        ("clshot", "财联社热门", "财联社热门 - 金融快讯"),
        ("thepaperhot", "澎湃热门", "澎湃热门 - 澎湃新闻热点"),
        ("ifenghot", "凤凰网热门", "凤凰网热门 - 凤凰资讯热点"),
    ];

    for (key, name, desc) in newsnow_feeds {
        feeds.insert(
            key.into(),
            FeedSourceConfig {
                display_name: Some(name.into()),
                description: Some(desc.into()),
                urls: vec![], // Uses NewsNow API, not RSS
                enabled: true,
            },
        );
    }

    feeds
}

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Server name
    #[serde(default = "default_server_name")]
    pub name: String,

    /// Server version
    #[serde(default = "default_server_version")]
    pub version: String,

    /// Server host
    #[serde(default = "default_host")]
    pub host: String,

    /// Server port
    #[serde(default = "default_port")]
    pub port: u16,

    /// Transport mode: stdio, http, sse, hybrid
    #[serde(default = "default_transport_mode")]
    pub transport_mode: String,
}

fn default_server_name() -> String {
    "news-mcp".to_string()
}

fn default_server_version() -> String {
    "0.1.0".to_string()
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    8080
}

fn default_transport_mode() -> String {
    "stdio".to_string()
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            name: default_server_name(),
            version: default_server_version(),
            host: default_host(),
            port: default_port(),
            transport_mode: default_transport_mode(),
        }
    }
}

/// Poller configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PollerConfig {
    /// Polling interval in seconds
    #[serde(default = "default_poll_interval")]
    pub interval_secs: u64,

    /// Enable background polling
    #[serde(default = "default_poll_enabled")]
    pub enabled: bool,
}

fn default_poll_interval() -> u64 {
    3600 // 1 hour
}

fn default_poll_enabled() -> bool {
    true
}

impl Default for PollerConfig {
    fn default() -> Self {
        Self {
            interval_secs: default_poll_interval(),
            enabled: default_poll_enabled(),
        }
    }
}

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Maximum articles per category
    #[serde(default = "default_max_articles")]
    pub max_articles_per_category: usize,
}

fn default_max_articles() -> usize {
    100
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_articles_per_category: default_max_articles(),
        }
    }
}

/// Article content fetch configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArticleFetchConfig {
    /// HTTP request timeout in seconds for fetching article content
    #[serde(default = "default_fetch_timeout")]
    pub fetch_timeout_secs: u64,
    /// Max chars of article content returned to client (default: 2000)
    #[serde(default = "default_max_chars")]
    pub max_chars: usize,
}

fn default_fetch_timeout() -> u64 {
    10
}

fn default_max_chars() -> usize {
    2000
}

impl Default for ArticleFetchConfig {
    fn default() -> Self {
        Self {
            fetch_timeout_secs: default_fetch_timeout(),
            max_chars: default_max_chars(),
        }
    }
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level
    #[serde(default = "default_log_level")]
    pub level: String,

    /// Enable console logging
    #[serde(default = "default_console_enabled")]
    pub enable_console: bool,
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_console_enabled() -> bool {
    true
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            enable_console: default_console_enabled(),
        }
    }
}

/// Transport mode enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportMode {
    /// Standard input/output mode
    Stdio,
    /// HTTP mode (Streamable HTTP)
    Http,
    /// Server-Sent Events mode
    Sse,
    /// Hybrid mode (HTTP + SSE)
    Hybrid,
}

impl std::str::FromStr for TransportMode {
    type Err = crate::error::Error;

    fn from_str(s: &str) -> crate::error::Result<Self> {
        match s.to_lowercase().as_str() {
            "stdio" => Ok(TransportMode::Stdio),
            "http" => Ok(TransportMode::Http),
            "sse" => Ok(TransportMode::Sse),
            "hybrid" => Ok(TransportMode::Hybrid),
            _ => Err(crate::error::Error::config(
                "transport_mode",
                format!("invalid transport mode: {}", s),
            )),
        }
    }
}

impl std::fmt::Display for TransportMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransportMode::Stdio => write!(f, "stdio"),
            TransportMode::Http => write!(f, "http"),
            TransportMode::Sse => write!(f, "sse"),
            TransportMode::Hybrid => write!(f, "hybrid"),
        }
    }
}
