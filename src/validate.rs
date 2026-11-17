//! Checks the content before anything is rendered: ids, references, cycles, materials.

use crate::markdown;
use crate::model::{Content, Diag};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

pub fn is_slug(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
}

pub fn is_hex_color(s: &str) -> bool {
    s.strip_prefix('#')
        .is_some_and(|h| matches!(h.len(), 3 | 6) && h.chars().all(|c| c.is_ascii_hexdigit()))
}

fn is_http(s: &str) -> bool {
    s.starts_with("https://") || s.starts_with("http://")
}

/// Returns every problem found; errors stop the build, warnings don't.
pub fn validate(c: &Content) -> Vec<Diag> {
    let mut d = Vec::new();
    let sf = &c.sections_file;

    // Sections
    let mut section_ids = HashSet::new();
    let mut orders: BTreeMap<i64, &str> = BTreeMap::new();
    for s in &c.sections {
        if !is_slug(&s.id) {
            d.push(Diag::error(
                sf,
                format!(
                    "раздел «{}»: id `{}` — только a-z, 0-9, - и _",
                    s.title, s.id
                ),
            ));
        }
        if !section_ids.insert(s.id.as_str()) {
            d.push(Diag::error(
                sf,
                format!("раздел `{}` объявлен дважды", s.id),
            ));
        }
        if !is_hex_color(&s.color) {
            d.push(Diag::error(
                sf,
                format!("раздел `{}`: цвет `{}` — нужен вид #rrggbb", s.id, s.color),
            ));
        }
        if let Some(other) = orders.insert(s.order, &s.id) {
            d.push(Diag::warning(
                sf,
                format!(
                    "разделы `{other}` и `{}` с одинаковым order = {}",
                    s.id, s.order
                ),
            ));
        }
    }
    if c.sections.is_empty() {
        d.push(Diag::error(sf, "нет ни одного раздела ([[section]])"));
    }

    // Topics: ids first, so references can be checked against the full set.
    let mut by_id: HashMap<&str, &Path> = HashMap::new();
    for t in &c.topics {
        let id = t.id();
        if !is_slug(id) {
            d.push(Diag::error(
                &t.file,
                format!("id `{id}` — только a-z, 0-9, - и _"),
            ));
        }
        if let Some(first) = by_id.insert(id, &t.file) {
            d.push(Diag::error(
                &t.file,
                format!("id `{id}` уже занят темой в {}", first.display()),
            ));
        }
    }
    let titles: HashMap<String, String> = c
        .topics
        .iter()
        .map(|t| (t.meta.id.clone(), t.meta.title.clone()))
        .collect();
    let levels: HashMap<&str, u8> = c.topics.iter().map(|t| (t.id(), t.meta.level)).collect();

    for t in &c.topics {
        let m = &t.meta;
        let f = &t.file;
        if m.title.trim().is_empty() {
            d.push(Diag::error(f, "пустое title"));
        }
        if !section_ids.contains(m.section.as_str()) {
            d.push(Diag::error(
                f,
                format!("раздел `{}` не объявлен в sections.toml", m.section),
            ));
        }
        let dir = f
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("");
        if dir != m.section {
            d.push(Diag::error(f, format!("файл лежит в папке `{dir}`, а section = `{}`; папка должна совпадать с разделом", m.section)));
        }
        let stem = f.file_stem().and_then(|n| n.to_str()).unwrap_or("");
        if stem != m.id {
            d.push(Diag::error(
                f,
                format!("имя файла `{stem}.md` не совпадает с id `{}`", m.id),
            ));
        }
        if !(1..=3).contains(&m.level) {
            d.push(Diag::error(
                f,
                format!("level = {}: допустимо 1, 2 или 3", m.level),
            ));
        }
        if m.summary.trim().is_empty() {
            d.push(Diag::warning(f, "нет summary (строки «зачем это знать»)"));
        }
        let mut seen = HashSet::new();
        for n in &m.needs {
            if n == &m.id {
                d.push(Diag::error(f, "тема указана в собственных needs"));
            } else if !by_id.contains_key(n.as_str()) {
                d.push(Diag::error(f, format!("needs: темы `{n}` нет")));
            } else if levels[n.as_str()] > m.level {
                d.push(Diag::warning(
                    f,
                    format!(
                        "needs: `{n}` уровня {} выше уровня этой темы ({})",
                        levels[n.as_str()],
                        m.level
                    ),
                ));
            }
            if !seen.insert(n) {
                d.push(Diag::warning(f, format!("needs: `{n}` указана дважды")));
            }
        }
        if m.materials.is_empty() {
            d.push(Diag::error(f, "нет ни одного материала (materials)"));
        } else if !m.materials.iter().any(|x| x.start) {
            d.push(Diag::warning(
                f,
                "ни один материал не отмечен start: true («с чего начать»)",
            ));
        }
        let mut urls = HashSet::new();
        for (i, mat) in m.materials.iter().enumerate() {
            let n = i + 1;
            if mat.title.trim().is_empty() {
                d.push(Diag::error(f, format!("материал {n}: пустое title")));
            }
            if !is_http(&mat.url) {
                d.push(Diag::error(
                    f,
                    format!(
                        "материал {n} «{}»: url должен начинаться с https://",
                        mat.title
                    ),
                ));
            }
            if mat.alt_url.as_deref().is_some_and(|u| !is_http(u)) {
                d.push(Diag::error(
                    f,
                    format!(
                        "материал {n} «{}»: alt_url должен начинаться с https://",
                        mat.title
                    ),
                ));
            }
            if !urls.insert(&mat.url) {
                d.push(Diag::warning(
                    f,
                    format!("материал {n} «{}»: ссылка повторяется", mat.title),
                ));
            }
            for k in mat.extra.keys() {
                d.push(Diag::warning(
                    f,
                    format!("материал {n}: неизвестное поле `{k}` (не показывается)"),
                ));
            }
        }
        for k in m.extra.keys().filter(|k| !k.starts_with('_')) {
            d.push(Diag::warning(
                f,
                format!("неизвестное поле `{k}` в шапке (не показывается)"),
            ));
        }
        let r = markdown::render(&t.body, &titles, "");
        for bad in r.bad_links {
            // Like check.py: a missing topic is a warning (it may be written later, the link
            // renders as plain text); anything else (javascript:, images, ftp) is an error.
            if is_slug(bad.trim_start_matches('#')) {
                d.push(Diag::warning(
                    f,
                    format!("ссылка `{bad}` в тексте: такой темы нет (показана простым текстом)"),
                ));
            } else {
                d.push(Diag::error(
                    f,
                    format!("ссылка `{bad}` в тексте: для внешних ссылок нужен https://"),
                ));
            }
        }
    }

    if let Some(cycle) = find_cycle(c) {
        let file = by_id
            .get(cycle[0].as_str())
            .map(|p| p.to_path_buf())
            .unwrap_or_default();
        d.push(Diag::error(
            file,
            format!("цикл в needs: {}", cycle.join(" → ")),
        ));
    }

    // Routes
    let rf = &c.routes_file;
    let mut route_ids = HashSet::new();
    for r in &c.routes {
        if !is_slug(&r.id) {
            d.push(Diag::error(
                rf,
                format!(
                    "маршрут «{}»: id `{}` — только a-z, 0-9, - и _",
                    r.title, r.id
                ),
            ));
        }
        if !route_ids.insert(r.id.as_str()) {
            d.push(Diag::error(
                rf,
                format!("маршрут `{}` объявлен дважды", r.id),
            ));
        }
        if r.topics.is_empty() {
            d.push(Diag::error(
                rf,
                format!("маршрут `{}`: пустой список topics", r.id),
            ));
        }
        let mut seen = HashSet::new();
        for t in &r.topics {
            if !by_id.contains_key(t.as_str()) {
                d.push(Diag::error(
                    rf,
                    format!("маршрут `{}`: темы `{t}` нет", r.id),
                ));
            }
            if !seen.insert(t) {
                d.push(Diag::warning(
                    rf,
                    format!("маршрут `{}`: тема `{t}` повторяется", r.id),
                ));
            }
        }
    }
    d
}

