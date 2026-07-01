//! News cache implementation
//!
//! Thread-safe in-memory cache for storing news articles by category.

use crate::error::{Error, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::RwLock;

/// Generate a stable ID from URL using simple hash
fn generate_id_from_url(url: &str) -> String {
    let hash = url
        .split('/')
        .rfind(|s| !s.is_empty())
        .map(|s| s.chars().take(12).collect::<String>())
        .unwrap_or_else(|| url.chars().take(12).collect());

    let clean_hash: String = hash
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
        .take(8)
        .collect();

    if clean_hash.is_empty() {
        format!("art-{:04x}", url.len() % 10000)
    } else {
        clean_hash
    }
}

/// News category — can be a builtin (Technology, HackerNews, China News, NewsNow)
/// or a user-defined custom category from config file.
#[derive(Debug, Clone, Eq)]
pub enum NewsCategory {
    // International
    Technology,
    Science,
    HackerNews,
    // China News categories
    Instant,
    Headlines,
    Politics,
    EastWest,
    Society,
    Finance,
    Life,
    Wellness,
    GreaterBayArea,
    Chinese,
    Video,
    Photo,
    Creative,
    Live,
    Education,
    Law,
    UnitedFront,
    EthnicUnity,
    Theory,
    Asean,
    // NewsNow Hot List categories
    WeiboHot,
    BaiduHot,
    ZhihuHot,
    DouyinHot,
    BilibiliHot,
    TiebaHot,
    ToutiaoHot,
    WallstreetcnHot,
    ClsHot,
    ThepaperHot,
    IfengHot,
    /// User-defined category from config `[feeds.<name>]`. Value is the config key.
    Custom(String),
}

// ── Serialize: plain string ─────────────────────────────────────────────
impl Serialize for NewsCategory {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        match self {
            Self::Custom(name) => serializer.serialize_str(name),
            other => serializer.serialize_str(other.config_key().as_ref()),
        }
    }
}

impl<'de> Deserialize<'de> for NewsCategory {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(Self::from_config_key(&s))
    }
}

impl PartialEq for NewsCategory {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Custom(a), Self::Custom(b)) => a == b,
            _ => std::mem::discriminant(self) == std::mem::discriminant(other),
        }
    }
}

impl std::hash::Hash for NewsCategory {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        if let Self::Custom(name) = self {
            name.hash(state);
        }
    }
}

impl std::str::FromStr for NewsCategory {
    type Err = std::convert::Infallible;

    /// Parse a category from a string. Never fails — unknown keys become `Custom(name)`.
    fn from_str(s: &str) -> std::result::Result<NewsCategory, Self::Err> {
        Ok(Self::from_config_key(s))
    }
}

impl NewsCategory {
    /// Normalize a user-provided string to a config key.
    fn normalize_key(s: &str) -> String {
        s.to_lowercase()
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
            .collect()
    }

