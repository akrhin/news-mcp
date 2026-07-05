# AGENTS.md — инструкции для AI-ассистента

Этот файл содержит информацию, необходимую LLM (Claude Code, Hermes Agent, Cursor) для корректной работы с кодом репозитория. Агент обязан прочитать этот файл перед тем, как писать, читать или диагностировать код.

## О проекте

**News MCP Server** — личный форк [KingingWang/news-mcp](https://github.com/KingingWang/news-mcp).  
Используется в связке с Hermes Agent для мониторинга CVE/security-новостей.

Основные изменения в форке:
- **Формат `compact`** для `get_news` — одна строка на статью вместо markdown-простыни
- **`max_chars`** для `get_article_content` — обрезка длинных статей до N символов (default 2000), настраивается в `config.toml`
- **Пользовательские категории** через `[feeds.*]` в конфиге
- **Форк `feed-rs`** — `https://github.com/akrhin/feed-rs` — с обновлённым quick-xml 0.41+ (апстрим заброшен с 2024-12)

## Key code patterns

### Форматы вывода

Функции форматирования живут в `src/utils/mod.rs`:

| Функция | Назначение |
|---------|-----------|
| `format_articles_as_markdown()` | Полный markdown — для человека |
| `format_articles_as_json()` | JSON — для машин |
| `format_articles_as_text()` | Текстовый — для терминала |
| `format_articles_as_compact()` | Одна строка на статью — для LLM (контекст минимален) |

При добавлении нового формата:
1. Добавить функцию в `src/utils/mod.rs`
2. Добавить вариант в `enum_values` макроса `#[json_schema(...)]` в `src/tools/get_news.rs`
3. Добавить вариант в `build_input_schema_properties().format_prop`
4. Добавить `"compact" => dispatch` в `execute()`

### Обрезка контента (max_chars)

- Поле `max_chars` в `config::ArticleFetchConfig` (default 2000)
- Применяется на выдаче, не в кэше (полный текст всегда хранится)
- Для обоих путей: cached hit и fresh fetch
- Добавляет маркер `_[truncated — N chars; full article: M chars total]_`

### Конфигурация

- `~/.hermes/news-mcp.toml` — активный конфиг
- `config.example.toml` — полный пример
- Секция `[article_fetch]` для max_chars и fetch_timeout_secs

## Деплой

Сервер запущен как MCP-сервер через Hermes gateway:

```yaml
mcp_servers:
  news-mcp:
    command: /home/sintez/go/bin/news-mcp
    args:
    - -c
    - /home/sintez/.hermes/news-mcp.toml
    - serve
    - --log-level
    - info
    enabled: true
```

Бинарник — release-сборка в `/home/sintez/go/bin/news-mcp`. Конфиг — `~/.hermes/news-mcp.toml`. Сборка из исходников:

```bash
cd ~/git/news-mcp && cargo build --release
cp target/release/news-mcp ~/go/bin/news-mcp
systemctl --user restart hermes-gateway
```

## Связанные проекты

- **commafeed-mcp** (Python) — RSS-ридер уровня выше (Fever API, категории, статистика). news-mcp заточен на CVE/security, commafeed — на общее чтение.
- **feed-rs форк** — `akrhin/feed-rs` с quick-xml 0.41+ (транзитивная зависимость)

## Build & Test

```bash
cargo build --release              # Release build
cargo test                         # 66+ tests
cargo test --test unit             # Unit tests
cargo test --test e2e              # Integration tests
cargo fmt && cargo clippy          # Lint
```

## CI

В `.github/workflows/ci.yml`:

| Job | Имя | Что проверяет |
|-----|-----|---------------|
| check | Code Quality | fmt + clippy |
| security-audit | Security Audit | cargo audit |
| security-deny | License/Advisory Deny | cargo-deny |
| test | Test (${{ matrix.os }}) | cargo build + cargo test |
| build | Linux x64 / Linux ARM64 / macOS Intel / macOS ARM | cross-compile |
| build-windows | Build Windows | cargo build --release |
| docs | Documentation | cargo doc |
| docker | Docker Build | Dockerfile.scratch |

## Структура проекта

```
src/
├── cache/           # NewsCache + ArticleCache (RwLock<HashMap>)
│   ├── mod.rs
│   ├── news_cache.rs    # NewsCategory enum, NewsArticle struct
│   └── article_cache.rs # CachedArticle struct
├── cli/             # CLI: serve, test commands
├── config/          # TOML config parsing
├── error/           # Error types
├── service/         # NewsService (RSS), NewsNowService (JSON API)
├── tools/           # MCP tools: get_news, get_categories, get_article_content
│   ├── mod.rs
│   ├── get_news.rs
│   ├── get_categories.rs
│   └── get_article_content.rs
└── utils/           # Formatting functions, HTTP client
```

## Зависимости

- `feed-rs` — через форк `akrhin/feed-rs` (git dependency), не crates.io
- `quick-xml` — транзитивно через feed-rs, с версией 0.41+ (без известных CVE)
- `rust-mcp-sdk` — из crates.io

Если добавлять новую зависимость — проверь `cargo audit` и `cargo-deny` сразу, старые CVE уже игнорируются в `deny.toml`.

## Частые ошибки

1. **Не забыть обновить `enum_values`** в двух местах: макрос `#[json_schema]` и `build_input_schema_properties()`
2. **max_chars применяется к тексту на выдаче**, не в кэше — клонируем CachedArticle перед обрезкой
3. **feed-rs** — форк, в `Cargo.toml` должна быть `git = "https://github.com/akrhin/feed-rs"`, не версия из crates.io
