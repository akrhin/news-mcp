# News MCP Server Architecture / 架构设计

This document describes the architecture and design decisions of the News MCP Server.

本文档描述 News MCP Server 的架构和设计决策。

## Table of Contents / 目录

- [System Overview](#system-overview)
- [Core Components](#core-components)
- [Data Flow](#data-flow)
- [Design Decisions](#design-decisions)
- [Extension Points](#extension-points)
- [Technology Stack](#technology-stack)

## System Overview / 系统概述

News MCP Server is a Rust-based MCP (Model Context Protocol) server that fetches news from multiple RSS feeds and APIs with background polling and caching.

News MCP Server 是一个基于 Rust 的 MCP 服务器，从多个 RSS 源和 API 获取新闻，支持后台轮询和缓存。

### Architecture Diagram / 架构图

```
┌─────────────────────────────────────────────────────────────────────┐
│                           Clients                                    │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐                  │
│  │Claude Desktop│  │ HTTP API   │  │ SSE Client  │                  │
│  └─────────────┘  └─────────────┘  └─────────────┘                  │
└────────────────────────────┬────────────────────────────────────────┘
                             │ MCP Protocol
                             ▼
┌─────────────────────────────────────────────────────────────────────┐
│                     Transport Layer                                  │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐                 │
│  │ stdio   │  │  HTTP   │  │  SSE    │  │ hybrid  │                 │
│  └─────────┘  └─────────┘  └─────────┘  └─────────┘                 │
└────────────────────────────┬────────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────────┐
│                     MCP Handler                                       │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │  Tool Registry: get_news, search_news, health_check, etc.   │    │
│  └─────────────────────────────────────────────────────────────┘    │
└────────────────────────────┬────────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────────┐
│                      News Cache                                       │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │  RwLock<HashMap<NewsCategory, Vec<NewsArticle>>>            │    │
│  └─────────────────────────────────────────────────────────────┘    │
└────────────────────────────┬────────────────────────────────────────┘
                             │
          ┌──────────────────┴──────────────────┐
          │                                      │
          ▼                                      ▼
┌─────────────────────┐              ┌─────────────────────┐
│   News Poller       │              │   Manual Refresh    │
│ (Background Task)   │              │   (refresh_news)    │
└─────────────────────┘              └─────────────────────┘
          │                                      │
          └──────────────────┬──────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    News Sources                                       │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │  NewsSource Trait (Pluggable)                                 │   │
│  │  ┌────────────────┐  ┌────────────────┐  ┌────────────────┐ │   │
│  │  │ NewsService    │  │  HnService     │  │ (Custom impl)  │ │   │
│  │  │ (RSS Feeds)    │  │ (HN API)       │  │               │ │   │
│  │  └────────────────┘  └────────────────┘  └────────────────┘ │   │
│  └──────────────────────────────────────────────────────────────┘   │
└────────────────────────────┬────────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    External Sources                                   │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌───────────────────────┐   │
│  │TechCrunch│ │ Ars Tech │ │ The Verge│ │ China News (21 cats)  │   │
│  └──────────┘ └──────────┘ └──────────┘ └───────────────────────┘   │
│  ┌──────────┐ ┌──────────┐                                           │
│  │ScienceDaily│ │Hacker News│                                          │
│  └──────────┘ ┌──────────┘                                           │
└─────────────────────────────────────────────────────────────────────┘
```

## Core Components / 核心组件

### 1. Cache Layer (`src/cache/`)

**Responsibility**: Thread-safe in-memory storage for news articles. **No persistence to disk.**

**职责**: 线程安全的内存新闻文章存储。**不持久化到磁盘。**

**Обязанность**: Потокобезопасное хранение новостных статей в оперативной памяти. **Без сохранения на диск.**

The server maintains **two independent in-memory caches**:

Сервер поддерживает **два независимых кеша в памяти**:

| Cache | Struct | Key | Value | Purpose / Назначение |
|-------|--------|-----|-------|----------------------|
| **NewsCache** | `RwLock<HashMap<NewsCategory, Vec<NewsArticle>>>` | Category | Article list | Per-category metadata index / Индекс метаданных по категориям |
| **ArticleCache** | `RwLock<HashMap<String, CachedArticle>>` | URL | Full text | Per-URL full article content / Полный текст статьи по URL |

#### NewsCache

```rust
pub struct NewsCache {
    articles: RwLock<HashMap<NewsCategory, Vec<NewsArticle>>>,
    last_updated: RwLock<HashMap<NewsCategory, DateTime<Utc>>>,
    max_articles_per_category: usize,
}
```

Articles hold **metadata only** on initial fetch (title, description, link, source). The `content` field starts as `None` — filled lazily on first `get_article_content` call. The poller **replaces** the entire category list on each cycle (no incremental merge).

文章在初始获取时只包含**元数据**（标题、描述、链接、来源）。`content` 字段初始为 `None` — 在首次调用 `get_article_content` 时惰性填充。轮询器每次循环**替换**整个类别的文章列表（无增量合并）。

При первичной загрузке статьи содержат только **метаданные** (заголовок, описание, ссылка, источник). Поле `content` изначально `None` — заполняется **лениво**, при первом вызове `get_article_content`. Поллер **заменяет** весь список категории целиком каждый цикл (без инкрементального слияния).

**Key Features**:
- Thread-safe via `RwLock` (allows concurrent reads)
- Per-category storage with configurable limits
- Timestamp tracking for freshness
- Full-text search across title/description

**关键特性**:
- 通过 `RwLock` 实现线程安全（允许并发读取）
- 按类别存储，可配置限制
- 时间戳跟踪数据新鲜度
- 支持标题/描述全文搜索

#### ArticleCache (`src/cache/article_cache.rs`)

```rust
pub struct ArticleCache {
    articles: RwLock<HashMap<String, CachedArticle>>,
    max_articles: usize,
}
```

Where `CachedArticle` stores:
- `content: String` — cleaned plain text from HTML
- `fetched_at: DateTime<Utc>` — timestamp for eviction ordering
- `word_count: usize` — computed on insertion

**Lazy loading:** `get_article_content(id)` first checks ArticleCache by resolved URL. On miss, fetches HTML, extracts text, stores in **both** caches (ArticleCache + NewsArticle.content). Subsequent calls serve from memory in microseconds.

**Eviction (FIFO by age):** At capacity (default 100 articles), the oldest entry (by `fetched_at`) is evicted when a new URL arrives.

**惰性加载:** `get_article_content(id)` 首先按解析后的 URL 检查 ArticleCache。未命中时，获取 HTML，提取文本，存储在**两个**缓存中（ArticleCache + NewsArticle.content）。后续调用在微秒内从内存返回。

**淘汰策略（按时间的 FIFO）:** 当缓存达到容量上限（默认 100 篇文章）时，新 URL 到达时最旧的条目（按 `fetched_at`）将被淘汰。

**Ленивая загрузка:** `get_article_content(id)` сначала проверяет ArticleCache по URL. При промахе — забирает HTML, извлекает текст, сохраняет в **оба** кеша (ArticleCache + NewsArticle.content). Последующие вызовы отдают из памяти за микросекунды.

**Вытеснение (FIFO по времени):** При заполнении кеша (по умолчанию 100 статей) самая старая запись (по `fetched_at`) удаляется при добавлении нового URL.

#### Persistence / 持久化 / Персистентность

**Why no disk? / 为什么要使用无持久化设计? / Почему не храним на диске?**

Simplicity and resource footprint. The MCP server is ephemeral — designed to run alongside an AI assistant session. RSS feeds poll hourly, so at most one hour of content is lost on restart. Full article text persistence would require a bounded SQLite store with cleanup logic, adding complexity for marginal gain given the use case.

简单性和资源占用。MCP 服务器是临时的——设计为伴随 AI 助手会话运行。RSS 源每小时轮询一次，重启时最多丢失一小时的缓存内容。完整文章文本的持久化需要带清理逻辑的有界 SQLite 存储，考虑到使用场景，增加复杂性的收益有限。

Простота и экономия ресурсов. MCP-сервер эфемерен — он спроектирован для работы рядом с сессией AI-ассистента. RSS-ленты опрашиваются раз в час, так что при перезапуске теряется не больше часа контента. Постоянное хранение полных текстов потребовало бы SQLite с логикой очистки, что добавляет сложности без ощутимой выгоды в данном сценарии.

**Cache lifecycle / 缓存生命周期 / Жизненный цикл кеша:**

| Event / Событие | NewsCache | ArticleCache |
|-------|-----------|--------------|
| Server start / Запуск сервера | Empty / Пуст | Empty / Пуст |
| First poll cycle / Первый цикл поллинга | Populated (metadata only) | Still empty / Всё ещё пуст |
| First `get_article_content` per URL / Первый запрос контента статьи | Content field filled / Поле content заполнено | URL entry added / Добавлена запись по URL |
| Server restart / Перезапуск сервера | **Lost / Потерян** | **Lost / Потерян** |

### 2. Poller (`src/poller/`)

**Responsibility**: Background task that periodically fetches news from all sources.

**职责**: 定期从所有源获取新闻的后台任务。

**Implementation**:
```rust
pub struct NewsPoller {
    sources: Vec<Arc<dyn NewsSource>>,
    cache: Arc<NewsCache>,
    config: PollerConfig,
    running: AtomicBool,
    initial_poll_completed: AtomicBool,
}
```

**Key Features**:
- Pluggable sources via `NewsSource` trait
- Configurable polling interval
- Atomic flags for state tracking
- Wait mechanism for initial poll completion

**关键特性**:
- 通过 `NewsSource` trait 实现可插拔源
- 可配置轮询间隔
- 使用 Atomic 标志跟踪状态
- 提供初始轮询完成等待机制

### 3. Service Layer (`src/service/`)

**Responsibility**: Fetch and parse news from external sources.

**职责**: 从外部源获取和解析新闻。

**NewsSource Trait** (Extensibility Pattern):
```rust
#[async_trait]
pub trait NewsSource: Send + Sync {
    fn name(&self) -> &str;
    async fn fetch(&self) -> Result<HashMap<NewsCategory, Vec<NewsArticle>>>;
}
```

**Implementations**:
- `NewsService`: RSS/Atom feed fetching with retry middleware
- `HnService`: Hacker News API via `newswrap` crate

**实现**:
- `NewsService`: RSS/Atom 获取，带重试中间件
- `HnService`: 通过 `newswrap` crate 使用 Hacker News API

### 4. Server (`src/server/`)

**Responsibility**: MCP protocol implementation and tool handling.

**职责**: MCP 协议实现和工具处理。

**Transport Modes**:
| Mode | Use Case | Description |
|------|----------|-------------|
| `stdio` | Claude Desktop | Standard input/output communication |
| `http` | Web/API clients | Streamable HTTP protocol |
| `sse` | Real-time updates | Server-Sent Events |
| `hybrid` | Mixed clients | HTTP + SSE combined |

**传输模式**:
| 模式 | 用途 | 描述 |
|------|----------|-------------|
| `stdio` | Claude Desktop | 标准输入/输出通信 |
| `http` | Web/API 客户端 | Streamable HTTP 协议 |
| `sse` | 实时更新 | Server-Sent Events |
| `hybrid` | 混合客户端 | HTTP + SSE 组合 |

### 5. Tools (`src/tools/`)

**MCP Tools Provided**:

| Tool | Function | Parameters |
|------|----------|------------|
| `get_news` | Fetch articles by category | `category`, `limit`, `format` |
| `search_news` | Search cached articles | `query`, `category`, `limit` |
| `get_categories` | List available categories | - |
| `health_check` | Server status and stats | `check_type`, `verbose` |
| `refresh_news` | Manual cache refresh | `category` (optional) |

**Output Formats**: `markdown`, `json`, `text`

## Data Flow / 数据流

### Startup Flow / 启动流程

```
1. Load config (TOML + env overrides)
2. Create NewsCache instance
3. Initialize NewsSource implementations (NewsService, HnService)
4. Create NewsPoller with sources and cache
5. Start poller background task
6. Wait for initial poll completion
7. Start transport layer (stdio/http/sse)
8. Ready to serve MCP requests
```

### Request Flow / 请求流程

```
Client Request → Transport → MCP Handler → Tool Registry → Tool Implementation
                                                                    ↓
                                                              Read from Cache
                                                                    ↓
                                                              Format Response
                                                                    ↓
Client Response ← Transport ← MCP Handler ← Tool Registry ←───┘
```

### Polling Flow / 轮询流程

```
┌─────────┐
│  Start  │
└────┬────┘
     │
     ▼
┌─────────────────┐
│ Initial Poll    │──► Fetch all sources concurrently
└────┬────────────┘    Parse RSS/API responses
     │                 Update cache
     │                 Set initial_poll_completed = true
     ▼
┌─────────────────┐
│ Sleep (interval)│
└────┬────────────┘
     │
     ▼
┌─────────────────┐
│ Poll Cycle      │──► Same as initial poll
└────┬────────────┘
     │
     └◄─── Loop until stopped
```

## Design Decisions / 设计决策

### 1. NewsSource Trait for Extensibility

**Decision**: Use a trait-based architecture for news sources.

**Rationale**: 
- Allows adding new sources without modifying core code
- Supports different fetch mechanisms (RSS, API, scraping)
- Enables third-party extensions

**决策**: 使用 trait 架构实现新闻源。

**理由**:
- 允许添加新源而不修改核心代码
- 支持不同的获取机制（RSS、API、爬虫）
- 支持第三方扩展

### 2. RwLock for Thread Safety

**Decision**: Use `RwLock<HashMap>` instead of `DashMap` or other concurrent maps.

**Rationale**:
- Multiple readers (MCP tools) vs single writer (poller)
- Simpler implementation for moderate concurrency
- Consider `DashMap` for higher concurrent write scenarios

**决策**: 使用 `RwLock<HashMap>` 而非 `DashMap`。

**理由**:
- 多读者（MCP 工具）vs 单写者（轮询器）
- 中等并发场景实现更简单
- 高并发写场景可考虑 `DashMap`

### 3. Concurrent Fetching

**Decision**: Fetch multiple RSS feeds concurrently using `futures::future::join_all`.

**Rationale**:
- Reduces total fetch time for categories with multiple sources
- Technology category has 3 feeds → 3x faster with concurrency
- Configurable retry middleware handles failures

**决策**: 使用 `futures::future::join_all` 并发获取多个 RSS 源。

**理由**:
- 减少多源类别的总获取时间
- Technology 类别有 3 个源 → 并发后快 3 倍
- 可配置重试中间件处理失败

### 4. Configurable Feed Sources

**Decision**: Support TOML configuration with environment variable overrides.

**Rationale**:
- Users can customize feeds without code changes
- Docker deployments can use env vars
- Falls back to built-in defaults

**决策**: 支持 TOML 配置和环境变量覆盖。

**理由**:
- 用户无需修改代码即可自定义源
- Docker 部署可使用环境变量
- 回退到内置默认值

## Extension Points / 扩展点

### Adding a New News Source / 添加新新闻源

1. Implement `NewsSource` trait:
```rust
#[async_trait]
impl NewsSource for MyCustomSource {
    fn name(&self) -> &str {
        "My Custom Source"
    }
    
    async fn fetch(&self) -> Result<HashMap<NewsCategory, Vec<NewsArticle>>> {
        // Your fetch logic
    }
}
```

2. Register in poller:
```rust
let sources: Vec<Arc<dyn NewsSource>> = vec![
    Arc::new(NewsService::with_config(config.clone())),
    Arc::new(HnService::new()),
    Arc::new(MyCustomSource::new()), // Add your source
];
```

### Adding a New MCP Tool / 添加新 MCP 工具

1. Create tool file in `src/tools/`:
```rust
pub struct MyTool;

impl MyTool {
    pub async fn execute(&self, params: MyParams, cache: &NewsCache) -> Result<String> {
        // Tool logic
    }
}
```

2. Register in `ToolRegistry`:
```rust
registry.register("my_tool", Box::new(MyTool));
```

### Adding a New Category / 添加新类别

1. Add enum variant in `src/cache/news_cache.rs`:
```rust
pub enum NewsCategory {
    // ... existing
    MyNewCategory,
}
```

2. Add feed URLs in config or `src/utils/mod.rs`:
```rust
NewsCategory::MyNewCategory => vec!["https://example.com/feed.xml"],
```

## Technology Stack / 技术栈

| Component | Technology | Purpose |
|-----------|------------|---------|
| Language | Rust 1.75+ | Performance, safety |
| Async Runtime | Tokio | Async I/O, tasks |
| HTTP Client | reqwest + reqwest-middleware | Fetching with retry |
| RSS Parsing | feed-rs | RSS/Atom parsing |
| MCP SDK | rust-mcp-sdk | Protocol implementation |
| HN API | newswrap | Hacker News client |
| Logging | tracing + tracing-subscriber | Structured logging |
| Config | toml + serde | Configuration |
| Serialization | serde_json | JSON output |
| CLI | clap | Command-line parsing |

---

*Last updated: 2026-04-12*