    /// Parse a category from a config key. Falls through to Custom if not a builtin.
    pub fn from_config_key(s: &str) -> NewsCategory {
        let key = Self::normalize_key(s);
        match key.as_str() {
            "technology" | "tech" => NewsCategory::Technology,
            "science" => NewsCategory::Science,
            "hackernews" | "hn" => NewsCategory::HackerNews,
            "instant" | "即时新闻" => NewsCategory::Instant,
            "headlines" | "要闻导读" => NewsCategory::Headlines,
            "politics" | "时政新闻" => NewsCategory::Politics,
            "eastwest" | "东西问" => NewsCategory::EastWest,
            "society" | "社会新闻" => NewsCategory::Society,
            "finance" | "财经新闻" => NewsCategory::Finance,
            "life" | "生活" => NewsCategory::Life,
            "wellness" | "健康" => NewsCategory::Wellness,
            "greaterbayarea" | "大湾区" => NewsCategory::GreaterBayArea,
            "chinese" | "华人" => NewsCategory::Chinese,
            "video" | "视频" => NewsCategory::Video,
            "photo" | "图片" => NewsCategory::Photo,
            "creative" | "创意" => NewsCategory::Creative,
            "live" | "直播" => NewsCategory::Live,
            "education" | "教育" => NewsCategory::Education,
            "law" | "法治" => NewsCategory::Law,
            "unitedfront" | "同心" => NewsCategory::UnitedFront,
            "ethnicunity" | "铸牢中华民族共同体意识" => NewsCategory::EthnicUnity,
            "theory" | "理论" => NewsCategory::Theory,
            "asean" | "中国—东盟商贸资讯平台" => NewsCategory::Asean,
            "weibohot" | "微博热搜" => NewsCategory::WeiboHot,
            "baiduhot" | "百度热搜" => NewsCategory::BaiduHot,
            "zhihuhot" | "知乎热榜" => NewsCategory::ZhihuHot,
            "douyinhot" | "抖音热点" => NewsCategory::DouyinHot,
            "bilibilihot" | "b站热搜" => NewsCategory::BilibiliHot,
            "tiebahot" | "贴吧热议" => NewsCategory::TiebaHot,
            "toutiaohot" | "今日头条热点" => NewsCategory::ToutiaoHot,
            "wallstreetcnhot" | "华尔街见闻热门" => NewsCategory::WallstreetcnHot,
            "clshot" | "财联社热门" => NewsCategory::ClsHot,
            "thepaperhot" | "澎湃热门" => NewsCategory::ThepaperHot,
            "ifenghot" | "凤凰网热门" => NewsCategory::IfengHot,
            _ => NewsCategory::Custom(key),
        }
    }

