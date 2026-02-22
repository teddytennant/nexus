use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitRepoStats {
    pub name: String,
    pub path: PathBuf,
    pub last_commit_date: Option<String>,
    pub commit_count_30d: usize,
    pub primary_language: Option<String>,
    pub lines_changed_30d: usize,
}

/// Scan a directory for git repos (depth 1) and extract activity metrics.
pub fn scan_repos(dir: &Path) -> Vec<GitRepoStats> {
    let mut repos = Vec::new();

    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return repos,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() && path.join(".git").exists() {
            if let Some(stats) = scan_single_repo(&path) {
                repos.push(stats);
            }
        }
    }

    // Sort by commit_count_30d descending
    repos.sort_by(|a, b| b.commit_count_30d.cmp(&a.commit_count_30d));
    repos
}

fn scan_single_repo(repo_path: &Path) -> Option<GitRepoStats> {
    let name = repo_path
        .file_name()?
        .to_string_lossy()
        .to_string();

    let last_commit_date = get_last_commit_date(repo_path);
    let commit_count_30d = get_commit_count_30d(repo_path);
    let lines_changed_30d = get_lines_changed_30d(repo_path);
    let primary_language = detect_primary_language(repo_path);

    Some(GitRepoStats {
        name,
        path: repo_path.to_path_buf(),
        last_commit_date,
        commit_count_30d,
        primary_language,
        lines_changed_30d,
    })
}

fn get_last_commit_date(repo_path: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["log", "-1", "--format=%ci"])
        .current_dir(repo_path)
        .output()
        .ok()?;
    let date = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if date.is_empty() { None } else { Some(date) }
}

fn get_commit_count_30d(repo_path: &Path) -> usize {
    let output = Command::new("git")
        .args(["rev-list", "--count", "--since=30.days", "HEAD"])
        .current_dir(repo_path)
        .output();
    match output {
        Ok(o) => String::from_utf8_lossy(&o.stdout)
            .trim()
            .parse()
            .unwrap_or(0),
        Err(_) => 0,
    }
}

fn get_lines_changed_30d(repo_path: &Path) -> usize {
    let output = Command::new("git")
        .args(["log", "--since=30.days", "--pretty=tformat:", "--numstat"])
        .current_dir(repo_path)
        .output();
    match output {
        Ok(o) => {
            let text = String::from_utf8_lossy(&o.stdout);
            text.lines()
                .filter_map(|line| {
                    let parts: Vec<&str> = line.split('\t').collect();
                    if parts.len() >= 2 {
                        let added: usize = parts[0].parse().unwrap_or(0);
                        let deleted: usize = parts[1].parse().unwrap_or(0);
                        Some(added + deleted)
                    } else {
                        None
                    }
                })
                .sum()
        }
        Err(_) => 0,
    }
}

