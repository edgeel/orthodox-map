//! End-to-end checks: content files → validation → layout → rendered pages.

use orthodox_map::layout;
use orthodox_map::model::{self, Content, Diag};
use orthodox_map::render::{self, SiteConfig};
use orthodox_map::validate;
use std::path::{Path, PathBuf};

const SECTIONS: &str = r##"
[[section]]
id = "osnovy"
title = "Основы веры"
order = 1
color = "#8C2F39"

[[section]]
id = "dogmatika"
title = "Догматика"
order = 2
color = "#2E6F62"
"##;

/// A fresh folder under target/ for one test.
fn tmp(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn topic(id: &str, section: &str, level: u8, needs: &[&str], body: &str) -> String {
    format!(
        "---\nid: \"{id}\"\ntitle: \"Тема {id}\"\nsection: \"{section}\"\nlevel: {level}\n\
         needs: [{needs}]\nsummary: \"Зачем знать {id}.\"\ntime: \"≈ 1 ч\"\nmaterials:\n\
         \x20 - title: \"Книга про {id}\"\n    author: \"свт. Филарет\"\n    type: \"book\"\n\
         \x20   url: \"https://azbyka.ru/otechnik/{id}/\"\n    lang: \"ru\"\n    start: true\n\
         ---\n\n{body}\n",
        needs = needs
            .iter()
            .map(|n| format!("\"{n}\""))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

/// Writes a content folder: `files` are (path under topics/, text).
fn content(name: &str, files: &[(&str, String)], routes: &str) -> (PathBuf, Content, Vec<Diag>) {
    let dir = tmp(name);
    std::fs::write(dir.join("sections.toml"), SECTIONS).unwrap();
    std::fs::write(dir.join("routes.toml"), routes).unwrap();
    for (rel, text) in files {
        let p = dir.join("topics").join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    let (c, diags) = model::load(&dir);
    (dir, c, diags)
}

fn errors(d: &[Diag]) -> Vec<String> {
    d.iter()
        .filter(|d| !d.warning)
        .map(|d| d.to_string())
        .collect()
}

fn warnings(d: &[Diag]) -> Vec<String> {
    d.iter()
        .filter(|d| d.warning)
        .map(|d| d.to_string())
        .collect()
}

fn good() -> Vec<(&'static str, String)> {
    vec![
        (
            "osnovy/simvol.md",
            topic(
                "simvol",
                "osnovy",
                1,
                &[],
                "О Символе.\n\n## Дальше\n\n- [x](troica)",
            ),
        ),
        (
            "osnovy/molitva.md",
            topic("molitva", "osnovy", 1, &[], "О молитве."),
        ),
        (
            "dogmatika/troica.md",
            topic("troica", "dogmatika", 2, &["simvol"], "См. [[simvol]]."),
        ),
    ]
}

#[test]
fn demo_content_is_valid() {
    let (c, load) = model::load(Path::new("content"));
    assert!(load.is_empty(), "{load:?}");
    let d = validate::validate(&c);
    assert!(errors(&d).is_empty(), "{:#?}", errors(&d));
}

#[test]
fn parses_frontmatter_fields() {
    let t = model::parse_topic(
        Path::new("topics/osnovy/simvol.md"),
        &topic("simvol", "osnovy", 1, &["a", "b"], "Тело."),
    )
    .unwrap();
    assert_eq!(t.meta.id, "simvol");
    assert_eq!(t.meta.level, 1);
    assert_eq!(t.meta.needs, ["a", "b"]);
    assert_eq!(t.meta.materials.len(), 1);
    assert!(t.meta.materials[0].start);
    assert_eq!(t.meta.materials[0].kind.key(), "book");
    assert_eq!(t.body.trim(), "Тело.");
}

#[test]
fn missing_frontmatter_says_how_to_fix() {
    let e = model::parse_topic(Path::new("x.md"), "# Заголовок\n").unwrap_err();
    assert!(!e.warning);
    assert!(e.message.contains("---"), "{}", e.message);
}

#[test]
fn colon_in_unquoted_value_points_to_line_and_quotes() {
    let text = "---\nid: x\ntitle: Молитва: как начать\n---\n";
    let e = model::parse_topic(Path::new("x.md"), text).unwrap_err();
    assert!(e.message.contains("строка 3"), "{}", e.message);
    assert!(e.message.contains("кавычки"), "{}", e.message);
}

#[test]
fn good_content_has_no_errors_or_warnings() {
    let (_, c, load) = content("good", &good(), "");
    assert!(load.is_empty());
    let d = validate::validate(&c);
    assert!(d.is_empty(), "{d:#?}");
}

#[test]
fn unknown_need_and_route_topic_are_errors_with_file_and_id() {
    let mut files = good();
    files.push((
        "osnovy/post.md",
        topic("post", "osnovy", 1, &["nesushchestvuet"], ""),
    ));
    let routes = "[[route]]\nid = \"r\"\ntitle = \"R\"\ntopics = [\"simvol\", \"net-takoy\"]\n";
    let (_, c, _) = content("unknown", &files, routes);
    let e = errors(&validate::validate(&c));
    assert!(
        e.iter()
            .any(|m| m.contains("post.md") && m.contains("`nesushchestvuet`")),
        "{e:#?}"
    );
    assert!(
        e.iter()
            .any(|m| m.contains("routes.toml") && m.contains("`net-takoy`")),
        "{e:#?}"
    );
}

#[test]
fn cycle_is_reported_as_a_path() {
    let files = vec![
        ("osnovy/a.md", topic("a", "osnovy", 1, &["b"], "")),
        ("osnovy/b.md", topic("b", "osnovy", 1, &["a"], "")),
    ];
    let (_, c, _) = content("cycle", &files, "");
    let e = errors(&validate::validate(&c));
    assert!(
        e.iter().any(|m| m.contains("цикл в needs: a → b → a")),
        "{e:#?}"
    );
}

#[test]
fn folder_file_name_and_level_must_match() {
    let files = vec![
        ("dogmatika/a.md", topic("a", "osnovy", 1, &[], "")),
        ("osnovy/b.md", topic("c", "osnovy", 4, &[], "")),
    ];
    let (_, c, _) = content("mismatch", &files, "");
    let e = errors(&validate::validate(&c)).join("\n");
    assert!(e.contains("папка должна совпадать"), "{e}");
    assert!(e.contains("не совпадает с id `c`"), "{e}");
    assert!(e.contains("level = 4"), "{e}");
}

#[test]
fn link_to_missing_topic_warns_but_unsafe_link_fails() {
    let files = vec![
        (
            "osnovy/a.md",
            topic("a", "osnovy", 1, &[], "[потом](budet-pozzhe)"),
        ),
        (
            "osnovy/b.md",
            topic("b", "osnovy", 1, &[], "[x](javascript:alert(1))"),
        ),
    ];
    let (_, c, _) = content("links", &files, "");
    let d = validate::validate(&c);
    let (e, w) = (errors(&d).join("\n"), warnings(&d).join("\n"));
    assert!(w.contains("a.md") && w.contains("budet-pozzhe"), "{w}");
    assert!(e.contains("b.md") && e.contains("javascript:"), "{e}");
    assert!(!e.contains("a.md"), "{e}");
}

#[test]
fn material_url_must_be_http() {
    let t = topic("a", "osnovy", 1, &[], "").replace("https://azbyka", "ftp://azbyka");
    let (_, c, _) = content("url", &[("osnovy/a.md", t)], "");
    let e = errors(&validate::validate(&c)).join("\n");
    assert!(e.contains("материал 1") && e.contains("https://"), "{e}");
}

#[test]
fn layout_is_deterministic_and_grows_upward() {
    let (_, c, _) = content("layout", &good(), "");
    let a = layout::compute(&c);
    let mut shuffled = c.clone();
    shuffled.topics.reverse();
    shuffled.sections.reverse();
    let b = layout::compute(&shuffled);
    assert_eq!(a, b);
    assert_eq!(a, layout::compute(&c));
    // Level 1 sits below level 2 (SVG y grows downwards).
    assert!(a.nodes["simvol"].y > a.nodes["troica"].y);
    // Sections are columns left to right by order.
    assert!(a.nodes["simvol"].x < a.nodes["troica"].x);
    assert!(
        a.edges
            .iter()
            .any(|e| e.from == "simvol" && e.to == "troica")
    );
}

#[test]
fn topic_page_has_materials_and_links() {
    let (dir, c, _) = content("render", &good(), "");
    let out = dir.join("dist");
    let built = render::build(&c, &SiteConfig::default(), &out)
        .unwrap()
        .unwrap();
    assert_eq!(built.topics, 3);
    let page = std::fs::read_to_string(out.join("topic/troica/index.html")).unwrap();
    assert!(page.contains("Тема troica"));
    assert!(page.contains("Книга про troica"));
    assert!(page.contains("href=\"https://azbyka.ru/otechnik/troica/\""));
    // needs and the wiki link in the body point to the other topic page.
    assert!(page.contains("../../topic/simvol/"), "{page}");
    assert!(out.join("index.html").exists());
    assert!(out.join("data.json").exists());
    assert!(out.join("404.html").exists());
}

#[test]
fn build_refuses_content_with_errors() {
    let files = vec![("osnovy/a.md", topic("a", "osnovy", 1, &["net"], ""))];
    let (dir, c, _) = content("refuse", &files, "");
    let res = render::build(&c, &SiteConfig::default(), &dir.join("dist")).unwrap();
    assert!(res.is_err());
    assert!(!dir.join("dist/index.html").exists());
}

const ROUTES: &str = r##"
[[route]]
id = "start"
title = "С чего начать"
description = "Первые шаги"
topics = ["simvol", "troica"]
"##;

#[test]
fn single_file_has_everything_inline_and_no_local_links() {
    let (_, c, _) = content("single", &good(), ROUTES);
    let site = SiteConfig {
        base_url: "https://edgeel.github.io/orthodox-map/".into(),
        ..SiteConfig::default()
    };
    let single = render::single(&c, &site).unwrap().unwrap();
    let html = &single.html;
    assert_eq!(single.topics, 3);

    // Data, styles and script are in the document; nothing is loaded from the site.
    let start = html
        .find(r#"<script type="application/json" id="omap-data">"#)
        .expect("inline data");
    let json = &html[start..];
    let json = &json[json.find('>').unwrap() + 1..json.find("</script>").unwrap()];
    let data: serde_json::Value = serde_json::from_str(json).unwrap();
    assert_eq!(data["topics"].as_array().unwrap().len(), 3);
    assert_eq!(data["routes"][0]["id"], "start");
    assert!(html.contains("<style>") && html.contains("--accent"));
    assert!(html.contains("pure:start"), "app.js inline");
    assert!(html.contains("data-single"));
    for bad in [
        r#"href="/"#,
        r#"src="/"#,
        r#"href="topic/"#,
        r#"href="route/"#,
        r#"href="assets/"#,
        r#"src="assets/"#,
        r#"href="./"#,
        "manifest.webmanifest",
        r#"rel="stylesheet""#,
        r#"<script src="#,
    ] {
        assert!(!html.contains(bad), "single file still has {bad}");
    }
    // Inside the cards (JSON-escaped) internal links are hashes too.
    assert!(!json.contains(r#"href=\"topic/"#), "{json}");
    assert!(json.contains(r##"href=\"#topic/simvol\""##), "{json}");
    // The share link points at the public site, not at the file.
    assert!(json.contains("https://edgeel.github.io/orthodox-map/topic/troica/"));
    // Topic and route links on the page stay inside the document.
    assert!(html.contains(r##"href="#topic/troica""##));
    assert!(html.contains(r##"href="#route/start""##));
    assert!(html.contains("Книга про troica"));
    assert!(html.contains("https://azbyka.ru/otechnik/troica/"));
    // Every href is a hash, http(s) or the inline icon.
    for part in html.split("href=\"").skip(1) {
        let url = &part[..part.find('"').unwrap()];
        assert!(
            url.starts_with('#') || url.starts_with("https://") || url.starts_with("data:"),
            "local link in the single file: {url}"
        );
    }
}

#[test]
fn site_build_also_writes_the_single_file() {
    let (dir, c, _) = content("with-single", &good(), ROUTES);
    let out = dir.join("dist");
    render::build(&c, &SiteConfig::default(), &out)
        .unwrap()
        .unwrap();
    let one = std::fs::read_to_string(out.join(render::SINGLE_FILE)).unwrap();
    assert!(one.contains(r#"id="omap-data""#));
    let index = std::fs::read_to_string(out.join("index.html")).unwrap();
    assert!(index.contains(r#"href="orthodox-map.html" download"#));
    // The site itself still loads its files.
    assert!(index.contains("assets/app.js?v="));
    assert!(!index.contains(r#"id="omap-data""#));
}

#[test]
fn single_file_into_a_new_folder_can_be_rebuilt_over() {
    let (dir, c, _) = content("single-new-dir", &good(), "");
    let file = dir.join("dist/orthodox-map.html");
    let single = render::single(&c, &SiteConfig::default()).unwrap().unwrap();
    render::write_single(&file, &single.html).unwrap();
    assert!(file.exists());
    // dist/ was made by omap, so a full build may clear it.
    render::build(&c, &SiteConfig::default(), &dir.join("dist"))
        .unwrap()
        .unwrap();
}

#[test]
fn card_has_three_progress_states_and_map_filters_by_them() {
    let (dir, c, _) = content("states", &good(), "");
    let out = dir.join("dist");
    render::build(&c, &SiteConfig::default(), &out)
        .unwrap()
        .unwrap();
    let page = std::fs::read_to_string(out.join("topic/simvol/index.html")).unwrap();
    for st in ["want", "reading", "done"] {
        assert!(page.contains(&format!(r#"data-st="{st}""#)), "{st}");
    }
    assert!(page.contains("Хочу изучить"));
    let index = std::fs::read_to_string(out.join("index.html")).unwrap();
    assert!(index.contains(r#"<select id="mine">"#));
    assert!(index.contains(r#"<option value="want">"#));
    assert!(index.contains(r#"id="filters-btn""#));
    assert!(index.contains(r#"id="sheet-grip""#));
}

#[test]
fn app_js_knows_the_want_state_and_opens_the_map_on_phones() {
    let js = std::fs::read_to_string("assets/app.js").unwrap();
    assert!(js.contains(r#"const STATES = ["want", "reading", "done"];"#));
    assert!(js.contains(r#"setView(store.get("omap.view", "map"), false);"#));
    assert!(
        !js.contains("</script"),
        "app.js is inlined into the single file"
    );
    let css = std::fs::read_to_string("assets/style.css").unwrap();
    assert!(css.contains(".node.st-want") && css.contains(".li.st-want"));
}
