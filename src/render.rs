//! Renders the validated content into a static site.

use crate::layout::{self, Layout};
use crate::markdown;
use crate::model::{Content, Diag, Material, Topic};
use crate::validate;
use anyhow::{Context, Result, bail};
use minijinja::{Environment, context};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

/// Site settings from `site.toml` (kept outside `content/`, which is replaced wholesale).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SiteConfig {
    #[serde(default = "default_title")]
    pub title: String,
    #[serde(default = "default_short")]
    pub short_title: String,
    #[serde(default)]
    pub description: String,
    /// Full public URL with a trailing slash, e.g. `https://edgeel.github.io/orthodox-map/`.
    #[serde(default)]
    pub base_url: String,
    #[serde(default = "default_lang")]
    pub lang: String,
    #[serde(default = "default_roots")]
    pub roots_label: String,
}

fn default_title() -> String {
    "Карта изучения православия".into()
}
fn default_short() -> String {
    "Карта".into()
}
fn default_lang() -> String {
    "ru".into()
}
fn default_roots() -> String {
    "Корни: вера Церкви".into()
}

impl Default for SiteConfig {
    fn default() -> Self {
        toml::from_str("").expect("defaults")
    }
}

impl SiteConfig {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(path).with_context(|| format!("{}", path.display()))?;
        let mut cfg: SiteConfig =
            toml::from_str(&text).with_context(|| format!("{}: не читается", path.display()))?;
        if !cfg.base_url.is_empty() && !cfg.base_url.ends_with('/') {
            cfg.base_url.push('/');
        }
        Ok(cfg)
    }

    /// Path part of `base_url` (`/orthodox-map/`), `/` when there is none.
    pub fn base_path(&self) -> String {
        let rest = self.base_url.split_once("://").map_or("", |(_, r)| r);
        match rest.find('/') {
            Some(i) => rest[i..].to_string(),
            None => "/".to_string(),
        }
    }
}

pub const LEVELS: [(u8, &str, &str, &str); 3] = [
    (1, "🌱", "Начальный", "начальный уровень"),
    (2, "🌿", "Средний", "средний уровень"),
    (3, "🌳", "Углублённый", "углублённый уровень"),
];

fn level_icon(l: u8) -> &'static str {
    LEVELS.iter().find(|x| x.0 == l).map_or("•", |x| x.1)
}
fn level_label(l: u8) -> &'static str {
    LEVELS.iter().find(|x| x.0 == l).map_or("", |x| x.3)
}

fn ru_plural(n: usize, one: &str, few: &str, many: &str) -> String {
    let (m10, m100) = (n % 10, n % 100);
    let w = if m10 == 1 && m100 != 11 {
        one
    } else if (2..=4).contains(&m10) && !(12..=14).contains(&m100) {
        few
    } else {
        many
    };
    format!("{n} {w}")
}

/// FNV-1a, for cache-busting versions without an extra crate.
pub fn short_hash(data: &[u8]) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in data {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{:08x}", h as u32 ^ (h >> 32) as u32)
}

#[derive(Serialize, Clone)]
struct Ref {
    id: String,
    title: String,
}

#[derive(Serialize, Clone)]
struct MatView {
    title: String,
    author: String,
    #[serde(rename = "type")]
    kind: &'static str,
    type_label: &'static str,
    icon: &'static str,
    url: String,
    lang: &'static str,
    time: String,
    start: bool,
    note: String,
    alt_url: Option<String>,
}

impl From<&Material> for MatView {
    fn from(m: &Material) -> Self {
        MatView {
            title: m.title.clone(),
            author: m.author.clone(),
            kind: m.kind.key(),
            type_label: m.kind.label(),
            icon: m.kind.icon(),
            url: m.url.clone(),
            lang: match m.lang {
                crate::model::Lang::Ru => "ru",
                crate::model::Lang::En => "en",
            },
            time: m.time.clone(),
            start: m.start,
            note: m.note.clone(),
            alt_url: m.alt_url.clone(),
        }
    }
}

