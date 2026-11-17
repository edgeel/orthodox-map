// Карта изучения православия: drawing only. The layout comes from the generator (data.json + inline SVG).
// No libraries, no build step, no requests other than this site's own files. In the single-file
// export (`body[data-single]`) data.json is inline in #omap-data and nothing is fetched at all.
(() => {
  "use strict";
  document.documentElement.classList.add("js");
  const $ = (s, el = document) => el.querySelector(s);
  const $$ = (s, el = document) => Array.from(el.querySelectorAll(s));
  const body = document.body;
  const root = body.dataset.root || "./";
  const single = "single" in body.dataset;
  function setHash(h) { try { history.replaceState(null, "", h); } catch (e) { /* file:// in some browsers */ } }
  const store = {
    get(k, d) { try { const v = localStorage.getItem(k); return v === null ? d : JSON.parse(v); } catch (e) { return d; } },
    set(k, v) { try { localStorage.setItem(k, JSON.stringify(v)); } catch (e) { /* private mode */ } },
  };

  // ── Theme ────────────────────────────────────────────
  const THEMES = ["", "light", "dark"];
  const THEME_NAMES = { "": "как в системе", light: "светлая", dark: "тёмная" };
  function applyTheme(t) {
    if (t) document.documentElement.dataset.theme = t; else delete document.documentElement.dataset.theme;
    try { if (t) localStorage.setItem("omap.theme", t); else localStorage.removeItem("omap.theme"); } catch (e) { /* */ }
    const n = $("#theme-name"); if (n) n.textContent = THEME_NAMES[t];
  }
  applyTheme(document.documentElement.dataset.theme || "");
  $("#theme-btn")?.addEventListener("click", () => {
    const cur = document.documentElement.dataset.theme || "";
    applyTheme(THEMES[(THEMES.indexOf(cur) + 1) % THEMES.length]);
  });

  // ── Progress states (pure: tools/jscheck.py runs this block in QuickJS) ──
  // pure:start
  // One map { topicId: state } under omap.progress.v1; "want" came later, older saves simply lack it.
  const STATES = ["want", "reading", "done"];
  function cleanProgress(p) {
    const out = {};
    if (p && typeof p === "object" && !Array.isArray(p)) {
      for (const [id, st] of Object.entries(p)) if (STATES.includes(st)) out[id] = st;
    }
    return out;
  }
  // `mine` is the "Мои отметки" filter: "" all, a state, or "none" for topics without a mark.
  function statusMatches(st, mine) { return !mine || (mine === "none" ? !st : st === mine); }
  // pure:end

  // ── Progress (localStorage) ──────────────────────────
  const PKEY = "omap.progress.v1";
  let progress = cleanProgress(store.get(PKEY, {}));
  let onProgress = null; // the map page refilters when "Мои отметки" is on
  function setStatus(id, st) {
    if (!st || progress[id] === st) delete progress[id]; else progress[id] = st;
    store.set(PKEY, progress);
    paintProgress();
  }
  function paintProgress() {
    $$("[data-id]").forEach((el) => {
      if (!el.classList.contains("node") && !el.classList.contains("li")) return;
      const st = progress[el.dataset.id];
      el.classList.toggle("st-want", st === "want");
      el.classList.toggle("st-reading", st === "reading");
      el.classList.toggle("st-done", st === "done");
    });
    $$(".status[data-id]").forEach((g) => {
      const st = progress[g.dataset.id];
      $$("button[data-st]", g).forEach((b) => b.setAttribute("aria-pressed", String(b.dataset.st === st)));
    });
    // per-section share of "изучил"
    const per = {};
    $$(".li[data-id]").forEach((li) => {
      const sec = li.closest("[data-section]")?.dataset.section;
      if (!sec) return;
      per[sec] = per[sec] || { done: 0, all: 0 };
      per[sec].all++;
      if (progress[li.dataset.id] === "done") per[sec].done++;
    });
    $$(".col[data-section]").forEach((g) => {
      const p = per[g.dataset.section] || { done: 0, all: 0 };
      const bar = $(".col-prog", g), cnt = $(".col-count", g);
      if (bar) bar.setAttribute("width", p.all ? (bar.dataset.full * p.done / p.all).toFixed(1) : "0");
      if (cnt) cnt.textContent = `${p.done}/${p.all}`;
    });
    $$(".sec-prog[data-section]").forEach((s) => {
      const p = per[s.dataset.section] || { done: 0, all: 0 };
      s.textContent = `${p.done}/${p.all}`;
      s.title = "Изучено тем";
    });
    if (onProgress) onProgress();
  }
  document.addEventListener("click", (e) => {
    const b = e.target.closest(".status[data-id] button[data-st]");
    if (!b) return;
    setStatus(b.closest(".status").dataset.id, b.dataset.st);
  });
  $("#export-btn")?.addEventListener("click", () => {
    const blob = new Blob([JSON.stringify({ app: "orthodox-map", version: 1, saved: new Date().toISOString(), progress }, null, 2)], { type: "application/json" });
    const a = document.createElement("a");
    a.href = URL.createObjectURL(blob);
    a.download = `orthodox-map-progress-${new Date().toISOString().slice(0, 10)}.json`;
    document.body.appendChild(a); a.click(); a.remove();
    setTimeout(() => URL.revokeObjectURL(a.href), 1000);
  });
  $("#import-file")?.addEventListener("change", async (e) => {
    const f = e.target.files[0]; if (!f) return;
    try {
      const j = JSON.parse(await f.text());
      const p = cleanProgress(j.progress || j);
      const n = Object.keys(p).length;
      Object.assign(progress, p);
      store.set(PKEY, progress); paintProgress();
      alert(`Загружено отметок: ${n}`);
    } catch (err) { alert("Не удалось прочитать файл прогресса: " + err.message); }
    e.target.value = "";
  });
  $("#reset-btn")?.addEventListener("click", () => {
    if (confirm("Сбросить все отметки «хочу изучить», «читаю» и «изучил» в этом браузере?")) { progress = {}; store.set(PKEY, progress); paintProgress(); }
  });
  paintProgress();
  window.addEventListener("storage", (e) => { if (e.key === PKEY) { progress = cleanProgress(store.get(PKEY, {})); paintProgress(); } });

  // ── Share a topic link ───────────────────────────────
  document.addEventListener("click", async (e) => {
    const a = e.target.closest("a[data-share]");
    if (!a || e.ctrlKey || e.metaKey || e.shiftKey || e.button !== 0) return;
    e.preventDefault();
    const url = new URL(a.getAttribute("href"), location.href).href;
    const title = $(".card-title", a.closest(".card"))?.textContent || document.title;
    if (navigator.share) { try { await navigator.share({ title, url }); } catch (err) { /* cancelled */ } return; }
    try { await navigator.clipboard.writeText(url); a.textContent = "✓ Ссылка скопирована"; } catch (err) { location.href = url; }
  });

  // ── Offline (PWA) ────────────────────────────────────
  if (!single && "serviceWorker" in navigator && (location.protocol === "https:" || location.hostname === "localhost" || location.hostname === "127.0.0.1")) {
    window.addEventListener("load", () => navigator.serviceWorker.register(root + "sw.js").catch(() => {}));
  }

  if (!body.classList.contains("page-map")) return;

  // ════════════════ Map page ════════════════
  const svg = $("#map"), vp = $("#vp"), wrap = $("#mapwrap");
  const panel = $("#panel"), panelBody = $("#panel-body");
  const top = $("#top"), filtersBtn = $("#filters-btn");
  const W = +svg.dataset.w, H = +svg.dataset.h;
  const nodes = new Map($$(".node", svg).map((n) => [n.dataset.id, n]));
  const edges = $$(".edge", svg);
  const lis = new Map($$(".li[data-id]").map((n) => [n.dataset.id, n]));
  let data = null;
  const topics = new Map();
  const inline = $("#omap-data");
  const ready = (inline
    ? Promise.resolve().then(() => JSON.parse(inline.textContent))
    : fetch(root + "data.json?v=" + body.dataset.version).then((r) => r.json()))
    .then((j) => { data = j; j.topics.forEach((t) => topics.set(t.id, t)); })
    .catch(() => { /* cards then fall back to topic pages */ });

  // ── Pan & zoom ───────────────────────────────────────
  const view = { k: 1, x: 0, y: 0 };
  const K_MIN = 0.15, K_MAX = 2.5;
  function apply() { vp.setAttribute("transform", `translate(${view.x.toFixed(1)},${view.y.toFixed(1)}) scale(${view.k.toFixed(4)})`); }
  function size() { return { w: wrap.clientWidth, h: wrap.clientHeight }; }
  function resize() { const s = size(); svg.setAttribute("viewBox", `0 0 ${s.w} ${s.h}`); svg.removeAttribute("width"); svg.removeAttribute("height"); }
  function clampView() {
    const s = size(), m = 80;
    view.x = Math.min(s.w - m, Math.max(m - W * view.k, view.x));
    view.y = Math.min(s.h - m, Math.max(m - H * view.k, view.y));
  }
  function zoomAt(px, py, k) {
    k = Math.min(K_MAX, Math.max(K_MIN, k));
    view.x = px - (px - view.x) * (k / view.k);
    view.y = py - (py - view.y) * (k / view.k);
    view.k = k; clampView(); apply();
  }
  function fitBox(b, maxK = 1.1, pad = 40) {
    const s = size();
    const panelW = panel.hidden || s.w < 700 ? 0 : panel.offsetWidth + 24;
    const aw = s.w - panelW, ah = s.h;
    const k = Math.min(maxK, Math.max(K_MIN, Math.min((aw - 2 * pad) / b.w, (ah - 2 * pad) / b.h)));
    view.k = k;
    view.x = (aw - b.w * k) / 2 - b.x * k;
    view.y = (ah - b.h * k) / 2 - b.y * k;
    clampView(); apply();
  }
  function fitAll() { fitBox({ x: 0, y: 0, w: W, h: H }); }
  function start() {
    // Readable from the first look: start at the roots, beginning on the left (основы веры).
    // The level-1 band and the roots under it always fit the height, also on a phone.
    const s = size();
    const fitK = Math.min(s.w / W, s.h / H);
    if (fitK >= 0.62) { fitAll(); return; }
    const band1 = $(".band-1", svg);
    const rootsH = H - (band1 ? +band1.getAttribute("y") : H * 0.38) + 16;
    view.k = Math.max(fitK, K_MIN, Math.min(0.8, (s.h - 16) / rootsH));
    view.x = 16;
    view.y = s.h - H * view.k - 8;
    clampView(); apply();
  }
  function centerOn(id) {
    const n = nodes.get(id); if (!n) return;
    const r = $(".node-bg", n), s = size();
    const x = +r.getAttribute("x") + 100, y = +r.getAttribute("y") + 35;
    const panelW = panel.hidden || s.w < 700 ? 0 : panel.offsetWidth + 24;
    const sx = view.x + x * view.k, sy = view.y + y * view.k;
    // On a phone the card is a sheet over the bottom: aim at what stays visible above it.
    const visibleH = s.w < 700 && !panel.hidden ? Math.max(120, s.h - panel.offsetHeight) : s.h;
    if (sx < 40 || sx > s.w - panelW - 40 || sy < 40 || sy > visibleH - 40 || view.k < 0.5) {
      if (view.k < 0.5) view.k = 0.75;
      view.x = (s.w - panelW) / 2 - x * view.k;
      view.y = visibleH / 2 - y * view.k;
      clampView(); apply();
    }
  }

  wrap.addEventListener("wheel", (e) => {
    e.preventDefault();
    const r = wrap.getBoundingClientRect();
    const trackpadPan = !e.ctrlKey && e.deltaMode === 0 && (Math.abs(e.deltaX) > 0.5 || !Number.isInteger(e.deltaY));
    if (trackpadPan) { view.x -= e.deltaX; view.y -= e.deltaY; clampView(); apply(); return; }
    const dy = e.deltaMode === 1 ? e.deltaY * 16 : e.deltaY;
    zoomAt(e.clientX - r.left, e.clientY - r.top, view.k * Math.exp(-dy * (e.ctrlKey ? 0.01 : 0.0015)));
  }, { passive: false });

  const ptrs = new Map();
  let drag = null, moved = 0, suppressClick = false;
  wrap.addEventListener("pointerdown", (e) => {
    if (e.target.closest(".zoom, .hint, .routebar, .panel")) return;
    ptrs.set(e.pointerId, { x: e.clientX, y: e.clientY });
    moved = 0; suppressClick = false;
    drag = { x: view.x, y: view.y, k: view.k, pts: new Map(ptrs) };
    if (ptrs.size === 2) drag.dist = pinchDist();
  });
  function pinchDist() { const [a, b] = [...ptrs.values()]; return Math.hypot(a.x - b.x, a.y - b.y); }
  wrap.addEventListener("pointermove", (e) => {
    if (!ptrs.has(e.pointerId) || !drag) return;
    const prev = ptrs.get(e.pointerId);
    ptrs.set(e.pointerId, { x: e.clientX, y: e.clientY });
    moved += Math.abs(e.clientX - prev.x) + Math.abs(e.clientY - prev.y);
    if (moved > 6 && !wrap.classList.contains("dragging")) {
      wrap.classList.add("dragging");
      try { wrap.setPointerCapture(e.pointerId); } catch (err) { /* */ }
    }
    if (ptrs.size === 1) {
      const d0 = drag.pts.get(e.pointerId); if (!d0) return;
      view.x = drag.x + (e.clientX - d0.x); view.y = drag.y + (e.clientY - d0.y); clampView(); apply();
    } else if (ptrs.size === 2 && drag.dist) {
      const r = wrap.getBoundingClientRect();
      const [a, b] = [...ptrs.values()];
      const mx = (a.x + b.x) / 2 - r.left, my = (a.y + b.y) / 2 - r.top;
      zoomAt(mx, my, drag.k * pinchDist() / drag.dist);
      drag.k = view.k; drag.dist = pinchDist();
    }
  });
  function endPtr(e) {
    ptrs.delete(e.pointerId);
    if (moved > 6) suppressClick = true;
    wrap.classList.remove("dragging");
    drag = ptrs.size ? { x: view.x, y: view.y, k: view.k, pts: new Map(ptrs) } : null;
  }
  wrap.addEventListener("pointerup", endPtr);
  wrap.addEventListener("pointercancel", endPtr);

  $("#zoom-in").addEventListener("click", () => { const s = size(); zoomAt(s.w / 2, s.h / 2, view.k * 1.3); });
  $("#zoom-out").addEventListener("click", () => { const s = size(); zoomAt(s.w / 2, s.h / 2, view.k / 1.3); });
  $("#zoom-fit").addEventListener("click", fitAll);

  // ── Topic card ───────────────────────────────────────
  let selected = null;
  function ancestors(id, acc = new Set()) {
    const t = topics.get(id);
    (t ? t.needs : []).forEach((n) => { if (!acc.has(n)) { acc.add(n); ancestors(n, acc); } });
    return acc;
  }
  function highlight(id) {
    nodes.forEach((n) => n.classList.toggle("sel", n.dataset.id === id));
    const chain = id ? ancestors(id) : new Set();
    if (id) chain.add(id);
    edges.forEach((e) => {
      const on = id && chain.has(e.dataset.to) && chain.has(e.dataset.from);
      const out = id && e.dataset.from === id;
      e.classList.toggle("hl", Boolean(on || out));
    });
  }
  async function openTopic(id, opts = {}) {
    await ready;
    const t = topics.get(id);
    if (!t) { if (nodes.has(id) || lis.has(id)) location.href = `${root}topic/${id}/`; return; }
    selected = id;
    panelBody.innerHTML = t.card;
    panel.hidden = false;
    paintProgress();
    highlight(id);
    if (body.classList.contains("view-map") && opts.pan !== false) centerOn(id);
    if (opts.hash !== false) setHash("#topic/" + id);
    if (opts.focus) panel.focus({ preventScroll: true });
    panel.scrollTop = 0;
  }
  function closePanel() {
    if (panel.hidden) return;
    panel.hidden = true;
    panel.style.transform = "";
    const prev = selected; selected = null;
    highlight(null);
    setHash(location.pathname + location.search + (route ? "#route/" + route : ""));
    if (prev && body.classList.contains("view-map")) nodes.get(prev)?.focus({ preventScroll: true });
  }
  $("#panel-close").addEventListener("click", closePanel);

  // Phone: pull the sheet down by its grip to close it.
  const grip = $("#sheet-grip");
  let pull = null;
  grip.addEventListener("pointerdown", (e) => { pull = { y: e.clientY, dy: 0 }; try { grip.setPointerCapture(e.pointerId); } catch (err) { /* */ } });
  grip.addEventListener("pointermove", (e) => {
    if (!pull) return;
    pull.dy = Math.max(0, e.clientY - pull.y);
    panel.style.transform = `translateY(${pull.dy}px)`;
  });
  function endPull() {
    if (!pull) return;
    const far = pull.dy > 70; pull = null;
    if (far) closePanel(); else panel.style.transform = "";
  }
  grip.addEventListener("pointerup", endPull);
  grip.addEventListener("pointercancel", endPull);

  svg.addEventListener("click", (e) => {
    const n = e.target.closest(".node");
    if (!n) { if (!suppressClick && !panel.hidden && e.target.closest("svg")) closePanel(); return; }
    if (e.ctrlKey || e.metaKey || e.shiftKey) return; // open the topic page in a new tab
    e.preventDefault();
    if (suppressClick) return;
    openTopic(n.dataset.id, { focus: e.detail === 0 });
  });
  // Links to topics inside the card and the list stay on this page.
  document.addEventListener("click", (e) => {
    const a = e.target.closest("a[data-topic]");
    if (a && !e.ctrlKey && !e.metaKey && !e.shiftKey) { e.preventDefault(); openTopic(a.dataset.topic, { focus: e.detail === 0 }); return; }
    const r = e.target.closest("a[data-route]");
    if (r && !e.ctrlKey && !e.metaKey) { e.preventDefault(); selectRoute(r.dataset.route); }
  });

  // ── Keyboard ─────────────────────────────────────────
  function nodeCenter(n) { const r = $(".node-bg", n); return { x: +r.getAttribute("x") + 100, y: +r.getAttribute("y") + 35 }; }
  svg.addEventListener("focusin", (e) => { const n = e.target.closest(".node"); if (n) centerOn(n.dataset.id); });
  svg.addEventListener("keydown", (e) => {
    const n = e.target.closest(".node");
    const dirs = { ArrowLeft: [-1, 0], ArrowRight: [1, 0], ArrowUp: [0, -1], ArrowDown: [0, 1] };
    if (n && dirs[e.key]) {
      e.preventDefault();
      const [dx, dy] = dirs[e.key], c = nodeCenter(n);
      let best = null, bestScore = Infinity;
      nodes.forEach((m) => {
        if (m === n || m.classList.contains("dim")) return;
        const p = nodeCenter(m), vx = p.x - c.x, vy = p.y - c.y;
        const along = vx * dx + vy * dy;
        if (along <= 1) return;
        const score = along + 2.5 * Math.abs(vx * dy - vy * dx);
        if (score < bestScore) { bestScore = score; best = m; }
      });
      best?.focus();
    }
  });
  document.addEventListener("keydown", (e) => {
    if (e.target.matches("input, select, textarea")) { if (e.key === "Escape") e.target.blur(); return; }
    if (e.key === "Escape") { closePanel(); return; }
    if (!body.classList.contains("view-map")) return;
    const s = size();
    if (e.key === "+" || e.key === "=") zoomAt(s.w / 2, s.h / 2, view.k * 1.3);
    else if (e.key === "-") zoomAt(s.w / 2, s.h / 2, view.k / 1.3);
    else if (e.key === "0") fitAll();
    else if (e.key === "/") { e.preventDefault(); $("#q").focus(); }
  });

  // ── Filters ──────────────────────────────────────────
  const norm = (s) => s.toLowerCase().replace(/ё/g, "е");
  const levels = new Set([1, 2, 3]);
  let query = "", mtype = "", mine = "", route = null;
  function matches(t) {
    if (!levels.has(t.level)) return false;
    if (!statusMatches(progress[t.id], mine)) return false;
    if (mtype && !t.types.includes(mtype)) return false;
    if (query) {
      t._s = t._s || norm(t.search);
      if (!query.split(/\s+/).every((w) => t._s.includes(w))) return false;
    }
    return true;
  }
  function applyFilters() {
    if (!data) return;
    const rset = route ? new Set(data.routes.find((r) => r.id === route)?.topics || []) : null;
    let shown = 0;
    topics.forEach((t) => {
      const ok = matches(t) && (!rset || rset.has(t.id));
      if (ok) shown++;
      nodes.get(t.id)?.classList.toggle("dim", !ok);
      nodes.get(t.id)?.classList.toggle("match", ok && Boolean(query || mine));
      lis.get(t.id)?.classList.toggle("dim", !ok);
    });
    edges.forEach((e) => {
      const off = nodes.get(e.dataset.from)?.classList.contains("dim") || nodes.get(e.dataset.to)?.classList.contains("dim");
      e.classList.toggle("dim", Boolean(off));
    });
    const filtered = Boolean(query || mtype || mine || levels.size < 3);
    $("#q-count").textContent = filtered ? String(shown) : "";
    $$(".list-sec").forEach((s) => { s.hidden = Boolean(query || mtype || mine || route) && !$$(".li", s).some((li) => !li.classList.contains("dim")); });
    top.classList.toggle("filtered", Boolean(mtype || mine || route || levels.size < 3));
  }
  onProgress = () => { if (mine) applyFilters(); };
  $("#q").addEventListener("input", (e) => { query = norm(e.target.value.trim()); ready.then(applyFilters); });
  $("#q").addEventListener("keydown", (e) => {
    if (e.key !== "Enter") return;
    const first = [...topics.values()].find((t) => !nodes.get(t.id)?.classList.contains("dim"));
    if (first) openTopic(first.id);
  });
  $$(".chip[data-level]").forEach((b) => b.addEventListener("click", () => {
    const l = +b.dataset.level;
    if (levels.has(l) && levels.size > 1) levels.delete(l); else levels.add(l);
    $$(".chip[data-level]").forEach((c) => c.setAttribute("aria-pressed", String(levels.has(+c.dataset.level))));
    ready.then(applyFilters);
  }));
  $("#type").addEventListener("change", (e) => { mtype = e.target.value; ready.then(applyFilters); });
  $("#mine").addEventListener("change", (e) => { mine = e.target.value; ready.then(applyFilters); });

  // Phone: filters and routes fold away behind one button next to the search.
  function showFilters(on) { top.classList.toggle("filters-open", on); filtersBtn.setAttribute("aria-expanded", String(on)); }
  filtersBtn.addEventListener("click", () => showFilters(!top.classList.contains("filters-open")));

  // ── Routes ───────────────────────────────────────────
  const routeLayer = $("#route-layer");
  let routebar = null;
  function selectRoute(id, opts = {}) {
    ready.then(() => {
      route = route === id && !opts.force ? null : id;
      $$(".route-chip").forEach((c) => c.setAttribute("aria-pressed", String(c.dataset.route === route)));
      nodes.forEach((n) => { n.classList.remove("in-route"); $(".node-num", n).textContent = ""; });
      routeLayer.innerHTML = "";
      routebar?.remove(); routebar = null;
      applyFilters();
      if (!route) { if (!selected) setHash(location.pathname + location.search); return; }
      const r = data.routes.find((x) => x.id === route);
      if (!r) return;
      const pts = [];
      r.topics.forEach((tid, i) => {
        const n = nodes.get(tid); if (!n) return;
        n.classList.add("in-route");
        $(".node-num", n).textContent = String(i + 1);
        const t = topics.get(tid);
        pts.push({ x: t.x + t.w / 2, y: t.y + t.h / 2, t });
      });
      let d = "";
      pts.forEach((p, i) => {
        if (i === 0) { d += `M${p.x},${p.y}`; return; }
        const q = pts[i - 1], my = (q.y + p.y) / 2;
        d += ` C${q.x},${my} ${p.x},${my} ${p.x},${p.y}`;
      });
      const path = document.createElementNS("http://www.w3.org/2000/svg", "path");
      path.setAttribute("d", d); path.setAttribute("class", "route-line");
      routeLayer.appendChild(path);
      if (pts.length && body.classList.contains("view-map")) {
        const xs = pts.map((p) => p.t.x), ys = pts.map((p) => p.t.y);
        const x0 = Math.min(...xs), y0 = Math.min(...ys);
        fitBox({ x: x0, y: y0, w: Math.max(...xs) + 200 - x0, h: Math.max(...ys) + 70 - y0 }, 1, 70);
      }
      routebar = document.createElement("div");
      routebar.className = "hint routebar";
      const page = single ? "" : ` · <a class="rb-page">Страница пути</a>`;
      routebar.innerHTML = `<p><b></b> <span class="rb-desc"></span><br><a href="#" class="rb-first">Начать с первой темы →</a>${page}</p><button type="button" aria-label="Выйти из пути">✕</button>`;
      $("b", routebar).textContent = r.title;
      $(".rb-desc", routebar).textContent = r.description || "";
      if (!single) $(".rb-page", routebar).href = `${root}route/${r.id}/`;
      $(".rb-first", routebar).addEventListener("click", (e) => { e.preventDefault(); openTopic(r.topics[0]); });
      $("button", routebar).addEventListener("click", () => selectRoute(route));
      $("#hint").hidden = true;
      wrap.appendChild(routebar);
      if (!selected) setHash("#route/" + route);
    });
  }
  $$(".route-chip").forEach((c) => c.addEventListener("click", () => {
    selectRoute(c.dataset.route);
    showFilters(false);
    if (body.classList.contains("view-list")) $(".list").scrollIntoView();
  }));

  // ── Map / list ───────────────────────────────────────
  let started = false;
  function setView(v, remember = true) {
    body.classList.toggle("view-map", v === "map");
    body.classList.toggle("view-list", v === "list");
    $$(".seg-btn").forEach((b) => b.setAttribute("aria-pressed", String(b.dataset.view === v)));
    if (remember) store.set("omap.view", v);
    if (v === "map") { resize(); if (!started) { started = true; start(); } }
  }
  $$(".seg-btn").forEach((b) => b.addEventListener("click", () => setView(b.dataset.view)));
  // The map area changes with the window and with the header (filters fold on phones).
  const onResize = () => { if (body.classList.contains("view-map")) { resize(); clampView(); apply(); } };
  if ("ResizeObserver" in window) new ResizeObserver(onResize).observe(wrap);
  else window.addEventListener("resize", onResize);

  // Hint
  if (store.get("omap.hint", false)) $("#hint").hidden = true;
  $("#hint-close").addEventListener("click", () => { $("#hint").hidden = true; store.set("omap.hint", true); });

  // ── Start ────────────────────────────────────────────
  // The map first everywhere, phones too (Bogdan, 16 Nov 2026); the list stays one tap away.
  setView(store.get("omap.view", "map"), false);
  function fromHash() {
    const h = decodeURIComponent(location.hash.slice(1));
    if (h === "map" || h === "list") { setView(h); return; }
    const [kind, id] = h.split("/");
    if (kind === "topic" && id) openTopic(id, { hash: false });
    else if (kind === "route" && id) selectRoute(id, { force: true });
  }
  window.addEventListener("hashchange", fromHash);
  fromHash();
})();