    /// Config key for feed URL lookups (`config.feeds.<key>`).
    pub fn config_key(&self) -> Cow<'_, str> {
        use NewsCategory::*;
        match self {
            Technology => Cow::Borrowed("technology"),
            Science => Cow::Borrowed("science"),
            HackerNews => Cow::Borrowed("hackernews"),
            Instant => Cow::Borrowed("instant"),
            Headlines => Cow::Borrowed("headlines"),
            Politics => Cow::Borrowed("politics"),
            EastWest => Cow::Borrowed("eastwest"),
            Society => Cow::Borrowed("society"),
            Finance => Cow::Borrowed("finance"),
            Life => Cow::Borrowed("life"),
            Wellness => Cow::Borrowed("wellness"),
            GreaterBayArea => Cow::Borrowed("greaterbayarea"),
            Chinese => Cow::Borrowed("chinese"),
            Video => Cow::Borrowed("video"),
            Photo => Cow::Borrowed("photo"),
            Creative => Cow::Borrowed("creative"),
            Live => Cow::Borrowed("live"),
            Education => Cow::Borrowed("education"),
            Law => Cow::Borrowed("law"),
            UnitedFront => Cow::Borrowed("unitedfront"),
            EthnicUnity => Cow::Borrowed("ethnicunity"),
            Theory => Cow::Borrowed("theory"),
            Asean => Cow::Borrowed("asean"),
            WeiboHot => Cow::Borrowed("weibohot"),
            BaiduHot => Cow::Borrowed("baiduhot"),
            ZhihuHot => Cow::Borrowed("zhihuhot"),
            DouyinHot => Cow::Borrowed("douyinhot"),
            BilibiliHot => Cow::Borrowed("bilibilihot"),
            TiebaHot => Cow::Borrowed("tiebahot"),
            ToutiaoHot => Cow::Borrowed("toutiaohot"),
            WallstreetcnHot => Cow::Borrowed("wallstreetcnhot"),
            ClsHot => Cow::Borrowed("clshot"),
            ThepaperHot => Cow::Borrowed("thepaperhot"),
            IfengHot => Cow::Borrowed("ifenghot"),
            Custom(name) => Cow::Owned(name.clone()),
        }
    }

    /// All builtin categories (excludes user-defined Custom).
    pub fn builtin() -> Vec<NewsCategory> {
        vec![
            NewsCategory::Technology,
            NewsCategory::Science,
            NewsCategory::HackerNews,
            NewsCategory::Instant,
            NewsCategory::Headlines,
            NewsCategory::Politics,
            NewsCategory::EastWest,
            NewsCategory::Society,
            NewsCategory::Finance,
            NewsCategory::Life,
            NewsCategory::Wellness,
            NewsCategory::GreaterBayArea,
            NewsCategory::Chinese,
            NewsCategory::Video,
            NewsCategory::Photo,
            NewsCategory::Creative,
            NewsCategory::Live,
            NewsCategory::Education,
            NewsCategory::Law,
            NewsCategory::UnitedFront,
            NewsCategory::EthnicUnity,
            NewsCategory::Theory,
            NewsCategory::Asean,
            NewsCategory::WeiboHot,
            NewsCategory::BaiduHot,
            NewsCategory::ZhihuHot,
            NewsCategory::DouyinHot,
            NewsCategory::BilibiliHot,
            NewsCategory::TiebaHot,
            NewsCategory::ToutiaoHot,
            NewsCategory::WallstreetcnHot,
            NewsCategory::ClsHot,
            NewsCategory::ThepaperHot,
            NewsCategory::IfengHot,
        ]
    }

    /// Display name for user-facing output.
    pub fn display_name(&self) -> Cow<'_, str> {
        use NewsCategory::*;
        match self {
            Technology => Cow::Borrowed("Technology"),
            Science => Cow::Borrowed("Science"),
            HackerNews => Cow::Borrowed("Hacker News"),
            Instant => Cow::Borrowed("即时新闻"),
            Headlines => Cow::Borrowed("要闻导读"),
            Politics => Cow::Borrowed("时政新闻"),
            EastWest => Cow::Borrowed("东西问"),
            Society => Cow::Borrowed("社会新闻"),
            Finance => Cow::Borrowed("财经新闻"),
            Life => Cow::Borrowed("生活"),
            Wellness => Cow::Borrowed("健康"),
            GreaterBayArea => Cow::Borrowed("大湾区"),
            Chinese => Cow::Borrowed("华人"),
            Video => Cow::Borrowed("视频"),
            Photo => Cow::Borrowed("图片"),
            Creative => Cow::Borrowed("创意"),
            Live => Cow::Borrowed("直播"),
            Education => Cow::Borrowed("教育"),
            Law => Cow::Borrowed("法治"),
            UnitedFront => Cow::Borrowed("同心"),
            EthnicUnity => Cow::Borrowed("铸牢中华民族共同体意识"),
            Theory => Cow::Borrowed("理论"),
            Asean => Cow::Borrowed("中国—东盟商贸资讯平台"),
            WeiboHot => Cow::Borrowed("微博热搜"),
            BaiduHot => Cow::Borrowed("百度热搜"),
            ZhihuHot => Cow::Borrowed("知乎热榜"),
            DouyinHot => Cow::Borrowed("抖音热点"),
            BilibiliHot => Cow::Borrowed("B站热搜"),
            TiebaHot => Cow::Borrowed("贴吧热议"),
            ToutiaoHot => Cow::Borrowed("今日头条热点"),
            WallstreetcnHot => Cow::Borrowed("华尔街见闻热门"),
            ClsHot => Cow::Borrowed("财联社热门"),
            ThepaperHot => Cow::Borrowed("澎湃热门"),
            IfengHot => Cow::Borrowed("凤凰网热门"),
            Custom(name) => Cow::Owned(name.clone()),
        }
    }

    /// Human-readable description.
    pub fn description(&self) -> Cow<'_, str> {
        use NewsCategory::*;
        match self {
            Technology => Cow::Borrowed("Technology news from TechCrunch, Ars Technica, The Verge"),
            Science => Cow::Borrowed("Science news from ScienceDaily"),
            HackerNews => Cow::Borrowed("Top stories from Hacker News"),
            Instant => Cow::Borrowed("即时新闻 - 中国新闻网滚动新闻"),
            Headlines => Cow::Borrowed("要闻导读 - 中国新闻网重要新闻"),
            Politics => Cow::Borrowed("时政新闻 - 中国新闻网时政要闻"),
            EastWest => Cow::Borrowed("东西问 - 中国新闻网文化对话"),
            Society => Cow::Borrowed("社会新闻 - 中国新闻网社会百态"),
            Finance => Cow::Borrowed("财经新闻 - 中国新闻网财经资讯"),
            Life => Cow::Borrowed("生活 - 中国新闻网生活服务"),
            Wellness => Cow::Borrowed("健康 - 中国新闻网健康资讯"),
            GreaterBayArea => Cow::Borrowed("大湾区 - 中国新闻网粤港澳大湾区"),
            Chinese => Cow::Borrowed("华人 - 中国新闻网海外华人"),
            Video => Cow::Borrowed("视频 - 中国新闻网视频新闻"),
            Photo => Cow::Borrowed("图片 - 中国新闻网图片新闻"),
            Creative => Cow::Borrowed("创意 - 中国新闻网创意产业"),
            Live => Cow::Borrowed("直播 - 中国新闻网直播报道"),
            Education => Cow::Borrowed("教育 - 中国新闻网教育资讯"),
            Law => Cow::Borrowed("法治 - 中国新闻网法治新闻"),
            UnitedFront => Cow::Borrowed("同心 - 中国新闻网统战新闻"),
            EthnicUnity => Cow::Borrowed("铸牢中华民族共同体意识 - 中国新闻网民族新闻"),
            Theory => Cow::Borrowed("理论 - 中国新闻网理论动态"),
            Asean => Cow::Borrowed("中国—东盟商贸资讯平台 - 中国新闻网东盟资讯"),
            WeiboHot => Cow::Borrowed("微博热搜 - 实时热搜榜"),
            BaiduHot => Cow::Borrowed("百度热搜 - 百度实时热搜"),
            ZhihuHot => Cow::Borrowed("知乎热榜 - 知乎热门话题"),
            DouyinHot => Cow::Borrowed("抖音热点 - 抖音热门视频"),
            BilibiliHot => Cow::Borrowed("B站热搜 - 哔哩哔哩热搜榜"),
            TiebaHot => Cow::Borrowed("贴吧热议 - 百度贴吧热议话题"),
            ToutiaoHot => Cow::Borrowed("今日头条热点 - 头条热门资讯"),
            WallstreetcnHot => Cow::Borrowed("华尔街见闻热门 - 财经资讯"),
            ClsHot => Cow::Borrowed("财联社热门 - 金融快讯"),
            ThepaperHot => Cow::Borrowed("澎湃热门 - 澎湃新闻热点"),
            IfengHot => Cow::Borrowed("凤凰网热门 - 凤凰资讯热点"),
            Custom(name) => Cow::Owned(format!("User-defined feed: {name}")),
        }
    }
}