#[derive(Serialize, Clone)]
struct TopicView {
    id: String,
    title: String,
    section: String,
    section_title: String,
    section_icon: String,
    color: String,
    level: u8,
    level_icon: &'static str,
    level_label: &'static str,
    order: i64,
    summary: String,
    time: String,
    tags: Vec<String>,
    needs: Vec<Ref>,
    opens: Vec<Ref>,
    routes: Vec<Ref>,
    next: Vec<String>,
    materials: Vec<MatView>,
    body_html: String,
}

/// Everything about one built site, for tests and for the CLI summary.
pub struct Built {
    pub layout: Layout,
    pub topics: usize,
    pub routes: usize,
    pub warnings: Vec<Diag>,
    pub files: Vec<String>,
}

pub fn environment() -> Environment<'static> {
    let mut env = Environment::new();
    // Escape &<>"' only: minijinja's default also turns every `/` in URLs into `&#x2f;`.
    env.set_formatter(|out, state, value| {
        if state.auto_escape() == minijinja::AutoEscape::Html
            && !value.is_safe()
            && let Some(s) = value.as_str()
        {
            return out
                .write_str(&markdown::escape(s))
                .map_err(minijinja::Error::from);
        }
        minijinja::escape_formatter(out, state, value)
    });
    for (name, src) in [
        ("base.html", include_str!("../templates/base.html")),
        ("index.html", include_str!("../templates/index.html")),
        ("card.html", include_str!("../templates/card.html")),
        ("topic.html", include_str!("../templates/topic.html")),
        ("route.html", include_str!("../templates/route.html")),
        ("404.html", include_str!("../templates/404.html")),
    ] {
        env.add_template(name, src).expect("template parses");
    }
    env
}

pub const ASSETS: [(&str, &str); 3] = [
    ("assets/style.css", include_str!("../assets/style.css")),
    ("assets/app.js", include_str!("../assets/app.js")),
    ("assets/icon.svg", include_str!("../assets/icon.svg")),
];
const SW_JS: &str = include_str!("../assets/sw.js");

/// Topics sorted the way the list and the sitemap show them: by section order, level, layout.
fn topic_views(c: &Content, layout: &Layout) -> (Vec<TopicView>, HashMap<String, String>) {
    let titles: HashMap<String, String> = c
        .topics
        .iter()
        .map(|t| (t.meta.id.clone(), t.meta.title.clone()))
        .collect();
    let sections: HashMap<&str, &crate::model::Section> =
        c.sections.iter().map(|s| (s.id.as_str(), s)).collect();
    let mut opens: BTreeMap<&str, Vec<&Topic>> = BTreeMap::new();
    for t in &c.topics {
        for n in &t.meta.needs {
            opens.entry(n.as_str()).or_default().push(t);
        }
    }
    let r = |id: &str| Ref {
        id: id.to_string(),
        title: titles.get(id).cloned().unwrap_or_default(),
    };

    let mut views: Vec<TopicView> = c
        .topics
        .iter()
        .map(|t| {
            let m = &t.meta;
            let s = sections.get(m.section.as_str());
            let body = markdown::render(&t.body, &titles, "{root}");
            let mut ops: Vec<&Topic> = opens.get(m.id.as_str()).cloned().unwrap_or_default();
            ops.sort_by(|a, b| {
                (a.meta.level, a.meta.order, &a.meta.id).cmp(&(
                    b.meta.level,
                    b.meta.order,
                    &b.meta.id,
                ))
            });
            let mut materials: Vec<MatView> = m.materials.iter().map(MatView::from).collect();
            materials.sort_by_key(|x| !x.start); // stable: «с чего начать» first, rest as written
            TopicView {
                id: m.id.clone(),
                title: m.title.clone(),
                section: m.section.clone(),
                section_title: s.map(|s| s.title.clone()).unwrap_or_default(),
                section_icon: s.map(|s| s.icon.clone()).unwrap_or_default(),
                color: s.map(|s| s.color.clone()).unwrap_or_default(),
                level: m.level,
                level_icon: level_icon(m.level),
                level_label: level_label(m.level),
                order: m.order,
                summary: m.summary.clone(),
                time: m.time.clone(),
                tags: m.tags.clone(),
                needs: m.needs.iter().map(|n| r(n)).collect(),
                opens: ops.iter().map(|o| r(o.id())).collect(),
                routes: c
                    .routes
                    .iter()
                    .filter(|rt| rt.topics.contains(&m.id))
                    .map(|rt| Ref {
                        id: rt.id.clone(),
                        title: rt.title.clone(),
                    })
                    .collect(),
                next: body.next,
                materials,
                body_html: body.html,
            }
        })
        .collect();

    let col_index: HashMap<&str, usize> = layout
        .columns
        .iter()
        .enumerate()
        .map(|(i, c)| (c.section.as_str(), i))
        .collect();
    views.sort_by(|a, b| {
        let ka = (
            col_index.get(a.section.as_str()),
            a.level,
            layout.nodes.get(&a.id).map(|n| (-(n.y as i64), n.x as i64)),
        );
        let kb = (
            col_index.get(b.section.as_str()),
            b.level,
            layout.nodes.get(&b.id).map(|n| (-(n.y as i64), n.x as i64)),
        );
        ka.cmp(&kb)
    });
    (views, titles)
}

