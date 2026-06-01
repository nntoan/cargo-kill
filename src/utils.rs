use std::path::Path;

/// A kind of project the scanner can detect. Add new entries to
/// [`PROJECT_KINDS`] to support more ecosystems (flutter, gradle, etc.).
pub struct ProjectKind {
    /// Short name shown to the user, e.g. "cargo", "npm".
    pub name: &'static str,
    /// File at the project root that identifies this kind.
    pub identifier: &'static str,
    /// Returns the candidate target directory names (relative to the project
    /// root) for this kind. Called once per detected project; allowed to
    /// inspect files at `project_root` (e.g. parse package.json).
    pub targets: fn(&Path) -> Vec<&'static str>,
}

pub const PROJECT_KINDS: &[ProjectKind] = &[
    ProjectKind {
        name: "cargo",
        identifier: "Cargo.toml",
        targets: cargo_targets,
    },
    ProjectKind {
        name: "npm",
        identifier: "package.json",
        targets: npm_targets,
    },
];

fn cargo_targets(_root: &Path) -> Vec<&'static str> {
    vec!["target"]
}

fn npm_targets(root: &Path) -> Vec<&'static str> {
    let mut dirs = vec!["node_modules"];
    dirs.extend(npm_framework_targets(&root.join("package.json")));
    dirs
}

// Maps an npm dependency name to one or more cache directories it produces.
const NPM_FRAMEWORK_MAP: &[(&str, &[&str])] = &[
    ("next", &[".next"]),
    ("nuxt", &[".nuxt", ".output"]),
    ("nuxt3", &[".nuxt", ".output"]),
    ("@sveltejs/kit", &[".svelte-kit"]),
];

fn npm_framework_targets(package_json: &Path) -> Vec<&'static str> {
    let text = match std::fs::read_to_string(package_json) {
        Ok(t) => t,
        Err(_) => return vec![],
    };
    let value: serde_json::Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(_) => return vec![],
    };

    let mut out: Vec<&'static str> = vec![];
    for section in ["dependencies", "devDependencies"] {
        let Some(obj) = value.get(section).and_then(|v| v.as_object()) else {
            continue;
        };
        for (dep_name, dirs) in NPM_FRAMEWORK_MAP {
            if obj.contains_key(*dep_name) {
                for d in *dirs {
                    if !out.contains(d) {
                        out.push(*d);
                    }
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        path::{Path, PathBuf},
        sync::atomic::{AtomicU64, Ordering},
    };

    static NEXT_TEST_DIR_ID: AtomicU64 = AtomicU64::new(0);

    struct TestDir {
        path: PathBuf,
    }

    impl TestDir {
        fn new(name: &str) -> Self {
            let id = NEXT_TEST_DIR_ID.fetch_add(1, Ordering::Relaxed);
            Self {
                path: std::env::temp_dir()
                    .join(format!("cargo-killer-{name}-{}-{id}", std::process::id())),
            }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            if self.path.exists() {
                fs::remove_dir_all(&self.path).expect("cleanup test directory");
            }
        }
    }

    #[test]
    fn npm_framework_targets_detects_supported_dependencies_once() {
        let root = TestDir::new("npm-frameworks");
        fs::create_dir_all(root.path()).expect("create test directory");
        let package_json = root.path().join("package.json");
        fs::write(
            &package_json,
            r#"{
                "dependencies": {"next": "14", "nuxt": "3"},
                "devDependencies": {"next": "14", "@sveltejs/kit": "2"}
            }"#,
        )
        .expect("write package.json");

        let targets = npm_framework_targets(&package_json);

        assert_eq!(targets, vec![".next", ".nuxt", ".output", ".svelte-kit"]);
    }

    #[test]
    fn npm_framework_targets_ignores_missing_or_malformed_package_json() {
        let root = TestDir::new("npm-invalid");
        fs::create_dir_all(root.path()).expect("create test directory");
        let package_json = root.path().join("package.json");

        assert!(npm_framework_targets(&package_json).is_empty());

        fs::write(&package_json, r#"{"dependencies":{"next""#).expect("write invalid JSON");
        assert!(npm_framework_targets(&package_json).is_empty());
    }

    #[test]
    fn npm_targets_always_includes_node_modules() {
        let root = TestDir::new("npm-targets");
        fs::create_dir_all(root.path()).expect("create test directory");
        fs::write(
            root.path().join("package.json"),
            r#"{"devDependencies":{"@sveltejs/kit":"2"}}"#,
        )
        .expect("write package.json");

        assert_eq!(
            npm_targets(root.path()),
            vec!["node_modules", ".svelte-kit"]
        );
    }
}
