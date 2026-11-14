//! Tests of the condition under which an acceptance case may expect an empty
//! population (see `acceptance/population.rs`).

#[path = "acceptance/population.rs"]
mod population;

use population::population_exists;
use std::path::PathBuf;

fn tree(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let root = std::env::temp_dir().join(format!("urzua-population-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join(".changeset")).unwrap();
    for (file, text) in files {
        std::fs::write(root.join(".changeset").join(file), text).unwrap();
    }
    root
}

fn exists(root: &PathBuf, key: Option<&str>) -> bool {
    let found = population_exists(root, ".changeset", "md", key);
    let _ = std::fs::remove_dir_all(root);
    found
}

#[test]
fn a_fragment_that_closes_nothing_is_not_a_population() {
    let root = tree(
        "empty-closes",
        &[("a.md", "---\ndefault: patch\ncloses: []\n---\n\nBody.\n")],
    );
    assert!(!exists(&root, Some("closes")));
}

#[test]
fn a_fragment_that_closes_a_record_is_a_population() {
    let root = tree(
        "closes-one",
        &[(
            "a.md",
            "---\ndefault: patch\ncloses: [BUG-1]\n---\n\nBody.\n",
        )],
    );
    assert!(exists(&root, Some("closes")));
}

#[test]
fn one_closing_fragment_among_empty_ones_is_a_population() {
    let root = tree(
        "mixed",
        &[
            ("a.md", "---\ncloses: []\n---\n"),
            ("b.md", "---\ncloses:\n  - MILE-2\n---\n"),
        ],
    );
    assert!(exists(&root, Some("closes")));
}

#[test]
fn a_readme_without_frontmatter_and_a_non_markdown_file_are_not_a_population() {
    let root = tree(
        "readme",
        &[
            ("README.md", "# Changesets\n"),
            ("notes.txt", "---\ncloses: [BUG-1]\n---\n"),
        ],
    );
    assert!(!exists(&root, Some("closes")));
}

#[test]
fn without_a_key_any_file_of_the_extension_counts() {
    let root = tree("no-key", &[("a.md", "---\ncloses: []\n---\n")]);
    assert!(exists(&root, None));
}

#[test]
fn a_missing_directory_is_no_population() {
    let root =
        std::env::temp_dir().join(format!("urzua-population-{}-missing", std::process::id()));
    assert!(!population_exists(
        &root,
        ".changeset",
        "md",
        Some("closes")
    ));
}