fn with_root(v: &TopicView, root: &str) -> TopicView {
    let mut v = v.clone();
    v.body_html = v.body_html.replace("{root}", root);
    v
}

fn write(out: &Path, rel: &str, data: impl AsRef<[u8]>, files: &mut Vec<String>) -> Result<()> {
    let path = out.join(rel);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).with_context(|| format!("создать {}", dir.display()))?;
    }
    std::fs::write(&path, data).with_context(|| format!("записать {}", path.display()))?;
    files.push(rel.to_string());
    Ok(())
}

/// Marker that lets `build` wipe an output folder it made before, and nothing else.
pub const MARKER: &str = ".omap-dist";

fn prepare_out(out: &Path) -> Result<()> {
    if out.exists() {
        let empty = std::fs::read_dir(out)?.next().is_none();
        if !empty && !out.join(MARKER).exists() {
            bail!(
                "{} не пуста и собрана не omap (нет файла {MARKER}); не трогаю её — укажите другую папку --out",
                out.display()
            );
        }
        std::fs::remove_dir_all(out).with_context(|| format!("очистить {}", out.display()))?;
    }
    std::fs::create_dir_all(out)?;
    std::fs::write(out.join(MARKER), "omap build output; safe to delete\n")?;
    Ok(())
}

/// Validated content, laid out and turned into views: what the site and the single file share.
struct Prepared<'a> {
    c: &'a Content,
    layout: Layout,
    views: Vec<TopicView>,
    sections: Vec<&'a crate::model::Section>,
    warnings: Vec<Diag>,
}

fn prepare(c: &Content) -> std::result::Result<Prepared<'_>, Vec<Diag>> {
    let diags = validate::validate(c);
    if validate::has_errors(&diags) {
        return Err(diags);
    }
    let layout = layout::compute(c);
    let (views, _titles) = topic_views(c, &layout);
    let mut sections: Vec<&crate::model::Section> = c.sections.iter().collect();
    sections.sort_by(|a, b| a.order.cmp(&b.order).then(a.id.cmp(&b.id)));
    Ok(Prepared {
        c,
        layout,
        views,
        sections,
        warnings: diags,
    })
}

