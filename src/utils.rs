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
        time::{SystemTime, UNIX_EPOCH},
    };

    fn temp_path(name: &str) -> std::path::PathBuf {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after UNIX epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("cargo-kill-{name}-{}-{now}", std::process::id()))
    }

    #[test]
    fn npm_framework_targets_detects_supported_dependencies_once() {
        let root = temp_path("npm-frameworks");
        fs::create_dir_all(&root).expect("create test directory");
        let package_json = root.join("package.json");
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

        fs::remove_dir_all(root).expect("cleanup test directory");
    }

    #[test]
    fn npm_framework_targets_ignores_missing_or_malformed_package_json() {
        let root = temp_path("npm-invalid");
        fs::create_dir_all(&root).expect("create test directory");
        let package_json = root.join("package.json");

        assert!(npm_framework_targets(&package_json).is_empty());

        fs::write(&package_json, r#"{"dependencies":{"next""#).expect("write invalid JSON");
        assert!(npm_framework_targets(&package_json).is_empty());

        fs::remove_dir_all(root).expect("cleanup test directory");
    }

    #[test]
    fn npm_targets_always_includes_node_modules() {
        let root = temp_path("npm-targets");
        fs::create_dir_all(&root).expect("create test directory");
        fs::write(
            root.join("package.json"),
            r#"{"devDependencies":{"@sveltejs/kit":"2"}}"#,
        )
        .expect("write package.json");

        assert_eq!(npm_targets(&root), vec!["node_modules", ".svelte-kit"]);

        fs::remove_dir_all(root).expect("cleanup test directory");
    }
}