/// The first cycle in the `needs` graph, as a path that ends where it starts.
pub fn find_cycle(c: &Content) -> Option<Vec<String>> {
    let needs: BTreeMap<&str, &[String]> = c
        .topics
        .iter()
        .map(|t| (t.id(), t.meta.needs.as_slice()))
        .collect();
    // 0 = new, 1 = on the stack, 2 = done
    let mut state: HashMap<&str, u8> = HashMap::new();
    let mut stack: Vec<&str> = Vec::new();

    fn visit<'a>(
        id: &'a str,
        needs: &BTreeMap<&'a str, &'a [String]>,
        state: &mut HashMap<&'a str, u8>,
        stack: &mut Vec<&'a str>,
    ) -> Option<Vec<String>> {
        match state.get(id).copied().unwrap_or(0) {
            1 => {
                let start = stack.iter().position(|s| *s == id).unwrap_or(0);
                let mut cyc: Vec<String> = stack[start..].iter().map(|s| s.to_string()).collect();
                cyc.push(id.to_string());
                return Some(cyc);
            }
            2 => return None,
            _ => {}
        }
        state.insert(id, 1);
        stack.push(id);
        for n in needs.get(id).copied().unwrap_or_default() {
            if needs.contains_key(n.as_str())
                && let Some(c) = visit(n.as_str(), needs, state, stack)
            {
                return Some(c);
            }
        }
        stack.pop();
        state.insert(id, 2);
        None
    }

    for id in needs.keys() {
        if let Some(c) = visit(id, &needs, &mut state, &mut stack) {
            return Some(c);
        }
    }
    None
}

pub fn has_errors(diags: &[Diag]) -> bool {
    diags.iter().any(|d| !d.warning)
}