/// `data.json`: layout, search text and the rendered card of every topic. For the single file
/// (`single`) internal links become `#topic/<id>` and the card's share link points at the public site.
fn data_value(
    p: &Prepared,
    env: &Environment,
    site: &SiteConfig,
    single: bool,
) -> Result<serde_json::Value> {
    let panel_tpl = env.get_template("card.html")?;
    let root = if single { "" } else { "./" };
    let mut data_topics = Vec::new();
    for v in &p.views {
        let tv = with_root(v, root);
        let card = panel_tpl.render(context! {
            t => &tv, root, heading => "h2", permalink_base => if single { &site.base_url } else { "" },
        })?;
        let card = if single { hash_links(&card) } else { card };
        let node = &p.layout.nodes[&v.id];
        let search = format!(
            "{} {} {} {}",
            v.title,
            v.summary,
            v.tags.join(" "),
            v.materials
                .iter()
                .map(|m| format!("{} {}", m.title, m.author))
                .collect::<Vec<_>>()
                .join(" ")
        );
        data_topics.push(serde_json::json!({
            "id": v.id, "title": v.title, "section": v.section, "level": v.level,
            "needs": v.needs.iter().map(|n| &n.id).collect::<Vec<_>>(),
            "next": v.next,
            "types": v.materials.iter().map(|m| m.kind).collect::<std::collections::BTreeSet<_>>(),
            "search": search,
            "x": node.x, "y": node.y, "w": node.w, "h": node.h,
            "card": card,
        }));
    }
    Ok(serde_json::json!({
        "title": site.title,
        "width": p.layout.width,
        "height": p.layout.height,
        "sections": p.sections.iter().map(|s| serde_json::json!({
            "id": s.id, "title": s.title, "color": s.color, "icon": s.icon,
            "count": p.views.iter().filter(|v| v.section == s.id).count(),
        })).collect::<Vec<_>>(),
        "topics": data_topics,
        "routes": p.c.routes,
    }))
}

/// Everything `index.html` needs besides `root`, `version` and the single-file parts.
fn index_context(
    p: &Prepared,
    site: &SiteConfig,
) -> Result<serde_json::Map<String, serde_json::Value>> {
    let (layout, views, sections) = (&p.layout, &p.views, &p.sections);
    let columns: Vec<_> = layout
        .columns
        .iter()
        .map(|col| {
            let s = sections
                .iter()
                .find(|s| s.id == col.section)
                .expect("column of a section");
            let max_chars = ((col.w - 32.0) / 9.0) as usize;
            serde_json::json!({
                "id": s.id, "x": col.x, "w": col.w, "color": s.color, "icon": s.icon,
                "lines": layout::wrap_title(&s.title, max_chars.max(12)),
                "count": views.iter().filter(|v| v.section == s.id).count(),
            })
        })
        .collect();
    let col_h = layout.bands.iter().map(|b| b.h).sum::<f64>() + layout::HEADER_H;
    let nodes: Vec<_> = views
        .iter()
        .map(|v| {
            let n = &layout.nodes[&v.id];
            let mats = ru_plural(v.materials.len(), "материал", "материала", "материалов");
            let meta = if v.time.is_empty() { mats } else { format!("{mats} · {}", v.time) };
            serde_json::json!({
                "id": v.id, "title": v.title, "section": v.section, "section_title": v.section_title,
                "color": v.color, "level": v.level, "level_icon": v.level_icon, "level_label": v.level_label,
                "x": n.x, "y": n.y, "w": n.w, "h": n.h, "lines": n.lines, "meta": meta,
            })
        })
        .collect();
    let list_sections: Vec<_> = sections
        .iter()
        .map(|s| {
            let mut ts: Vec<&TopicView> = views.iter().filter(|v| v.section == s.id).collect();
            ts.sort_by_key(|v| (v.level, v.order));
            serde_json::json!({ "id": s.id, "title": s.title, "icon": s.icon, "color": s.color,
                                "description": s.description, "topics": ts })
        })
        .collect();
    let levels: Vec<_> = LEVELS
        .iter()
        .map(|(l, icon, short, label)| serde_json::json!({ "level": l, "icon": icon, "short": short, "label": label }))
        .collect();
    let bands: Vec<_> = layout
        .bands
        .iter()
        .map(|b| serde_json::json!({ "level": b.level, "y": b.y, "h": b.h, "icon": level_icon(b.level) }))
        .collect();
    let mut layout_ctx = serde_json::to_value(layout)?;
    layout_ctx["bands"] = serde_json::Value::Array(bands);
    let types: Vec<_> = crate::model::MaterialType::ALL
        .iter()
        .map(|t| serde_json::json!({ "key": t.key(), "label": t.label(), "icon": t.icon() }))
        .collect();
    let serde_json::Value::Object(ctx) = serde_json::json!({
        "site": site, "layout": layout_ctx, "columns": columns, "col_h": col_h, "nodes": nodes,
        "sections": list_sections, "levels": levels, "types": types, "routes": p.c.routes,
    }) else {
        unreachable!("json! of an object")
    };
    Ok(ctx)
}

