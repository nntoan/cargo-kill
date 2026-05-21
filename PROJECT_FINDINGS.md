# Project Findings

## Overview

- `cargo-killer` is a Rust CLI that scans a directory tree for Cargo and npm projects, calculates reclaimable cache/build directory size, and deletes selected targets.
- The package is published as `cargo-killer`, with version bumps and changelog/release management handled through Release Please.
- Main implementation files are `src/main.rs`, `src/find.rs`, and `src/utils.rs`.

## Build and test commands

- Build: `cargo build`
- Tests: `cargo test --all-targets`
- Formatting check: `cargo fmt --all --check`
- Linting: `cargo clippy --all-targets -- -D warnings`
- Package verification before publishing: `cargo package --locked`

## Security and risk notes

- The CLI performs permanent recursive deletion using `remove_dir_all`; the dry-run mode and confirmation prompt are important safeguards.
- `--include-git` can delete `.git` directories and local repository history. Keep this behavior opt-in and clearly documented.
- Directory size scanning intentionally does not follow symlinks, reducing the risk of traversing outside the requested tree while calculating sizes.
- Directory read and metadata errors are treated as empty/skipped subtrees, which prevents crashes but may under-report reclaimable space.
- Channel send failures in the concurrent scanner should not panic; failed sends are now ignored because they only occur when the receiving side has already gone away.
- User-provided root paths are accepted directly. Future hardening could canonicalize the root path and clearly report invalid or inaccessible roots before scanning.
- Known dependency vulnerability scanning is not currently part of CI. Consider adding `cargo audit` after selecting an installation strategy.

## CI/CD and release findings

- Rust CI should run on pull requests and pushes to `main` with formatting, clippy, build, tests, and package verification.
- Release Please should manage changelog generation, version bumps, tags, and GitHub releases.
- crates.io publishing should happen only after Release Please creates a release, using the `CARGO_REGISTRY_TOKEN` repository secret.

## Test coverage findings

- Unit coverage should focus on project detection, npm framework cache detection, symlink-safe scanning, `.git` opt-in behavior, malformed JSON handling, and thread-count edge cases.
- Interactive deletion flow remains difficult to test as currently structured; future refactoring could separate selection/deletion logic from terminal prompts.
