# News MCP Server

[![Rust](https://img.shields.io/badge/rust-1.75+-orange.svg)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Fork](https://img.shields.io/badge/fork-KingingWang/news--mcp-blueviolet)](https://github.com/KingingWang/news-mcp)

> **Форк** [KingingWang/news-mcp](https://github.com/KingingWang/news-mcp) — MCP-сервер на Rust для получения новостей из RSS-лент.

**🇬🇧 English: [README.md](README.md)**

---

## Что добавляет этот форк

- **Пользовательские категории** — добавьте любую RSS-ленту как новую категорию через `config.toml`; не нужно перекомпилировать
- **Настройка через конфиг** — все URL-адреса лент живут в конфигурации, а не в коде
- **CVE и Security пресеты** — встроенные примеры для лент CISA, The Hacker News, OpenNET, Debian, Ubuntu, Red Hat
- **Динамическая схема инструментов** — `get_news` и `get_categories` автоматически обнаруживают пользовательские категории
- **Без блокировки при старте** — сервер запускается сразу, кеш заполняется на первом цикле опроса

## Возможности

- **Фоновый опрос** — сервер периодически запрашивает RSS-источники и кеширует результат
- **Режимы транспорта** — HTTP, SSE, stdio, гибридный
- **MCP-инструменты** — `get_news`, `get_categories`, `get_article_content`
- **Пользовательские RSS-ленты** — добавьте любую ленту без пересборки
- **Встроенные источники** — Technology, Science, HackerNews, 21 категория China News, 11 горячих списков NewsNow
- **Подключаемые источники** — трейт `NewsSource` для добавления собственных обработчиков
- **Кеш в памяти** — с возможностью поиска по заголовкам и описаниям
- **Механизм повторов** — при сбоях запросы автоматически повторяются

## Быстрый старт

### Сборка из исходников

```bash
git clone https://github.com/akrhin/news-mcp
cd news-mcp
cargo build --release
# Бинарник: ./target/release/news-mcp
```

### Запуск сервера

```bash
# Режим stdio (для Claude Desktop / MCP-хостов)
news-mcp serve

# HTTP режим
news-mcp serve --mode http --port 8080

# С пользовательским конфигом
news-mcp -c /path/to/config.toml serve
```

### Файл конфигурации

Создайте `config.toml` (все секции опциональны — значения по умолчанию применяются автоматически):

```toml
[server]
host = "127.0.0.1"
port = 8080
transport_mode = "stdio"  # stdio | http | sse | hybrid

[poller]
interval_secs = 3600  # опрос раз в час
enabled = true

[cache]
max_articles_per_category = 100

[logging]
level = "info"  # trace, debug, info, warn, error

# ── Пользовательские ленты ──────────────────────────
# Каждый ключ в [feeds.*] становится новой категорией.
[feeds.cisa]
display_name = "CISA Alerts"
description = "Уведомления о кибербезопасности CISA"
urls = ["https://www.cisa.gov/cybersecurity-advisories/all.xml"]
enabled = true

[feeds.redhat-security]
display_name = "Red Hat Security"
description = "Уведомления безопасности Red Hat"
urls = ["https://access.redhat.com/security/data/meta/v1/rhsa.rss"]
enabled = true
```

Полный пример: [config.example.toml](config.example.toml).

## Архитектура

```
┌──────────────┐     ┌─────────────────────────────────────┐
│   Клиент     │     │         News MCP Server             │
│ (Claude/HTTP)│────▶│  Transport → Handler → ToolRegistry │
└──────────────┘     │       ↓          ↓          ↓       │
                     │  get_news  get_categories  get_...   │
                     │       ↓                            │
                     │  ┌──────────────┐                  │
                     │  │ Кеш (RwLock) │◀── Poller        │
                     │  └──────────────┘     │            │
                     │                  ┌────┴─────┐      │
                     │                  │ NewsService│    │
                     │                  └────┬─────┘      │
                     └──────────────────────┼─────────────┘
                                            │
                    ┌───────────────────────┼───────────────────┐
                    │  RSS-ленты            │   NewsNow API     │
                    │  (TechCrunch, CISA,   │   (微博热搜, ...) │
                    │   Debian, Custom...)  │                   │
                    └───────────────────────┴───────────────────┘
```

Подробнее: [ARCHITECTURE.md](ARCHITECTURE.md).

## MCP-инструменты

### get_news

Получение статей по категории. Параметры: `category` (строка), `limit` (1–50, по умолч. 10), `format` (markdown|json|text).

```json
{"category": "technology", "limit": 5, "format": "markdown"}
```

### get_categories

Список всех доступных категорий (встроенные + пользовательские) с количеством статей.

### get_article_content

Полный текст статьи по ID. Работает только для RSS-источников, не для горячих списков.

**Параметры:** `id` (строка), `format` (markdown|json|text).

## Категории

### Встроенные международные

| Категория | Источники |
|-----------|-----------|
| `technology` | TechCrunch, Ars Technica, The Verge |
| `science` | ScienceDaily |
| `hackernews` | Hacker News |

### Встроенные China News (21 лента)

`instant`, `headlines`, `politics`, `eastwest`, `society`, `finance`, `life`, `wellness`, `greaterbayarea`, `chinese`, `video`, `photo`, `creative`, `live`, `education`, `law`, `unitedfront`, `ethnicunity`, `theory`, `asean`

### Встроенные горячие списки (NewsNow)

`weibohot`, `baiduhot`, `zhihuhot`, `douyinhot`, `bilibilihot`, `tiebahot`, `toutiaohot`, `wallstreetcnhot`, `clshot`, `thepaperhot`, `ifenghot`

### Пользовательские категории

Любой ключ, добавленный в секцию `[feeds.*]` файла `config.toml`, автоматически становится доступным как категория. Перекомпиляция не требуется.

## Интеграция с Claude Desktop

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

## Переменные окружения

| Переменная | По умолчанию | Описание |
|-----------|-------------|----------|
| `NEWS_MCP_PORT` | 8080 | Порт сервера |
| `NEWS_MCP_HOST` | 127.0.0.1 | Хост сервера |
| `NEWS_MCP_TRANSPORT` | stdio | Режим транспорта |
| `NEWS_MCP_INTERVAL` | 3600 | Интервал опроса (секунды) |
| `NEWS_MCP_LOG_LEVEL` | info | Уровень логирования |

## Разработка

```bash
cargo test              # Все тесты (66+)
cargo test --test unit  # Модульные тесты
cargo fmt && cargo clippy
```

## Документация

- [Архитектура](ARCHITECTURE.md) — обзор компонентов и настройка
- [config.example.toml](config.example.toml) — полный пример конфига с CVE-лентами
- [README.md](README.md) — English version

## Лицензия

MIT — см. [LICENSE](LICENSE).

## Благодарности

- [KingingWang/news-mcp](https://github.com/KingingWang/news-mcp) — оригинальный проект
- [rust-mcp-sdk](https://github.com/rust-mcp-stack/rust-mcp-sdk) — MCP SDK
- [feed-rs](https://github.com/feed-rs/feed-rs) — парсинг RSS/Atom
- [tokio](https://tokio.rs) — асинхронный рантайм
