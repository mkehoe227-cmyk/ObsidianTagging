use anyhow::{Context, Result};
use std::fs;

pub fn read_note(path: &str) -> Result<(String, String)> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Cannot read note: {}", path))?;

    let (frontmatter, body) = split_frontmatter(&content);
    let title = extract_title(frontmatter, body, path);

    Ok((title, body.to_string()))
}

fn split_frontmatter(content: &str) -> (Option<&str>, &str) {
    if !content.starts_with("---") {
        return (None, content);
    }
    let rest = &content[3..];
    if let Some(end) = rest.find("\n---") {
        let fm = &rest[..end];
        let body_start = end + 4;
        let body = if body_start < rest.len() { &rest[body_start..] } else { "" };
        (Some(fm), body.trim_start_matches('\n'))
    } else {
        (None, content)
    }
}

fn extract_title(frontmatter: Option<&str>, body: &str, path: &str) -> String {
    if let Some(fm) = frontmatter {
        for line in fm.lines() {
            if let Some(rest) = line.strip_prefix("title:") {
                let t = rest.trim().trim_matches('"').trim_matches('\'').to_string();
                if !t.is_empty() {
                    return t;
                }
            }
        }
    }
    for line in body.lines() {
        if let Some(rest) = line.strip_prefix("# ") {
            return rest.trim().to_string();
        }
    }
    std::path::Path::new(path)
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn write_temp(content: &str) -> NamedTempFile {
        let mut f = NamedTempFile::new().unwrap();
        write!(f, "{}", content).unwrap();
        f
    }

    #[test]
    fn test_title_from_frontmatter() {
        let f = write_temp("---\ntitle: \"My Note\"\ntags: []\n---\n\nBody text here.");
        let (title, body) = read_note(f.path().to_str().unwrap()).unwrap();
        assert_eq!(title, "My Note");
        assert_eq!(body.trim(), "Body text here.");
    }

    #[test]
    fn test_title_from_heading() {
        let f = write_temp("---\ntags: []\n---\n\n# Great Heading\n\nBody.");
        let (title, _) = read_note(f.path().to_str().unwrap()).unwrap();
        assert_eq!(title, "Great Heading");
    }

    #[test]
    fn test_title_fallback_to_filename() {
        let f = write_temp("No frontmatter. No heading. Just body.");
        let (title, body) = read_note(f.path().to_str().unwrap()).unwrap();
        assert!(!title.is_empty());
        assert!(body.contains("Just body."));
    }

    #[test]
    fn test_no_frontmatter() {
        let f = write_temp("# Solo Heading\n\nJust a body.");
        let (title, body) = read_note(f.path().to_str().unwrap()).unwrap();
        assert_eq!(title, "Solo Heading");
        assert!(body.contains("Just a body."));
    }
}