/// Validates and renders `content` into `out`. Errors in the content come back as `Err(diags)`.
pub fn build(
    c: &Content,
    site: &SiteConfig,
    out: &Path,
) -> Result<std::result::Result<Built, Vec<Diag>>> {
    let p = match prepare(c) {
        Ok(p) => p,
        Err(diags) => return Ok(Err(diags)),
    };
    let views = &p.views;
    let by_id: HashMap<&str, &TopicView> = views.iter().map(|v| (v.id.as_str(), v)).collect();
    let env = environment();

    // data.json first: its hash versions every asset URL and the service worker cache.
    let data_json = serde_json::to_string(&data_value(&p, &env, site, false)?)?;
    let mut hash_src = data_json.clone().into_bytes();
    for (_, a) in ASSETS {
        hash_src.extend_from_slice(a.as_bytes());
    }
    hash_src.extend_from_slice(SW_JS.as_bytes());
    let version = short_hash(&hash_src);

    prepare_out(out)?;
    let mut files = Vec::new();
    write(out, "data.json", &data_json, &mut files)?;
    for (rel, src) in ASSETS {
        write(out, rel, src, &mut files)?;
    }

    let canonical = |rel: &str| {
        if site.base_url.is_empty() {
            String::new()
        } else {
            format!("{}{rel}", site.base_url)
        }
    };
    let mut ctx = index_context(&p, site)?;
    ctx.insert("root".into(), "./".into());
    ctx.insert("version".into(), version.clone().into());
    ctx.insert("canonical".into(), canonical("").into());
    let index = env.get_template("index.html")?.render(&ctx)?;
    write(out, "index.html", index, &mut files)?;

    // topic pages
    let topic_tpl = env.get_template("topic.html")?;
    for v in views {
        let tv = with_root(v, "../../");
        let html = topic_tpl.render(context! {
            site, root => "../../", version, t => tv, canonical => canonical(&format!("topic/{}/", v.id)),
        })?;
        write(out, &format!("topic/{}/index.html", v.id), html, &mut files)?;
    }

    // route pages
    let route_tpl = env.get_template("route.html")?;
    for r in &c.routes {
        let steps: Vec<_> = r
            .topics
            .iter()
            .filter_map(|id| by_id.get(id.as_str()))
            .map(|v| {
                serde_json::json!({
                    "id": v.id, "title": v.title, "summary": v.summary, "time": v.time, "color": v.color,
                    "level_icon": v.level_icon, "section_title": v.section_title,
                    "start": v.materials.iter().find(|m| m.start),
                })
            })
            .collect();
        let html = route_tpl.render(context! {
            site, root => "../../", version, canonical => canonical(&format!("route/{}/", r.id)),
            r => context! { id => &r.id, title => &r.title, description => &r.description, steps },
        })?;
        write(out, &format!("route/{}/index.html", r.id), html, &mut files)?;
    }

    // 404: served at any path, so links are absolute from the site's base path.
    let html = env
        .get_template("404.html")?
        .render(context! { site, root => site.base_path(), version })?;
    write(out, "404.html", html, &mut files)?;

    // sitemap, robots
    if !site.base_url.is_empty() {
        let mut sm = String::from(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n",
        );
        let mut url = |rel: &str| {
            sm.push_str(&format!(
                "  <url><loc>{}</loc></url>\n",
                markdown::escape(&canonical(rel))
            ));
        };
        url("");
        for v in views {
            url(&format!("topic/{}/", v.id));
        }
        for r in &c.routes {
            url(&format!("route/{}/", r.id));
        }
        sm.push_str("</urlset>\n");
        write(out, "sitemap.xml", sm, &mut files)?;
        write(
            out,
            "robots.txt",
            format!(
                "User-agent: *\nAllow: /\nSitemap: {}sitemap.xml\n",
                site.base_url
            ),
            &mut files,
        )?;
    }

    // PWA
    let manifest = serde_json::json!({
        "name": site.title, "short_name": site.short_title, "description": site.description,
        "lang": site.lang, "start_url": "./", "scope": "./", "display": "standalone",
        "background_color": "#f7f6f3", "theme_color": "#8a1c1c",
        "icons": [{ "src": "assets/icon.svg", "sizes": "any", "type": "image/svg+xml", "purpose": "any" }],
    });
    write(
        out,
        "manifest.webmanifest",
        serde_json::to_string_pretty(&manifest)?,
        &mut files,
    )?;
    let mut shell: Vec<String> = vec![
        "./".into(),
        "data.json".into(),
        "manifest.webmanifest".into(),
    ];
    shell.extend(ASSETS.iter().map(|(rel, _)| format!("{rel}?v={version}")));
    let sw = SW_JS
        .replace("__VERSION__", &version)
        .replace("\"__SHELL__\"", &serde_json::to_string(&shell)?);
    write(out, "sw.js", sw, &mut files)?;
    write(out, ".nojekyll", "", &mut files)?;

    // The same site as one file, offered in the menu for sending to someone.
    let one = single_html(&p, &env, site)?;
    write(out, SINGLE_FILE, one, &mut files)?;

    let topics = views.len();
    Ok(Ok(Built {
        layout: p.layout,
        topics,
        routes: c.routes.len(),
        warnings: p.warnings,
        files,
    }))
}

