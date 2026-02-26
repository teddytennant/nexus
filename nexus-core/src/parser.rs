use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

static TAG_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"(?:^|[\s,(])#([a-zA-Z][a-zA-Z0-9_/-]*)").unwrap());

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub id: String,
    pub title: String,
    pub path: PathBuf,
    pub outgoing_links: Vec<String>,
    pub tags: Vec<String>,
    pub word_count: usize,
    pub directory: String,
}

/// Normalize a string to a lowercase kebab-case ID.
pub fn normalize_id(s: &str) -> String {
    s.trim()
        .to_lowercase()
        .replace(' ', "-")
}

/// Extract wikilinks from markdown content, ignoring those inside code blocks.
pub fn extract_wikilinks(content: &str) -> Vec<String> {
    let mut links = Vec::new();
    let mut seen = HashSet::new();
    let mut in_code_block = false;

    for line in content.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            in_code_block = !in_code_block;
            continue;
        }
        if in_code_block {
            continue;
        }

        // Find all [[...]] patterns in the line, skipping inline code
        let chars: Vec<char> = line.chars().collect();
        let len = chars.len();
        let mut i = 0;
        let mut in_inline_code = false;

        while i < len {
            if chars[i] == '`' {
                in_inline_code = !in_inline_code;
                i += 1;
                continue;
            }
            if in_inline_code {
                i += 1;
                continue;
            }
            if i + 1 < len && chars[i] == '[' && chars[i + 1] == '[' {
                // Find closing ]]
                let start = i + 2;
                let mut end = None;
                let mut j = start;
                while j + 1 < len {
                    if chars[j] == ']' && chars[j + 1] == ']' {
                        end = Some(j);
                        break;
                    }
                    j += 1;
                }
                if let Some(end_pos) = end {
                    let inner: String = chars[start..end_pos].iter().collect();
                    // Handle [[link|alias]] — take the part before |
                    let target = inner.split('|').next().unwrap_or(&inner);
                    let normalized = normalize_id(target);
                    if !normalized.is_empty() && seen.insert(normalized.clone()) {
                        links.push(normalized);
                    }
                    i = end_pos + 2;
                } else {
                    i += 1;
                }
            } else {
                i += 1;
            }
        }
    }

    links
}

/// Extract #tags from markdown content, ignoring those inside code blocks and headings.
pub fn extract_tags(content: &str) -> Vec<String> {
    let mut tags = Vec::new();
    let mut seen = HashSet::new();
    let mut in_code_block = false;

    for line in content.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            in_code_block = !in_code_block;
            continue;
        }
        if in_code_block {
            continue;
        }
        // Skip heading lines (# Heading, ## Heading, etc.)
        if trimmed.starts_with('#') {
            let after_hashes = trimmed.trim_start_matches('#');
            if after_hashes.starts_with(' ') || after_hashes.is_empty() {
                continue;
            }
        }

        for cap in TAG_RE.captures_iter(line) {
            let tag = cap[1].to_lowercase();
            if seen.insert(tag.clone()) {
                tags.push(tag);
            }
        }
    }

    tags
}

/// Extract the title from markdown content (first H1 heading, or None).
pub fn extract_title(content: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(title) = trimmed.strip_prefix("# ") {
            let title = title.trim();
            if !title.is_empty() {
                return Some(title.to_string());
            }
        }
    }
    None
}

/// Count words in content (simple whitespace split).
pub fn word_count(content: &str) -> usize {
    content.split_whitespace().count()
}

/// Parse a single markdown file into a Note.
pub fn parse_note(path: &Path) -> std::io::Result<Note> {
    let content = std::fs::read_to_string(path)?;
    let filename = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let id = normalize_id(&filename);
    let title = extract_title(&content).unwrap_or_else(|| filename.clone());
    let directory = path
        .parent()
        .and_then(|p| p.file_name())
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();

    Ok(Note {
        id,
        title,
        path: path.to_path_buf(),
        outgoing_links: extract_wikilinks(&content),
        tags: extract_tags(&content),
        word_count: word_count(&content),
        directory,
    })
}

