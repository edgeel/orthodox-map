# Карта изучения православия: генератор сайта `omap`.

repo := "edgeel/orthodox-map"

# Показать рецепты
default:
    @just --list

# Тесты: Rust и JavaScript (QuickJS через uv)
test:
    cargo test
    uv run -q tools/jscheck.py

# Формат и clippy без предупреждений
lint:
    cargo fmt --check
    cargo clippy --all-targets -- -D warnings

# Проверить содержание (по умолчанию content/; можно указать другую папку)
check content="content":
    cargo run -q -- check --content {{content}}

# Собрать сайт в dist/
build content="content":
    cargo run -q --release -- build --content {{content}}

# Весь сайт одним HTML-файлом: открывается с диска, без сети; JS проверяется в QuickJS
single file="dist/orthodox-map.html" content="content":
    cargo run -q --release -- build --content {{content}} --single {{file}}
    uv run -q tools/jscheck.py {{file}}

# Собрать и показать на http://127.0.0.1:8000/
serve content="content":
    cargo run -q -- serve --content {{content}}

# Установить omap в ~/.cargo/bin
install:
    cargo install --path . --locked

# Выложить сайт: проверки, затем push в main; GitHub Actions соберёт и опубликует Pages
deploy:
    #!/usr/bin/env bash
    set -euo pipefail
    if [ -n "$(git status --porcelain)" ]; then
        echo "есть незакоммиченные изменения:"; git status --short; exit 1
    fi
    git pull --ff-only
    just test lint
    cargo run -q -- check
    git push origin main
    echo "отправлено; сборка Pages: gh run list -R {{repo}} -w pages.yml"

# Один раз: создать публичный репозиторий на GitHub, запушить и включить Pages
publish:
    gh repo create {{repo}} --public --source . --push \
        --description "Карта изучения православия: интерактивное древо тем с книгами, лекциями и видео"
    gh api -X POST repos/{{repo}}/pages -f build_type=workflow \
        || gh api -X PUT repos/{{repo}}/pages -f build_type=workflow
    gh repo edit {{repo}} --homepage https://edgeel.github.io/orthodox-map/
    @echo "Pages включены; сайт появится после сборки: gh run list -R {{repo}}"

# Версия собранного omap
version:
    cargo run -q -- --version
