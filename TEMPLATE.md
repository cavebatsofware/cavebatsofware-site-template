# Using this repo as a cargo-generate template

This repository is a [`cargo-generate`](https://cargo-generate.github.io/cargo-generate/) template. The files in this tree contain liquid template tokens (e.g. `{{project-name}}`, `{{crate_name}}`) that are substituted when a new project is generated from it — they are **not** meant to compile as Rust directly in this repo.

## Generating a new project

Install `cargo-generate`, then:

```bash
cargo install cargo-generate
cargo generate --git https://github.com/cavebatsofware/cavebatsofware-site-template.git --name my-new-site
```

You will be prompted for:

| Placeholder | Description |
|-------------|-------------|
| `project-name` | Kebab-case project name (e.g. `my-new-site`) |
| `project-description` | Short description (goes in Cargo.toml and package.json) |
| `author` | Author name |
| `author-email` | Author email |
| `github-org` | GitHub org/user for repository URLs |
| `copyright-year` | Year for license header (if kept) |
| `license_style` | `gpl-3.0`, `bsd-3-clause`, or `none` — controls per-file headers, `Cargo.toml`/`package.json` license field, and which `LICENSE-*` file ships |

`cargo-generate` automatically derives `{{crate_name}}` (snake_case) from `project-name`.

## Developing on the template

Because the source tree contains liquid tokens, `cargo build` will fail on this repo directly. To iterate on the template:

1. Make edits in this repo.
2. Generate a scratch project: `cargo generate --path . --name scratch-site` into a sibling directory.
3. Build, run, and test there.
4. Port fixes back into the template.

A shortcut for local iteration:

```bash
# From a parent directory
cargo generate --path ./cavebatsofware-site-template --name scratch-site
cd scratch-site && cargo build
```

## Template structure

- `cargo-generate.toml` — placeholder definitions and conditional file ignores
- `hooks/pre.rhai` — minimal pre-generation hook (prints chosen options)
- `.cargo-generate-ignore` — files excluded from generated output (WIP notes, node_modules, etc.)

## What is templated

- `Cargo.toml` — package name, version, author, description, repository, license
- `package.json` — name, author, repository URLs
- `Dockerfile`, `entrypoint.sh`, `Makefile` — binary/image names
- `docker-compose.yml`, `docker-compose.test.yml` — container names, networks, DB defaults
- `src/**/*.rs`, `tests/**/*.rs`, `admin-frontend/src/**/*.jsx` — crate-name references and optional dual-license headers
- `src/migration/m20251202_000002_seed_site_settings.rs` — default `site_name` seed value
- `README.md` — project name, repo URLs

## What is NOT templated

- `Cargo.toml` git-dep URLs for `axum-login` and `tower-sessions-stores` — these are deliberately pinned to the upstream template author's forks.

## Ignored at generation

`ReviewFindings.md`, `TestCoverageAnalysis.md`, `.zed/`, `.env`, `node_modules/`, `target/`, `admin-assets/`, `public-assets/`, `backups/` — see `.cargo-generate-ignore`.