/// Parse all markdown files in a vault directory.
pub fn parse_vault(vault_path: &Path) -> std::io::Result<Vec<Note>> {
    let mut notes = Vec::new();
    for entry in walkdir::WalkDir::new(vault_path)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.extension().map_or(false, |ext| ext == "md") {
            match parse_note(path) {
                Ok(note) => notes.push(note),
                Err(_) => continue,
            }
        }
    }
    Ok(notes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::TempDir;

    // Helper to create a temp vault with files
    fn create_temp_vault(files: &[(&str, &str)]) -> TempDir {
        let dir = TempDir::new().unwrap();
        for (name, content) in files {
            let path = dir.path().join(name);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            let mut f = fs::File::create(&path).unwrap();
            f.write_all(content.as_bytes()).unwrap();
        }
        dir
    }

    // === AC1.1: Parses [[simple-link]] correctly ===
    #[test]
    fn test_extract_simple_wikilink() {
        let content = "This links to [[gradient]] and [[building]].";
        let links = extract_wikilinks(content);
        assert_eq!(links, vec!["gradient", "building"]);
    }

    // === AC1.2: Parses [[link|display text]] extracting "link" as the target ===
    #[test]
    fn test_extract_aliased_wikilink() {
        let content = "See [[housing|the housing crisis]] for details.";
        let links = extract_wikilinks(content);
        assert_eq!(links, vec!["housing"]);
    }

    // === AC1.3: Ignores wikilinks inside code blocks ===
    #[test]
    fn test_ignore_wikilinks_in_fenced_code_block() {
        let content = "Before\n```\n[[ignored-link]]\n```\nAfter [[real-link]]";
        let links = extract_wikilinks(content);
        assert_eq!(links, vec!["real-link"]);
    }

    #[test]
    fn test_ignore_wikilinks_in_inline_code() {
        let content = "Use `[[not-a-link]]` but [[real-link]] is real.";
        let links = extract_wikilinks(content);
        assert_eq!(links, vec!["real-link"]);
    }

    // === AC1.4: Extracts #tags but not # headings ===
    #[test]
    fn test_extract_tags_not_headings() {
        let content = "# My Heading\n\nSome text with #philosophy and #ai-safety tags.";
        let tags = extract_tags(content);
        assert_eq!(tags, vec!["philosophy", "ai-safety"]);
    }

    #[test]
    fn test_tags_ignored_in_code_blocks() {
        let content = "```\n#not-a-tag\n```\n#real-tag";
        let tags = extract_tags(content);
        assert_eq!(tags, vec!["real-tag"]);
    }

    // === AC1.5: Handles files with no links gracefully ===
    #[test]
    fn test_note_with_no_links() {
        let content = "Just some plain text with no links at all.";
        let links = extract_wikilinks(content);
        assert!(links.is_empty());
    }

    // === AC1.6: Normalizes link targets to lowercase kebab-case ===
    #[test]
    fn test_normalize_id() {
        assert_eq!(normalize_id("The Gradient"), "the-gradient");
        assert_eq!(normalize_id("AI Safety"), "ai-safety");
        assert_eq!(normalize_id("  spaces  "), "spaces");
    }

    #[test]
    fn test_wikilink_normalization() {
        let content = "[[The Gradient]] and [[AI Safety]]";
        let links = extract_wikilinks(content);
        assert_eq!(links, vec!["the-gradient", "ai-safety"]);
    }

    // === Deduplication ===
    #[test]
    fn test_duplicate_links_deduplicated() {
        let content = "[[gradient]] mentioned twice [[gradient]].";
        let links = extract_wikilinks(content);
        assert_eq!(links, vec!["gradient"]);
    }

    // === Title extraction ===
    #[test]
    fn test_extract_title() {
        let content = "# The Gradient\n\nSome content here.";
        assert_eq!(extract_title(content), Some("The Gradient".to_string()));
    }

    #[test]
    fn test_extract_title_none() {
        let content = "No heading here, just text.";
        assert_eq!(extract_title(content), None);
    }

    // === Word count ===
    #[test]
    fn test_word_count() {
        assert_eq!(word_count("hello world foo bar"), 4);
        assert_eq!(word_count(""), 0);
    }

    // === Full vault parsing ===
    #[test]
    fn test_parse_vault() {
        let vault = create_temp_vault(&[
            ("philosophy/gradient.md", "# The Gradient\n\nLinks to [[building]] and [[thermodynamics]].\n\n#philosophy #core"),
            ("philosophy/building.md", "# Building\n\nSee [[gradient]] for why.\n\n#philosophy"),
            ("science/thermodynamics.md", "# Thermodynamics\n\nFoundation of [[gradient]]."),
            ("orphan.md", "# Orphan Note\n\nNo links here."),
        ]);

        let notes = parse_vault(vault.path()).unwrap();
        assert_eq!(notes.len(), 4);

        let gradient = notes.iter().find(|n| n.id == "gradient").unwrap();
        assert_eq!(gradient.title, "The Gradient");
        assert_eq!(gradient.outgoing_links, vec!["building", "thermodynamics"]);
        assert!(gradient.tags.contains(&"philosophy".to_string()));
        assert!(gradient.tags.contains(&"core".to_string()));

        let orphan = notes.iter().find(|n| n.id == "orphan").unwrap();
        assert!(orphan.outgoing_links.is_empty());
    }

    #[test]
    fn test_parse_note_directory() {
        let vault = create_temp_vault(&[
            ("ideas/philosophy/gradient.md", "# The Gradient"),
        ]);
        let notes = parse_vault(vault.path()).unwrap();
        let gradient = notes.iter().find(|n| n.id == "gradient").unwrap();
        assert_eq!(gradient.directory, "philosophy");
    }
}