/// The whole site as one HTML file, for sending to someone or opening from disk (`file://`).
pub struct Single {
    pub html: String,
    pub topics: usize,
    pub routes: usize,
    pub warnings: Vec<Diag>,
}

/// Renders the map page with everything inline: styles, script, `data.json`, the icon. Links to
/// topic and route pages become `#topic/<id>` and `#route/<id>`; no service worker, no requests.
pub fn single(c: &Content, site: &SiteConfig) -> Result<std::result::Result<Single, Vec<Diag>>> {
    let p = match prepare(c) {
        Ok(p) => p,
        Err(diags) => return Ok(Err(diags)),
    };
    let html = single_html(&p, &environment(), site)?;
    Ok(Ok(Single {
        html,
        topics: p.views.len(),
        routes: c.routes.len(),
        warnings: p.warnings,
    }))
}

/// Name of the single file inside a built site (`dist/orthodox-map.html`).
pub const SINGLE_FILE: &str = "orthodox-map.html";

fn single_html(p: &Prepared, env: &Environment, site: &SiteConfig) -> Result<String> {
    let data = data_value(p, env, site, true)?;
    // `<` only occurs inside JSON strings, where `\u003c` means the same; no `</script>` can end the block.
    let data_json = serde_json::to_string(&data)?.replace('<', "\\u003c");
    let mut ctx = index_context(p, site)?;
    ctx.insert("root".into(), "".into());
    ctx.insert("version".into(), short_hash(data_json.as_bytes()).into());
    ctx.insert("single".into(), true.into());
    ctx.insert("css".into(), ASSETS[0].1.into());
    ctx.insert("js".into(), ASSETS[1].1.into());
    ctx.insert("data".into(), data_json.into());
    ctx.insert("icon_uri".into(), svg_data_uri(ASSETS[2].1).into());
    Ok(hash_links(&env.get_template("index.html")?.render(&ctx)?))
}

