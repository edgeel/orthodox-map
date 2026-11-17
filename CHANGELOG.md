# Changelog

## Unreleased

- Third progress state «🔖 Хочу изучить» next to «📖 Читаю» and «✓ Изучил»: a button on the card, a dashed
  frame on the map node, a mark in the list. Stored in the same `omap.progress.v1` map; older saves and
  exported files load unchanged, import accepts the new state.
- Filter «Мои отметки»: show only topics I want to study, am reading, have done, or haven't marked.
- Phones open the map by default (the list is one tap away). One-row header: search, a filters button (with a
  dot when a filter is on), map/list icons, menu; levels, types, marks and routes fold behind the button. The card
  is a bottom sheet with a grip to pull it closed; the map re-centres a topic above the sheet. The start view
  fits the level-1 band and the roots into the height on any screen.
- Single-file export: `omap build --single FILE` and `just single` write the whole site into one HTML file
  (data, styles, script and icon inline; topic and route links become `#topic/<id>`, `#route/<id>`; no service
  worker, no requests). `omap build` also writes it as `dist/orthodox-map.html`, offered in the menu as
  «Скачать карту одним файлом». In the single file «🔗 Ссылка на тему» points at the public site.
- `just test` also checks the JavaScript in QuickJS (`tools/jscheck.py`, via uv).

## 0.1.0 — 16 Nov 2026

First version.

- `omap check`, `omap build`, `omap serve`: content schema (sections, routes, Markdown topics with YAML
  frontmatter), validation with Russian messages, deterministic bottom-up tree layout.
- Site: SVG tree map with pan/zoom, topic cards, search, level and material-type filters, routes, list view
  (default on phones), progress in localStorage, light/dark theme, PWA offline, print CSS, topic and route pages,
  404, sitemap.
- Agrees with the content repo's check.py: a body link to a topic that doesn't exist yet is a warning
  (rendered as plain text); frontmatter keys starting with `_` are ignored.
- Demo content: 11 sections, 15 topics, 3 routes.
- GitHub Pages workflow, justfile (`test`, `lint`, `check`, `build`, `serve`, `install`, `deploy`, `publish`).