impl std::fmt::Display for NewsCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// News article structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewsArticle {
    /// Article ID (generated from URL hash for stable identification)
    pub id: String,
    /// Article title
    pub title: String,
    /// Article description/summary
    pub description: Option<String>,
    /// Full article content (fetched from HTML, may be None if not yet fetched)
    pub content: Option<String>,
    /// Article URL link
    pub link: String,
    /// Source name
    pub source: String,
    /// Category
    pub category: NewsCategory,
    /// Publication date
    pub published_at: Option<DateTime<Utc>>,
    /// Author name
    pub author: Option<String>,
}

impl NewsArticle {
    /// Create a new article
    pub fn new(
        title: String,
        description: Option<String>,
        link: String,
        source: String,
        category: NewsCategory,
        published_at: Option<DateTime<Utc>>,
        author: Option<String>,
    ) -> Self {
        let id = generate_id_from_url(&link);
        Self {
            id,
            title,
            description,
            content: None,
            link,
            source,
            category,
            published_at,
            author,
        }
    }

    /// Create a new article with full content
    #[allow(clippy::too_many_arguments)]
    pub fn with_content(
        title: String,
        description: Option<String>,
        content: Option<String>,
        link: String,
        source: String,
        category: NewsCategory,
        published_at: Option<DateTime<Utc>>,
        author: Option<String>,
    ) -> Self {
        let id = generate_id_from_url(&link);
        Self {
            id,
            title,
            description,
            content,
            link,
            source,
            category,
            published_at,
            author,
        }
    }
}

