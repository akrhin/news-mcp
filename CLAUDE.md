# CLAUDE.md — инструкции для AI-ассистента

Этот файл содержит информацию, необходимую LLM (Claude Code, Hermes, Cursor) для корректной работы с кодом репозитория. Агент обязан прочитать этот файл перед тем, как писать, читать или диагностировать код.

## Build & Test Commands

```bash
# Build
cargo build
cargo build --release

# Test all
cargo test                          # 66 тестов (unit + e2e)
cargo test --test unit              # Unit tests only
cargo test --test e2e               # E2E tests only
cargo test test_name                # Single test

# Lint & Format
cargo fmt && cargo clippy

# Run (из корня репозитория)
cargo run -- serve -c config.example.toml          # С конфигом
cargo run -- serve -c ~/.hermes/news-mcp.toml      # Production config
```

## Ключевая архитектурная особенность

**Poller НЕ блокирует старт сервера.** Cache пуст при запуске. Первое заполнение — через `poller.interval_secs` (по умолчанию 3600с = 1 час).

Не спрашивай «почему пусто?» — подожди цикла опроса или проверь `get_categories` на наличие статей.

Подробная архитектура: [ARCHITECTURE.md](ARCHITECTURE.md)

## Форматы вывода (get_news)

| Формат | Для кого | Размер |
|--------|----------|--------|
| `markdown` | Человек | Многострочный, ~800 байт/статья |
| `compact` | AI / LLM | Одна строка: заголовок | источник | дата | ссылка + short description |
| `json` | Машины | Структурированный |
| `text` | Терминал | Плоский текст |

Всегда используй `compact`, когда запрашиваешь новости как AI-ассистент. Если нужна конкретная статья — дёргай `get_article_content`.

## max_chars (get_article_content)

Обрезка длинных статей. Настраивается в конфиге:
```toml
[article_fetch]
max_chars = 2000  # default
```

Полный текст всегда хранится в кэше, обрезка — только на выдаче.

## Доступные MCP-тулы (текущая версия)

- `get_news` — статьи по категории из кеша
- `get_categories` — список категорий с количеством статей
- `get_article_content` — полный текст статьи по ID

## Custom-категории из TOML-конфига

Категории из секции `[feeds.*]` в news-mcp.toml маппятся в `NewsCategory::Custom(String)`. Они отображаются в get_categories и доступны через get_news после первого цикла опроса.

## Логи

Логи пишутся в stderr (`.with_writer(std::io::stderr)` в `src/utils/mod.rs`). Stdio — только JSON-RPC. Это важно: если бинарник вдруг начнёт писать логи в stdout — Hermes сломается (ожидает JSON-RPC, получает ANSI-текст).

## Деплой

- Бинарь: `~/go/bin/news-mcp` (через `cargo build --release && cp target/release/news-mcp ~/go/bin/`)
- Конфиг: `~/.hermes/news-mcp.toml`
- После смены конфига или бинаря: `systemctl --user restart hermes-gateway`
- Проверка: `ps aux | grep news-mcp`

## Распространённые ошибки

| Симптом | Причина | Решение |
|---------|---------|---------|
| "No articles found" после старта | Poller начнёт опрос через 3600с | Подождать или проверить позже |
| keepalive failure | Бинарй пишет логи в stdout | Проверить `with_writer(std::io::stderr)` |
| `Custom(String)` не отображается | Категория ещё не опрошена | Дождаться poll-цикла |