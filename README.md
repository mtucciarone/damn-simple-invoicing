# Damn Simple Invoicing

Local-first invoicing desktop app built with SvelteKit, Tauri, Rust, and SQLite.

## What This Project Is

- Offline-only
- Local-only storage
- No accounts
- No cloud sync
- No telemetry
- No online APIs
- Single-user by design

## Stack

- Frontend: SvelteKit + TypeScript + Tailwind
- Desktop shell: Tauri
- Backend: Rust
- Database: SQLite
- PDF export: HTML templates rendered to local PDF files

## Prerequisites

Install these once on your machine:

- Rust stable
- Node.js 20+ or newer
- pnpm 9+
- Tauri CLI

One-time Tauri CLI install options:

```bash
cargo install tauri-cli --locked
```

or:

```bash
pnpm add -D @tauri-apps/cli
```

On Linux, you will also need the system packages required by Tauri/WebKitGTK. On Ubuntu/Debian, a typical baseline is:

```bash
sudo apt-get update
sudo apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf
```

## Run The App Locally

From the repository root:

```bash
pnpm install
cargo tauri dev
```

If you installed the CLI as a local project dependency, you can also run:

```bash
pnpm tauri dev
```

That starts the frontend dev server, then opens the Tauri desktop app.

## Cargo-Only Commands

`cargo run` by itself is not the normal entry point for this app because the desktop shell depends on the frontend build pipeline.

Use these instead:

```bash
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
```

If you want the full desktop app, use `cargo tauri dev` after installing the Tauri CLI.

## Production Build

Build local release artifacts with:

```bash
pnpm tauri build
```

You can also use:

```bash
cargo tauri build
```

## GitHub Releases

This repository includes a GitHub Actions release workflow under `.github/workflows/release.yml`.

Release flow:

1. Bump the app version in `src-tauri/tauri.conf.json` and keep `package.json` in sync
2. Create a tag that matches the version, for example `v0.1.0`
3. Push the tag to GitHub
4. The workflow builds release assets for:
   - Windows x64
   - Linux
   - macOS Intel and Apple Silicon
5. The assets are attached to a GitHub Release

## Local Data

All data stays on the machine running the app.

- SQLite database and app state are stored locally in the user data directory
- Backups are exported to a local folder
- PDF exports are written to local disk

## Documentation

See [docs/architecture.md](./docs/architecture.md) for the schema, command surface, folder structure, and MVP implementation plan.