/// News cache structure
#[derive(Debug)]
pub struct NewsCache {
    /// Articles stored by category
    articles: RwLock<HashMap<NewsCategory, Vec<NewsArticle>>>,
    /// Last update time for each category
    last_updated: RwLock<HashMap<NewsCategory, DateTime<Utc>>>,
    /// Maximum articles per category
    max_articles_per_category: usize,
}

impl NewsCache {
    /// Create a new news cache
    pub fn new(max_articles_per_category: usize) -> Self {
        Self {
            articles: RwLock::new(HashMap::new()),
            last_updated: RwLock::new(HashMap::new()),
            max_articles_per_category,
        }
    }

    /// Get articles for a specific category
    pub fn get_category_news(&self, category: &NewsCategory) -> Result<Vec<NewsArticle>> {
        let articles = self
            .articles
            .read()
            .map_err(|e| Error::cache(e.to_string()))?;
        Ok(articles.get(category).cloned().unwrap_or_default())
    }

    /// Set articles for a specific category
    pub fn set_category_news(
        &self,
        category: NewsCategory,
        articles: Vec<NewsArticle>,
    ) -> Result<()> {
        let mut cache = self
            .articles
            .write()
            .map_err(|e| Error::cache(e.to_string()))?;
        let limited_articles = articles
            .into_iter()
            .take(self.max_articles_per_category)
            .collect();
        cache.insert(category.clone(), limited_articles);

        let mut updated = self
            .last_updated
            .write()
            .map_err(|e| Error::cache(e.to_string()))?;
        updated.insert(category, Utc::now());

        Ok(())
    }

    /// Search articles by query string
    pub fn search(&self, query: &str, category: Option<&NewsCategory>) -> Result<Vec<NewsArticle>> {
        let articles = self
            .articles
            .read()
            .map_err(|e| Error::cache(e.to_string()))?;
        let query_lower = query.to_lowercase();

        let results: Vec<NewsArticle> = if let Some(cat) = category {
            articles
                .get(cat)
                .map(|arts| {
                    arts.iter()
                        .filter(|a| {
                            a.title.to_lowercase().contains(&query_lower)
                                || a.description
                                    .as_ref()
                                    .map(|d| d.to_lowercase().contains(&query_lower))
                                    .unwrap_or(false)
                        })
                        .cloned()
                        .collect()
                })
                .unwrap_or_default()
        } else {
            articles
                .values()
                .flat_map(|arts| arts.iter())
                .filter(|a| {
                    a.title.to_lowercase().contains(&query_lower)
                        || a.description
                            .as_ref()
                            .map(|d| d.to_lowercase().contains(&query_lower))
                            .unwrap_or(false)
                })
                .cloned()
                .collect()
        };

        Ok(results)
    }

    /// Get all available categories with article counts.
    pub fn get_all_categories(&self) -> Result<Vec<(NewsCategory, usize)>> {
        let articles = self
            .articles
            .read()
            .map_err(|e| Error::cache(e.to_string()))?;

        let mut result: Vec<(NewsCategory, usize)> = NewsCategory::builtin()
            .into_iter()
            .map(|cat| {
                let count = articles.get(&cat).map(|v| v.len()).unwrap_or(0);
                (cat, count)
            })
            .collect();

        for (cat, arts) in articles.iter() {
            if matches!(cat, NewsCategory::Custom(_)) {
                result.push((cat.clone(), arts.len()));
            }
        }

        Ok(result)
    }

