//! A small local web server for previewing `dist/`. Not for production.

use anyhow::{Result, anyhow};
use std::path::{Component, Path, PathBuf};

fn mime(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()).unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "webmanifest" => "application/manifest+json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "xml" => "application/xml; charset=utf-8",
        "txt" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

/// Maps a request path to a file under `root`; `base` (e.g. `/orthodox-map/`) is stripped when present,
/// so links built for GitHub Pages work locally too. `None` for anything outside `root`.
pub fn resolve(root: &Path, base: &str, url: &str) -> Option<PathBuf> {
    let path = url.split(['?', '#']).next().unwrap_or("");
    let path = percent_decode(path)?;
    let path = if base != "/" {
        path.strip_prefix(base.trim_end_matches('/'))
            .unwrap_or(&path)
    } else {
        &path
    };
    let mut out = root.to_path_buf();
    for c in Path::new(path.trim_start_matches('/')).components() {
        match c {
            Component::Normal(p) => out.push(p),
            Component::CurDir => {}
            _ => return None,
        }
    }
    if out.is_dir() {
        out.push("index.html");
    }
    Some(out)
}

fn percent_decode(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = s.get(i + 1..i + 3)?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

pub fn serve(root: &Path, base: &str, addr: &str) -> Result<()> {
    let server =
        tiny_http::Server::http(addr).map_err(|e| anyhow!("не удалось открыть {addr}: {e}"))?;
    for req in server.incoming_requests() {
        let file = resolve(root, base, req.url());
        let (status, path) = match file {
            Some(p) if p.is_file() => (200, p),
            _ => (404, root.join("404.html")),
        };
        let body = std::fs::read(&path).unwrap_or_default();
        let ct = tiny_http::Header::from_bytes("Content-Type", mime(&path)).expect("header");
        let nc = tiny_http::Header::from_bytes("Cache-Control", "no-cache").expect("header");
        let resp = tiny_http::Response::from_data(body)
            .with_status_code(status)
            .with_header(ct)
            .with_header(nc);
        let _ = req.respond(resp);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_blocks_traversal_and_strips_base() {
        let root = Path::new("/srv/dist");
        assert_eq!(
            resolve(root, "/", "/data.json"),
            Some(PathBuf::from("/srv/dist/data.json"))
        );
        assert_eq!(
            resolve(root, "/orthodox-map/", "/orthodox-map/data.json?v=1"),
            Some(PathBuf::from("/srv/dist/data.json"))
        );
        assert_eq!(resolve(root, "/", "/../etc/passwd"), None);
        assert_eq!(resolve(root, "/", "/%2e%2e/etc/passwd"), None);
    }
}
