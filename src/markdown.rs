//! Markdown body of a topic → HTML, with links between topics resolved.
//!
//! A link to another topic may be written as `[[id]]`, `[[id|текст]]`, `[текст](id)`,
//! `[текст](#topic/id)` or `[текст](topic:id)`. Raw HTML in the body is shown as text,
//! links other than http(s)/mailto are dropped: the content may come from pull requests.

use pulldown_cmark::{CowStr, Event, HeadingLevel, LinkType, Options, Parser, Tag, TagEnd};
use std::collections::HashMap;

pub struct Rendered {
    pub html: String,
    /// Topic ids linked under a «Дальше» heading, in order.
    pub next: Vec<String>,
    /// Every topic id linked from the body, in order.
    pub links: Vec<String>,
    /// Link targets that are neither a known topic nor an allowed URL.
    pub bad_links: Vec<String>,
}

pub fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}

/// The topic id a link target points to, if it is written as one.
fn topic_target(dest: &str) -> Option<&str> {
    let id = dest
        .strip_prefix("#topic/")
        .or_else(|| dest.strip_prefix("topic:"))
        .unwrap_or(dest);
    let id = id.trim_end_matches('/');
    let ok = !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    ok.then_some(id)
}

fn is_safe_url(dest: &str) -> bool {
    let lower = dest.to_ascii_lowercase();
    lower.starts_with("https://") || lower.starts_with("http://") || lower.starts_with("mailto:")
}

/// Normalises a heading for matching: lowercase, ё→е, letters only.
pub fn heading_key(s: &str) -> String {
    s.to_lowercase()
        .replace('ё', "е")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == ' ')
        .collect::<String>()
        .trim()
        .to_string()
}

/// Renders `md`. `titles` maps topic ids to titles; `root` is the relative path to the site root.
pub fn render(md: &str, titles: &HashMap<String, String>, root: &str) -> Rendered {
    let opts = Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_SMART_PUNCTUATION
        | Options::ENABLE_WIKILINKS;
    let mut next = Vec::new();
    let mut links = Vec::new();
    let mut bad_links = Vec::new();
    let mut events: Vec<Event> = Vec::new();
    let mut in_h2 = false;
    let mut h2_text = String::new();
    let mut section = String::new();
    // For each open link: Some(title) when its text must be replaced by the topic title.
    let mut link_stack: Vec<(bool, Option<String>)> = Vec::new();

    for ev in Parser::new_ext(md, opts) {
        match ev {
            Event::Start(Tag::Heading {
                level: HeadingLevel::H2,
                ..
            }) => {
                in_h2 = true;
                h2_text.clear();
                events.push(ev);
            }
            Event::End(TagEnd::Heading(HeadingLevel::H2)) => {
                in_h2 = false;
                section = heading_key(&h2_text);
                events.push(ev);
            }
            Event::Start(Tag::Link {
                link_type,
                dest_url,
                ..
            }) => {
                let dest = dest_url.to_string();
                if let Some(id) = topic_target(&dest).filter(|id| titles.contains_key(*id)) {
                    links.push(id.to_string());
                    if section.starts_with("дальше") {
                        next.push(id.to_string());
                    }
                    let replace = matches!(link_type, LinkType::WikiLink { has_pothole: false })
                        .then(|| titles[id].clone());
                    events.push(Event::Html(CowStr::from(format!(
                        "<a class=\"tlink\" href=\"{root}topic/{id}/\" data-topic=\"{id}\">",
                        id = escape(id)
                    ))));
                    link_stack.push((true, replace));
                } else if is_safe_url(&dest) {
                    events.push(Event::Html(CowStr::from(format!(
                        "<a href=\"{}\" target=\"_blank\" rel=\"noopener noreferrer\">",
                        escape(&dest)
                    ))));
                    link_stack.push((true, None));
                } else {
                    bad_links.push(dest);
                    link_stack.push((false, None));
                }
            }
            Event::End(TagEnd::Link) => {
                if let Some((open, _)) = link_stack.pop()
                    && open
                {
                    events.push(Event::Html(CowStr::from("</a>")));
                }
            }
            Event::Text(t) => {
                if in_h2 {
                    h2_text.push_str(&t);
                }
                // `[[id]]`: the id text is replaced by the topic title.
                match link_stack
                    .last_mut()
                    .and_then(|(_, replace)| replace.take())
                {
                    Some(title) => events.push(Event::Text(CowStr::from(title))),
                    None => events.push(Event::Text(t)),
                }
            }
            Event::Html(h) | Event::InlineHtml(h) => events.push(Event::Text(h)),
            Event::Start(Tag::Image { dest_url, .. }) => {
                // Images are not supported in topic bodies (only links to materials).
                bad_links.push(dest_url.to_string());
                events.push(Event::Start(Tag::Emphasis));
            }
            Event::End(TagEnd::Image) => events.push(Event::End(TagEnd::Emphasis)),
            other => events.push(other),
        }
    }

    let mut html = String::new();
    pulldown_cmark::html::push_html(&mut html, events.into_iter());
    Rendered {
        html,
        next,
        links,
        bad_links,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn titles() -> HashMap<String, String> {
        HashMap::from([
            ("creed".to_string(), "Символ веры".to_string()),
            ("prayer".to_string(), "Молитва".to_string()),
        ])
    }

    #[test]
    fn wiki_links_get_titles_and_next_is_collected() {
        let md = "Вступление [[creed]].\n\n## Дальше\n\n- [[prayer]]\n- [другое](creed)\n";
        let r = render(md, &titles(), "../../");
        assert!(r.html.contains("href=\"../../topic/creed/\""), "{}", r.html);
        assert!(r.html.contains(">Символ веры</a>"), "{}", r.html);
        assert!(r.html.contains(">Молитва</a>"), "{}", r.html);
        assert!(r.html.contains(">другое</a>"), "{}", r.html);
        assert_eq!(r.next, vec!["prayer", "creed"]);
        assert!(r.bad_links.is_empty());
    }

    #[test]
    fn piped_wiki_link_keeps_text() {
        let r = render("[[creed|Символ]]", &titles(), "");
        assert!(r.html.contains(">Символ</a>"), "{}", r.html);
    }

    #[test]
    fn external_links_open_in_new_tab_and_unsafe_are_dropped() {
        let md = "[a](https://azbyka.ru/) [b](javascript:alert(1)) [c](missing-topic) <script>x</script>";
        let r = render(md, &titles(), "");
        assert!(
            r.html
                .contains("target=\"_blank\" rel=\"noopener noreferrer\"")
        );
        assert!(!r.html.contains("javascript:"));
        assert!(!r.html.contains("<script>"));
        assert!(r.html.contains("&lt;script&gt;"));
        assert_eq!(r.bad_links, vec!["javascript:alert(1)", "missing-topic"]);
    }

    #[test]
    fn hash_and_scheme_forms_resolve() {
        let r = render("[x](#topic/creed) [y](topic:prayer)", &titles(), "./");
        assert_eq!(r.links, vec!["creed", "prayer"]);
    }
}
