# News MCP Server

[![Rust](https://img.shields.io/badge/rust-1.75+-orange.svg)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Fork](https://img.shields.io/badge/fork-KingingWang/news--mcp-blueviolet)](https://github.com/KingingWang/news-mcp)

> **Fork** of [KingingWang/news-mcp](https://github.com/KingingWang/news-mcp) — a Rust-based MCP server for fetching news from RSS feeds.

**🇷🇺 Русская версия: [README_RU.md](README_RU.md)**

---

## What This Fork Adds

- **Custom categories** — add any RSS feed as a new category via `config.toml`; no code changes needed
- **Config-driven sources** — all feed URLs live in config, not in enum variants
- **CVE & security presets** — built-in example configs for CISA, The Hacker News, OpenNET, Debian, Ubuntu, Red Hat security feeds
- **Dynamic tool schema** — `get_news` and `get_categories` auto-discover custom categories from cache
- **No startup blocking** — server starts immediately; cache fills on first poll tick

## Features

- **Background Polling** — periodically fetches news from RSS sources and caches locally
- **Multiple Transport Modes** — HTTP, SSE, stdio, hybrid
- **MCP Tools** — `get_news`, `get_categories`, `get_article_content`
- **Custom RSS Feeds** — add any feed without recompiling
- **Built-in Sources** — Technology, Science, HackerNews, 21 China News categories, 11 NewsNow hot lists
- **Pluggable Sources** — extensible `NewsSource` trait
- **In-memory Cache** — high-performance article cache with search
- **Retry Mechanism** — automatic retry for failed fetch requests

## Quick Start

### Build from Source

```bash
git clone https://github.com/akrhin/news-mcp
cd news-mcp
cargo build --release
# Binary: ./target/release/news-mcp
```

### Run Server

```bash
# stdio mode (for Claude Desktop / MCP hosts)
news-mcp serve

# HTTP mode
news-mcp serve --mode http --port 8080

# With custom config
news-mcp -c /path/to/config.toml serve
```

### Configuration File

Create `config.toml` (all sections optional — defaults apply when omitted):

```toml
[server]
host = "127.0.0.1"
port = 8080
transport_mode = "stdio"  # stdio | http | sse | hybrid

[poller]
interval_secs = 3600  # poll every hour
enabled = true

[cache]
max_articles_per_category = 100

[logging]
level = "info"  # trace, debug, info, warn, error

# ── Custom Feeds ─────────────────────────────────────
# Any key under [feeds.*] becomes a new category.
[feeds.cisa]
display_name = "CISA Alerts"
description = "CISA cybersecurity alerts"
urls = ["https://www.cisa.gov/cybersecurity-advisories/all.xml"]
enabled = true

[feeds.thehackernews]
display_name = "The Hacker News"
description = "Cybersecurity news"
urls = ["https://feeds.feedburner.com/TheHackersNews"]
enabled = true

[feeds.redhat-security]
display_name = "Red Hat Security"
description = "Red Hat security advisories"
urls = ["https://access.redhat.com/security/data/meta/v1/rhsa.rss"]
enabled = true
```

See [config.example.toml](config.example.toml) for the full reference.

## Architecture

```
┌──────────────┐     ┌─────────────────────────────────────┐
│   Client     │     │         News MCP Server             │
│ (Claude/HTTP)│────▶│  Transport → Handler → ToolRegistry │
└──────────────┘     │       ↓          ↓          ↓       │
                     │  get_news  get_categories  get_...   │
                     │       ↓                            │
                     │  ┌──────────────┐                  │
                     │  │ Cache (RwLock)│◀── Poller       │
                     │  └──────────────┘     │            │
                     │                  ┌────┴─────┐      │
                     │                  │ NewsService│    │
                     │                  └────┬─────┘      │
                     └──────────────────────┼─────────────┘
                                            │
                    ┌───────────────────────┼───────────────────┐
                    │  RSS Feeds            │   NewsNow API     │
                    │  (TechCrunch, CISA,   │   (微博热搜, ...) │
                    │   Debian, Custom...)  │                   │
                    └───────────────────────┴───────────────────┘
```

## MCP Tools

### get_news

Fetch articles by category. Parameters: `category` (string), `limit` (1–50, default 10), `format` (markdown|json|text).

```json
{"category": "technology", "limit": 5, "format": "markdown"}
```

### get_categories

List all available categories (built-in + custom) with article counts.

### get_article_content

Fetch full article content by article ID. Only works for RSS-based sources, not for hot search / trending topics.

**Parameters:** `id` (string), `format` (markdown|json|text).

## Categories

### Built-in International

| Category | Sources |
|----------|---------|
| `technology` | TechCrunch, Ars Technica, The Verge |
| `science` | ScienceDaily |
| `hackernews` | Hacker News |

### Built-in China News (21 feeds)

`instant`, `headlines`, `politics`, `eastwest`, `society`, `finance`, `life`, `wellness`, `greaterbayarea`, `chinese`, `video`, `photo`, `creative`, `live`, `education`, `law`, `unitedfront`, `ethnicunity`, `theory`, `asean`

### Built-in Hot Lists (NewsNow)

`weibohot`, `baiduhot`, `zhihuhot`, `douyinhot`, `bilibilihot`, `tiebahot`, `toutiaohot`, `wallstreetcnhot`, `clshot`, `thepaperhot`, `ifenghot`

### Custom Categories

Any key added under `[feeds.*]` in `config.toml` automatically appears as a new category. No recompile needed.

## Claude Desktop Integration

```json
{
  "mcpServers": {
    "news": {
      "command": "/path/to/news-mcp",
      "args": ["-c", "/path/to/config.toml", "serve"]
    }
  }
}
```

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `NEWS_MCP_PORT` | 8080 | Server port |
| `NEWS_MCP_HOST` | 127.0.0.1 | Server host |
| `NEWS_MCP_TRANSPORT` | stdio | Transport mode |
| `NEWS_MCP_INTERVAL` | 3600 | Polling interval (seconds) |
| `NEWS_MCP_LOG_LEVEL` | info | Log level |

## Development

```bash
cargo test              # All tests (66+)
cargo test --test unit  # Unit tests
cargo fmt && cargo clippy
```

## Documentation

- [Architecture](ARCHITECTURE.md) — component overview and configuration guide
- [config.example.toml](config.example.toml) — full config reference with CVE feed presets
- [README_RU.md](README_RU.md) — русская версия

## License

MIT — see [LICENSE](LICENSE).

## Acknowledgments

- [KingingWang/news-mcp](https://github.com/KingingWang/news-mcp) — original project
- [rust-mcp-sdk](https://github.com/rust-mcp-stack/rust-mcp-sdk) — MCP SDK
- [feed-rs](https://github.com/feed-rs/feed-rs) — RSS/Atom parsing
- [tokio](https://tokio.rs) — async runtime