    /// Get last update time for a category
    pub fn get_last_updated(&self, category: &NewsCategory) -> Result<Option<DateTime<Utc>>> {
        let updated = self
            .last_updated
            .read()
            .map_err(|e| Error::cache(e.to_string()))?;
        Ok(updated.get(category).copied())
    }

    /// Get total article count across all categories
    pub fn total_article_count(&self) -> Result<usize> {
        let articles = self
            .articles
            .read()
            .map_err(|e| Error::cache(e.to_string()))?;
        Ok(articles.values().map(|v| v.len()).sum())
    }

    /// Get article by ID across all categories
    pub fn get_article_by_id(&self, id: &str) -> Result<Option<NewsArticle>> {
        let articles = self
            .articles
            .read()
            .map_err(|e| Error::cache(e.to_string()))?;

        for arts in articles.values() {
            for article in arts {
                if article.id == id {
                    return Ok(Some(article.clone()));
                }
            }
        }

        Ok(None)
    }

    /// Update content for an article identified by its URL
    pub fn update_article_content(&self, url: &str, content: String) -> Result<bool> {
        let mut articles = self
            .articles
            .write()
            .map_err(|e| Error::cache(e.to_string()))?;

        for arts in articles.values_mut() {
            for article in arts.iter_mut() {
                if article.link == url {
                    article.content = Some(content);
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// Clear all cached articles
    pub fn clear(&self) -> Result<()> {
        let mut articles = self
            .articles
            .write()
            .map_err(|e| Error::cache(e.to_string()))?;
        articles.clear();

        let mut updated = self
            .last_updated
            .write()
            .map_err(|e| Error::cache(e.to_string()))?;
        updated.clear();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_from_config_key() {
        assert_eq!(
            NewsCategory::from_config_key("technology"),
            NewsCategory::Technology
        );
        assert_eq!(
            NewsCategory::from_config_key("tech"),
            NewsCategory::Technology
        );
        assert_eq!(
            NewsCategory::from_config_key("hackernews"),
            NewsCategory::HackerNews
        );
    }

    #[test]
    fn test_custom_from_config_key() {
        let cat = NewsCategory::from_config_key("my-custom-feed");
        assert!(matches!(cat, NewsCategory::Custom(ref n) if n == "my-custom-feed"));
    }

    #[test]
    fn test_custom_case_insensitive() {
        let cat = NewsCategory::from_config_key("CISA-ALERTS");
        assert!(matches!(cat, NewsCategory::Custom(ref n) if n == "cisa-alerts"));
    }

    #[test]
    fn test_custom_serialize_roundtrip() {
        let cat = NewsCategory::Custom("cisa".to_string());
        let json = serde_json::to_string(&cat).unwrap();
        assert_eq!(json, "\"cisa\"");
        let back: NewsCategory = serde_json::from_str(&json).unwrap();
        assert_eq!(back, cat);
    }

    #[test]
    fn test_builtin_serialize() {
        let cat = NewsCategory::Technology;
        let json = serde_json::to_string(&cat).unwrap();
        assert_eq!(json, "\"technology\"");
        let back: NewsCategory = serde_json::from_str(&json).unwrap();
        assert_eq!(back, NewsCategory::Technology);
    }

    #[test]
    fn test_custom_eq() {
        let a = NewsCategory::Custom("cisa".to_string());
        let b = NewsCategory::Custom("cisa".to_string());
        let c = NewsCategory::Custom("other".to_string());
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn test_from_str_never_fails() {
        "anything".parse::<NewsCategory>().unwrap();
        "".parse::<NewsCategory>().unwrap();
        "!@#$".parse::<NewsCategory>().unwrap();
    }

    #[test]
    fn test_builtin_count() {
        assert_eq!(NewsCategory::builtin().len(), 34);
    }

    #[test]
    fn test_config_key_custom() {
        let cat = NewsCategory::Custom("my-feed".to_string());
        assert_eq!(cat.config_key().as_ref(), "my-feed");
    }
}
