//! Content model: sections, routes and topics as they are written in `content/`.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

/// One problem found in the content, tied to the file it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diag {
    pub file: PathBuf,
    pub message: String,
    pub warning: bool,
}

impl Diag {
    pub fn error(file: impl Into<PathBuf>, message: impl Into<String>) -> Self {
        Self {
            file: file.into(),
            message: message.into(),
            warning: false,
        }
    }
    pub fn warning(file: impl Into<PathBuf>, message: impl Into<String>) -> Self {
        Self {
            file: file.into(),
            message: message.into(),
            warning: true,
        }
    }
}

impl fmt::Display for Diag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let kind = if self.warning {
            "предупреждение"
        } else {
            "ошибка"
        };
        write!(f, "{kind}: {}: {}", self.file.display(), self.message)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Section {
    pub id: String,
    pub title: String,
    pub order: i64,
    pub color: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub icon: String,
}

#[derive(Debug, Deserialize)]
pub struct SectionsFile {
    #[serde(default)]
    pub section: Vec<Section>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Route {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    pub topics: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct RoutesFile {
    #[serde(default)]
    pub route: Vec<Route>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MaterialType {
    Video,
    Playlist,
    Book,
    Article,
    Course,
    Audio,
}

impl MaterialType {
    pub const ALL: [MaterialType; 6] = [
        MaterialType::Book,
        MaterialType::Article,
        MaterialType::Video,
        MaterialType::Playlist,
        MaterialType::Course,
        MaterialType::Audio,
    ];

    pub fn key(self) -> &'static str {
        match self {
            MaterialType::Video => "video",
            MaterialType::Playlist => "playlist",
            MaterialType::Book => "book",
            MaterialType::Article => "article",
            MaterialType::Course => "course",
            MaterialType::Audio => "audio",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            MaterialType::Video => "Видео",
            MaterialType::Playlist => "Плейлист",
            MaterialType::Book => "Книга",
            MaterialType::Article => "Статья",
            MaterialType::Course => "Курс",
            MaterialType::Audio => "Аудио",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            MaterialType::Video => "▶",
            MaterialType::Playlist => "☰",
            MaterialType::Book => "📖",
            MaterialType::Article => "📄",
            MaterialType::Course => "🎓",
            MaterialType::Audio => "🎧",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    Ru,
    En,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Material {
    pub title: String,
    #[serde(default)]
    pub author: String,
    #[serde(rename = "type")]
    pub kind: MaterialType,
    pub url: String,
    pub lang: Lang,
    #[serde(default)]
    pub time: String,
    #[serde(default)]
    pub start: bool,
    #[serde(default)]
    pub note: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alt_url: Option<String>,
    /// Fields outside the schema; reported as warnings, never rendered.
    #[serde(flatten, skip_serializing)]
    pub extra: BTreeMap<String, serde_yaml::Value>,
}

/// The YAML frontmatter of a topic file.
#[derive(Debug, Clone, Deserialize)]
pub struct Frontmatter {
    pub id: String,
    pub title: String,
    pub section: String,
    pub level: u8,
    #[serde(default)]
    pub needs: Vec<String>,
    #[serde(default)]
    pub order: i64,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub time: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub materials: Vec<Material>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_yaml::Value>,
}

#[derive(Debug, Clone)]
pub struct Topic {
    pub meta: Frontmatter,
    /// Markdown body after the frontmatter.
    pub body: String,
    /// Path of the source file, for messages.
    pub file: PathBuf,
}

impl Topic {
    pub fn id(&self) -> &str {
        &self.meta.id
    }
}

/// Everything read from `content/`, before validation.
#[derive(Debug, Clone, Default)]
pub struct Content {
    pub sections: Vec<Section>,
    pub routes: Vec<Route>,
    pub topics: Vec<Topic>,
    pub sections_file: PathBuf,
    pub routes_file: PathBuf,
}

/// Splits `---\nyaml\n---\nbody` into (yaml, body).
pub fn split_frontmatter(text: &str) -> Option<(&str, &str)> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let rest = text.strip_prefix("---")?;
    let rest = rest
        .strip_prefix("\r\n")
        .or_else(|| rest.strip_prefix('\n'))?;
    let mut pos = 0;
    for line in rest.split_inclusive('\n') {
        if line.trim_end() == "---" {
            return Some((&rest[..pos], &rest[pos + line.len()..]));
        }
        pos += line.len();
    }
    if rest[pos..].trim_end() == "---" {
        return Some((&rest[..pos], ""));
    }
    None
}

pub fn parse_topic(file: &Path, text: &str) -> Result<Topic, Diag> {
    let (yaml, body) = split_frontmatter(text).ok_or_else(|| {
        Diag::error(
            file,
            "нет шапки YAML: файл должен начинаться с `---`, шапка закрывается строкой `---`",
        )
    })?;
    let meta: Frontmatter = serde_yaml::from_str(yaml).map_err(|e| {
        let text = e.to_string();
        let msg = text
            .rfind(" at line ")
            .map_or(text.as_str(), |i| &text[..i]);
        // +1: the opening `---` line comes before the YAML.
        let at = e
            .location()
            .map(|l| format!("строка {}: ", l.line() + 1))
            .unwrap_or_default();
        let hint = if msg.contains("mapping values") {
            " (в значении есть «: »? возьмите его в кавычки: title: \"Молитва: как начать\")"
        } else {
            ""
        };
        Diag::error(file, format!("{at}шапка YAML не читается: {msg}{hint}"))
    })?;
    Ok(Topic {
        meta,
        body: body.to_string(),
        file: file.to_path_buf(),
    })
}

fn read(path: &Path) -> Result<String, Diag> {
    std::fs::read_to_string(path).map_err(|e| Diag::error(path, format!("не читается: {e}")))
}

/// Reads `content/` into memory. Collects every problem instead of stopping at the first.
pub fn load(dir: &Path) -> (Content, Vec<Diag>) {
    let mut diags = Vec::new();
    let mut content = Content {
        sections_file: dir.join("sections.toml"),
        routes_file: dir.join("routes.toml"),
        ..Content::default()
    };

    match read(&content.sections_file) {
        Ok(text) => match toml::from_str::<SectionsFile>(&text) {
            Ok(f) => content.sections = f.section,
            Err(e) => diags.push(Diag::error(&content.sections_file, e.to_string())),
        },
        Err(d) => diags.push(d),
    }

    if content.routes_file.exists() {
        match read(&content.routes_file) {
            Ok(text) => match toml::from_str::<RoutesFile>(&text) {
                Ok(f) => content.routes = f.route,
                Err(e) => diags.push(Diag::error(&content.routes_file, e.to_string())),
            },
            Err(d) => diags.push(d),
        }
    }

    let topics_dir = dir.join("topics");
    let mut files = Vec::new();
    collect_md(&topics_dir, &mut files, &mut diags);
    files.sort();
    for file in files {
        match read(&file).and_then(|text| parse_topic(&file, &text)) {
            Ok(t) => content.topics.push(t),
            Err(d) => diags.push(d),
        }
    }
    (content, diags)
}

fn collect_md(dir: &Path, out: &mut Vec<PathBuf>, diags: &mut Vec<Diag>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            diags.push(Diag::error(dir, format!("папка не читается: {e}")));
            return;
        }
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_md(&path, out, diags);
        } else if path.extension().is_some_and(|e| e == "md") {
            out.push(path);
        }
    }
}
