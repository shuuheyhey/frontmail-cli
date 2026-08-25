use std::{
    fs,
    path::{Path, PathBuf},
};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn required_community_files_exist() {
    for path in [
        "CHANGELOG.md",
        "CODE_OF_CONDUCT.md",
        "CONTRIBUTING.md",
        "SECURITY.md",
        "SUPPORT.md",
        ".github/ISSUE_TEMPLATE/bug_report.yml",
        ".github/ISSUE_TEMPLATE/feature_request.yml",
        ".github/ISSUE_TEMPLATE/config.yml",
        ".github/pull_request_template.md",
    ] {
        assert!(root().join(path).is_file(), "missing {path}");
    }
}

#[test]
fn agent_instructions_and_front_skill_exist() {
    let skill = root().join("skills/front/SKILL.md");
    assert!(skill.is_file(), "missing skills/front/SKILL.md");

    let claude = root().join("CLAUDE.md");
    let metadata = fs::symlink_metadata(&claude).expect("missing CLAUDE.md symlink");
    if metadata.file_type().is_symlink() {
        assert_eq!(fs::read_link(claude).unwrap(), PathBuf::from("AGENTS.md"));
    } else {
        #[cfg(target_os = "windows")]
        assert_eq!(fs::read_to_string(claude).unwrap(), "AGENTS.md");
        #[cfg(not(target_os = "windows"))]
        panic!("CLAUDE.md is not a symlink");
    }
}

#[test]
fn github_yaml_is_valid() {
    for path in github_yaml_files(&root().join(".github")) {
        let source = fs::read_to_string(&path).unwrap();
        serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&source)
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    }
}

#[test]
fn public_documentation_layout_exists() {
    for path in [
        "docs/README.md",
        "docs/getting-started.md",
        "docs/configuration.md",
        "docs/commands.md",
        "docs/output-format.md",
        "docs/api-support.md",
        "docs/architecture.md",
        "docs/development/designs/2026-08-25-rust-rewrite-design.md",
        "docs/development/plans/2026-08-25-rust-rewrite.md",
    ] {
        assert!(root().join(path).is_file(), "missing {path}");
    }
    assert!(!root().join("docs/superpowers").exists());
}

#[test]
fn markdown_relative_links_resolve() {
    let repository = root();
    let mut files = root_markdown_files(&repository);
    collect_markdown_files(&repository.join("docs"), &mut files);
    collect_markdown_files(&repository.join("skills"), &mut files);
    files.sort();

    let mut missing = Vec::new();
    for source_path in files {
        let source = fs::read_to_string(&source_path).unwrap();
        let mut in_fence = false;
        for line in source.lines() {
            if line.trim_start().starts_with("```") {
                in_fence = !in_fence;
                continue;
            }
            if in_fence {
                continue;
            }
            for tail in line.split("](").skip(1) {
                let Some(end) = tail.find(')') else {
                    continue;
                };
                let raw = tail[..end].trim();
                let destination = raw
                    .strip_prefix('<')
                    .and_then(|value| value.strip_suffix('>'))
                    .unwrap_or(raw)
                    .split_whitespace()
                    .next()
                    .unwrap_or_default();
                if destination.is_empty()
                    || destination.starts_with('#')
                    || destination.starts_with("http://")
                    || destination.starts_with("https://")
                    || destination.starts_with("mailto:")
                {
                    continue;
                }
                let path = destination.split('#').next().unwrap();
                let resolved = source_path.parent().unwrap().join(path);
                if !resolved.exists() {
                    missing.push(format!(
                        "{} -> {destination}",
                        source_path.strip_prefix(&repository).unwrap().display()
                    ));
                }
            }
        }
    }
    assert!(
        missing.is_empty(),
        "missing Markdown links:\n{}",
        missing.join("\n")
    );
}

#[test]
fn cargo_manifest_has_public_metadata() {
    let cargo = fs::read_to_string(root().join("Cargo.toml")).unwrap();
    assert!(cargo.contains("repository = \"https://github.com/shuuheyhey/frontmail-cli\""));
    assert!(cargo.contains("readme = \"README.md\""));
    assert!(cargo.contains("categories = [\"command-line-utilities\"]"));
}

#[test]
fn ci_uses_locked_read_only_checks() {
    let ci = fs::read_to_string(root().join(".github/workflows/ci.yml")).unwrap();
    assert!(ci.contains("contents: read"));
    for command in [
        "cargo fmt --all -- --check",
        "cargo clippy --locked --all-targets --all-features -- -D warnings",
        "cargo test --locked --all-targets",
        "cargo build --locked --release",
    ] {
        assert!(ci.contains(command), "CI missing {command}");
    }
}

fn github_yaml_files(directory: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_yaml_files(directory, &mut files);
    files.sort();
    files
}

fn collect_yaml_files(directory: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect_yaml_files(&path, files);
        } else if matches!(
            path.extension().and_then(|value| value.to_str()),
            Some("yml" | "yaml")
        ) {
            files.push(path);
        }
    }
}

fn root_markdown_files(repository: &Path) -> Vec<PathBuf> {
    fs::read_dir(repository)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.is_file() && path.extension().is_some_and(|value| value == "md"))
        .collect()
}

fn collect_markdown_files(directory: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect_markdown_files(&path, files);
        } else if path.extension().is_some_and(|value| value == "md") {
            files.push(path);
        }
    }
}
