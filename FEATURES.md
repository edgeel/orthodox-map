# Features

Status: ✅ done · 🚧 in progress · 📋 planned · 💡 idea

## CONTENT — schema and checks
- ✅ CONTENT-1 sections.toml, routes.toml, one Markdown file per topic with YAML frontmatter
- ✅ CONTENT-2 `omap check`: ids, folder/file names, levels, `needs` (missing, self, cycles), materials, routes; Russian messages with file and fix
- ✅ CONTENT-3 agrees with the content repo's check.py (missing body link = warning, `_` fields ignored)
- 📋 CONTENT-4 switch `content/` to the real content from orthodox-map-content
- 💡 CONTENT-5 link checker (`omap links`): HTTP status of every material URL

## MAP — the tree page
- ✅ MAP-1 deterministic layout: sections as columns by order, levels bottom-up, edges for `needs`, roots and trunk
- ✅ MAP-2 SVG map with pan and zoom (mouse, trackpad, touch pinch), keyboard navigation
- ✅ MAP-3 topic card: summary, «с чего начать», materials with type, author, time, language; needs and next
- ✅ MAP-4 search (ё = е), filters by level and material type, section legend
- ✅ MAP-5 routes: highlight a route's topics in order, route pages
- ✅ MAP-6 list view (switch next to the search); works without JavaScript via topic links
- ✅ MAP-7 progress (want / reading / done) in localStorage; marks on nodes and in the list; JSON export/import (older saves load)
- ✅ MAP-8 filter «Мои отметки»: only topics I want to study / am reading / have done / haven't marked
- ✅ MAP-9 phones: the map opens by default; one-row header with filters folded; card as a bottom sheet (pull to close); start view fits level 1 and the roots

## SITE — pages and publishing
- ✅ SITE-1 topic and route pages, 404, sitemap.xml, robots.txt, canonical URLs from site.toml
- ✅ SITE-2 light/dark theme (system + toggle), no external CDN, fonts or trackers
- ✅ SITE-3 PWA: manifest, service worker, offline after the first visit; hashed asset URLs
- ✅ SITE-4 print CSS: the list with material URLs
- ✅ SITE-5 GitHub Pages workflow (build on push to main), `just publish`, `just deploy`
- ✅ SITE-6 single-file HTML export: `omap build --single FILE`, `just single`, also `dist/orthodox-map.html` with a download link in the menu; works from `file://`, no requests
- 💡 SITE-7 Telegram Mini App

## DEV — tooling
- ✅ DEV-1 `omap --version` with git SHA and build time
- ✅ DEV-2 local preview server `omap serve` (strips the Pages base path)
- 🚧 DEV-3 screenshots with Playwright (tools/screenshots.py; needs Chromium system libraries)
- 💡 DEV-4 `omap serve --watch`: rebuild on content changes
- ✅ DEV-5 JavaScript checks in QuickJS (`tools/jscheck.py`, part of `just test`)