fn detect_primary_language(repo_path: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["log", "--since=90.days", "--pretty=tformat:", "--name-only"])
        .current_dir(repo_path)
        .output()
        .ok()?;

    let text = String::from_utf8_lossy(&output.stdout);
    let mut ext_counts: HashMap<String, usize> = HashMap::new();

    for line in text.lines() {
        if let Some(ext) = Path::new(line).extension() {
            let ext = ext.to_string_lossy().to_lowercase();
            *ext_counts.entry(ext).or_insert(0) += 1;
        }
    }

    let language_map: HashMap<&str, &str> = [
        ("rs", "Rust"),
        ("py", "Python"),
        ("ts", "TypeScript"),
        ("tsx", "TypeScript"),
        ("js", "JavaScript"),
        ("jsx", "JavaScript"),
        ("zig", "Zig"),
        ("c", "C"),
        ("cpp", "C++"),
        ("h", "C/C++"),
        ("swift", "Swift"),
        ("kt", "Kotlin"),
        ("java", "Java"),
        ("go", "Go"),
        ("nix", "Nix"),
        ("sh", "Shell"),
        ("md", "Markdown"),
    ]
    .into_iter()
    .collect();

    ext_counts
        .iter()
        .filter(|(ext, _)| language_map.contains_key(ext.as_str()))
        .max_by_key(|(_, count)| *count)
        .and_then(|(ext, _)| language_map.get(ext.as_str()).map(|s| s.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn create_test_repo(dir: &Path) {
        Command::new("git")
            .args(["init"])
            .current_dir(dir)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(dir)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(dir)
            .output()
            .unwrap();
    }

    // === AC6.1: Finds repos by .git directories ===
    #[test]
    fn test_scan_finds_git_repos() {
        let tmp = tempfile::TempDir::new().unwrap();
        let repo_a = tmp.path().join("repo-a");
        let repo_b = tmp.path().join("repo-b");
        let not_repo = tmp.path().join("not-a-repo");
        fs::create_dir_all(&repo_a).unwrap();
        fs::create_dir_all(&repo_b).unwrap();
        fs::create_dir_all(&not_repo).unwrap();

        create_test_repo(&repo_a);
        create_test_repo(&repo_b);
        // not_repo has no .git

        let repos = scan_repos(tmp.path());
        let names: Vec<&str> = repos.iter().map(|r| r.name.as_str()).collect();
        assert!(names.contains(&"repo-a"));
        assert!(names.contains(&"repo-b"));
        assert!(!names.contains(&"not-a-repo"));
    }

    // === AC6.2: Extracts last commit date ===
    #[test]
    fn test_last_commit_date() {
        let tmp = tempfile::TempDir::new().unwrap();
        let repo = tmp.path().join("myrepo");
        fs::create_dir_all(&repo).unwrap();
        create_test_repo(&repo);

        // Create a commit
        fs::write(repo.join("test.rs"), "fn main() {}").unwrap();
        Command::new("git").args(["add", "."]).current_dir(&repo).output().unwrap();
        Command::new("git").args(["commit", "-m", "init"]).current_dir(&repo).output().unwrap();

        let date = get_last_commit_date(&repo);
        assert!(date.is_some());
    }

    // === AC6.3: Counts commits in last 30 days ===
    #[test]
    fn test_commit_count_30d() {
        let tmp = tempfile::TempDir::new().unwrap();
        let repo = tmp.path().join("myrepo");
        fs::create_dir_all(&repo).unwrap();
        create_test_repo(&repo);

        fs::write(repo.join("test.rs"), "fn main() {}").unwrap();
        Command::new("git").args(["add", "."]).current_dir(&repo).output().unwrap();
        Command::new("git").args(["commit", "-m", "first"]).current_dir(&repo).output().unwrap();

        fs::write(repo.join("test2.rs"), "fn foo() {}").unwrap();
        Command::new("git").args(["add", "."]).current_dir(&repo).output().unwrap();
        Command::new("git").args(["commit", "-m", "second"]).current_dir(&repo).output().unwrap();

        let count = get_commit_count_30d(&repo);
        assert_eq!(count, 2);
    }

    // === AC6.5: Detects primary language ===
    #[test]
    fn test_detect_primary_language() {
        let tmp = tempfile::TempDir::new().unwrap();
        let repo = tmp.path().join("myrepo");
        fs::create_dir_all(&repo).unwrap();
        create_test_repo(&repo);

        fs::write(repo.join("main.rs"), "fn main() {}").unwrap();
        fs::write(repo.join("lib.rs"), "pub fn foo() {}").unwrap();
        fs::write(repo.join("readme.md"), "# Hello").unwrap();
        Command::new("git").args(["add", "."]).current_dir(&repo).output().unwrap();
        Command::new("git").args(["commit", "-m", "init"]).current_dir(&repo).output().unwrap();

        let lang = detect_primary_language(&repo);
        assert_eq!(lang, Some("Rust".to_string()));
    }

    // === Edge case: Empty directory ===
    #[test]
    fn test_scan_empty_dir() {
        let tmp = tempfile::TempDir::new().unwrap();
        let repos = scan_repos(tmp.path());
        assert!(repos.is_empty());
    }

    // === Edge case: Non-existent directory ===
    #[test]
    fn test_scan_nonexistent_dir() {
        let repos = scan_repos(Path::new("/nonexistent/path"));
        assert!(repos.is_empty());
    }
}
