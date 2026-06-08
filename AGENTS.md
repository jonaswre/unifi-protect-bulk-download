# Repository Guidelines

## Project Structure & Module Organization

This is a Rust CLI for downloading UniFi Protect recordings. Source code lives in `src/`:

- `src/main.rs`: CLI entrypoint, download flow, date parsing, and unit tests.
- `src/parse_args.rs`: `clap` argument definitions and enums.
- `src/app_error.rs`: structured application errors and helpers.

Project metadata is in `Cargo.toml` and `Cargo.lock`. CI/CD workflows live in `.github/workflows/`. Docker packaging is defined by `Dockerfile` and `.dockerignore`. There is no separate assets directory.

## Build, Test, and Development Commands

- `cargo build --locked`: compile the debug binary using the checked-in lockfile.
- `cargo run -- download <uri> <username> <password> <out_path> <mode> <recording_type> <start_date> <end_date> [cameras]`: run the CLI locally.
- `cargo test --locked`: run unit tests.
- `cargo clippy --all-targets --all-features -- -D warnings`: run lint checks as CI does.
- `cargo fmt -- --check`: verify formatting.
- `docker build -t unifi-protect-bulk-download:ci .`: validate the Docker image build.

Use `cargo build --release --locked` for local release builds.

## Coding Style & Naming Conventions

Use standard Rust formatting via `rustfmt`; do not hand-format around it. Keep functions and variables in `snake_case`, types and enums in `PascalCase`, and enum variants descriptive. Prefer small helpers for testable logic, as in camera selection and date parsing. Keep error messages specific and route user-facing failures through `AppError` where practical.

## Testing Guidelines

Tests use Rust’s built-in test framework. Place focused unit tests near the code under `#[cfg(test)]`. Name tests by behavior, for example `selects_only_cameras_matching_requested_name_or_id`. Before pushing, run:

```bash
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --locked
```

## Commit & Pull Request Guidelines

History uses concise, imperative commit subjects such as `Fix date parsing` and `Add CI and release workflows`. Keep commits scoped to one logical change. PRs should describe the user-visible behavior, list validation commands, and link related issues when available. For release-related PRs, mention affected workflows and target platforms.

## Release & CI Notes

CI runs on Linux, macOS, and Windows. Release binaries are built when pushing `v*` tags and attached to GitHub Releases for Linux x64, Windows x64, macOS Intel, and macOS Apple Silicon. Keep `Cargo.toml`, `Cargo.lock`, and the release tag version aligned.
