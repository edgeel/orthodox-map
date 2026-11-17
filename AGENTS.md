# AGENTS.md — orthodox-map

Static site generator for «Карта изучения православия» (Orthodox study map). Owner: Bogdan (@edgeel).
User-facing text (site, CLI messages, README) is Russian; code, comments and this file are English.

## Layout

- `src/model.rs` — content model (sections, routes, topics with YAML frontmatter), `load()` collects every
  problem as a `Diag` instead of stopping at the first.
- `src/validate.rs` — ids, references, folder/file name, levels, `needs` cycles, materials, body links.
  Errors stop the build, warnings don't. Messages are Russian and name the file and the fix.
- `src/markdown.rs` — topic body → HTML (pulldown-cmark): topic links `[t](id)` / `[[id]]`, `## Дальше`
  collected as `next`, raw HTML escaped, only http(s) external links.
- `src/layout.rs` — deterministic tree layout (sections = columns by `order`, level 1 at the bottom);
  the browser only draws what is computed here.
- `src/render.rs` — minijinja templates → `dist/`: `index.html` (SVG map + list), `topic/<id>/`,
  `route/<id>/`, `data.json`, `404.html`, `sitemap.xml`, manifest, service worker. Asset URLs carry a content hash.
  `single()` renders the same `index.html` with `single = true`: CSS, JS, icon (data URI) and data.json
  (`#omap-data`, `<` escaped) inline, `hash_links` turns `topic/<id>/` and `route/<id>/` into hashes; `build`
  also writes it as `dist/orthodox-map.html`.
- `src/serve.rs` — tiny local preview server (`tiny_http`), strips the GitHub Pages base path.
- `src/main.rs` — CLI `omap check|build|serve`, `--version` with git SHA and build time (`build.rs`).
- `templates/`, `assets/` — embedded at compile time (`include_str!`); no external CDN, fonts or trackers.
- `content/` — demo content in the schema; will be replaced by the real content
  (`../orthodox-map-content`). The schema is shared with that repo's `check.py`
  (spec: `../orthodox-map-tools/SPEC.md`); keep both in agreement: anything `check.py` accepts must pass
  `omap check` (omap may warn more, never fail more).
- `site.toml` — site title, `base_url`, labels; not part of `content/`.
- `tools/screenshots.py` — Playwright screenshots (needs Chromium system libraries; optional).
- `tools/jscheck.py` — QuickJS (PyPI, via `uv run`): compiles `app.js`/`sw.js`, runs the `// pure:start …
  // pure:end` block of `app.js` (progress states) against old and new saves, and checks a single file's inline
  script and data. Part of `just test`.

## Commands

`just` lists recipes: `test` (cargo test + jscheck), `lint` (fmt + clippy `-D warnings`), `check`, `build`,
`single` (one HTML file), `serve`, `install`,
`deploy` (checks, then push to main; Pages builds in Actions), `publish` (one-off: create the GitHub repo and
enable Pages), `version`.

## Conventions

- Rust 2024, pure-Rust crates only (no `-sys`, no openssl). Check new deps with `cargo tree`.
- `FEATURES.md` is the only status tracker (area IDs like `MAP-3`); `CHANGELOG.md` gets an entry for every
  user-visible change under `## Unreleased`; landing on main bumps the version in `Cargo.toml`.
- Tests: unit tests next to the code, end-to-end in `tests/site.rs` (temp content → validate → layout →
  render). Run `cargo test && git commit …` so a red test stops the commit; `just lint` before landing.
- Small commits with a clear message; work on a branch, land on `main`.
- Phones (`max-width: 700px`): the map opens by default like everywhere (Bogdan, 16 Nov 2026), the header is one
  row with filters folded behind `#filters-btn`, the card is a bottom sheet (`#sheet-grip` pulls it closed),
  `start()` fits the level-1 band and roots into the height. Print CSS prints the list with material URLs.
- Progress: `omap.progress.v1` in localStorage is `{ topicId: "want" | "reading" | "done" }`; `cleanProgress`
  drops anything else, so older saves and export files load unchanged. Keep new states in `STATES`.
- The single file must not reference local files and must not fetch: `tests/site.rs` checks every `href`.
  app.js must never contain `</script`.