/// Writes the single file. A folder it has to create gets the build marker, so a later
/// `omap build` into that folder may clear it as before.
pub fn write_single(file: &Path, html: &str) -> Result<()> {
    if let Some(dir) = file.parent().filter(|d| !d.as_os_str().is_empty())
        && !dir.exists()
    {
        std::fs::create_dir_all(dir).with_context(|| format!("создать {}", dir.display()))?;
        std::fs::write(dir.join(MARKER), "omap build output; safe to delete\n")?;
    }
    std::fs::write(file, html).with_context(|| format!("записать {}", file.display()))
}

/// Rewrites `href="topic/<id>/"` to `href="#topic/<id>"` (routes the same) and `href="./"` to `#`.
fn hash_links(html: &str) -> String {
    const HREF: &str = "href=\"";
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    while let Some(i) = rest.find(HREF) {
        let (head, tail) = rest.split_at(i + HREF.len());
        out.push_str(head);
        let end = tail.find('"').unwrap_or(tail.len());
        let url = &tail[..end];
        match ["topic/", "route/"]
            .iter()
            .find_map(|k| url.strip_prefix(k).map(|id| (k, id)))
        {
            Some((kind, id)) => {
                out.push('#');
                out.push_str(kind);
                out.push_str(id.trim_end_matches('/'));
            }
            None if url == "./" || url.is_empty() => out.push('#'),
            None => out.push_str(url),
        }
        rest = &tail[end..];
    }
    out.push_str(rest);
    out
}

/// `data:image/svg+xml,…` with everything but plain URL characters percent-encoded.
fn svg_data_uri(svg: &str) -> String {
    let mut out = String::from("data:image/svg+xml,");
    for b in svg.trim().bytes() {
        if b.is_ascii_alphanumeric() || b"-_.~/:=;,!*()+@".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plural_forms() {
        assert_eq!(
            ru_plural(1, "материал", "материала", "материалов"),
            "1 материал"
        );
        assert_eq!(
            ru_plural(3, "материал", "материала", "материалов"),
            "3 материала"
        );
        assert_eq!(
            ru_plural(11, "материал", "материала", "материалов"),
            "11 материалов"
        );
        assert_eq!(
            ru_plural(22, "материал", "материала", "материалов"),
            "22 материала"
        );
    }

    #[test]
    fn hash_links_rewrite_only_internal_pages() {
        let html = r##"<a href="topic/simvol/">a</a> <a href="route/start/" data-route="start">b</a>
<a href="./">c</a> <a href="#topic/x">d</a> <a href="https://azbyka.ru/topic/y/">e</a>"##;
        assert_eq!(
            hash_links(html),
            r##"<a href="#topic/simvol">a</a> <a href="#route/start" data-route="start">b</a>
<a href="#">c</a> <a href="#topic/x">d</a> <a href="https://azbyka.ru/topic/y/">e</a>"##
        );
    }

    #[test]
    fn svg_data_uri_has_no_raw_quotes_or_brackets() {
        let uri = svg_data_uri("<svg a=\"1\" fill=\"#fff\">\n</svg>");
        assert!(uri.starts_with("data:image/svg+xml,%3Csvg"));
        assert!(!uri.contains(['"', '<', '>', '#', ' ', '\n']), "{uri}");
    }

    #[test]
    fn base_path_from_url() {
        let mut s = SiteConfig::default();
        assert_eq!(s.base_path(), "/");
        s.base_url = "https://edgeel.github.io/orthodox-map/".into();
        assert_eq!(s.base_path(), "/orthodox-map/");
    }
}
