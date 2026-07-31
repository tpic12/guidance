# Guidance

A self-hosted, Docker Compose-deployable TTRPG toolkit built with [Leptos](https://github.com/leptos-rs/leptos) + [Axum](https://github.com/tokio-rs/axum). One instance hosts a shared compendium (imported rulebook content + homebrew) for one or more **trusted gaming groups**. No cloud dependencies, no account required — data stays on your machine.

**Status:** Pre-1.0, Phase 1–2 in active development. Current features: spell/class/item compendium import and read-only lookup; character creation builder. See [`CLAUDE.md`](CLAUDE.md) for architecture and implementation details.

## Quick start (Docker)

Fastest path if you have Docker/OrbStack:

```bash
docker compose up -d --build
# Serves at http://localhost:3000
```

See [Docker dev environment](#docker-dev-environment) below for more details. No local Rust/Node install needed.

## Prerequisites (local development)

If you prefer to develop locally without Docker:

- **Rust nightly** — pinned by `rust-toolchain.toml`; `rustup` will pick it up automatically.
- **`wasm32-unknown-unknown` target** — `rustup target add wasm32-unknown-unknown`
- **[`cargo-leptos`](https://github.com/leptos-rs/cargo-leptos)** — `cargo install cargo-leptos --locked`
- **Node.js** (version pinned in `.nvmrc`) **+ [pnpm](https://pnpm.io/)** — used by `cargo-leptos` to build Tailwind/daisyUI CSS.
- SQLite is bundled via `sqlx`'s `sqlite` feature — no separate install needed.

## Command shortcuts

A [`Justfile`](Justfile) wraps the commands in this doc into short aliases (`just dev`, `just seed`, `just docker-up`, ...). Install [`just`](https://github.com/casey/just) (`brew install just`) and run `just --list` to see all of them. Optional — every raw command documented below always works too.

## Setup (local development)

After installing prerequisites:

```bash
pnpm install                                    # installs tailwindcss + daisyui (used by the cargo-leptos CSS build)
cargo run --features ssr --bin seed             # creates/migrates sqlite://dev.db and loads fixtures/spells.seed.json
```

Re-run the seed command with `SEED_RESET=1` to wipe and reload the `spells` table:

```bash
SEED_RESET=1 cargo run --features ssr --bin seed
```

## Running the dev server

```bash
cargo leptos watch
```

Serves the app at `http://127.0.0.1:3000` with hot reload (browser auto-reload on port `3001`).

## Docker dev environment

`Dockerfile.dev` + `compose.yaml` run `cargo leptos watch` inside a container with hot reload — no local Rust/Node install needed, just Docker/OrbStack and `docker compose`.

```bash
docker compose up -d --build   # or: just docker-up
```

Serves at `http://localhost:3000` (hot reload on `3001`). First build takes a few minutes; subsequent starts are fast. `target/`, cargo registry, and `node_modules` all live in named volumes. Source is bind-mounted, so file edits auto-reload.

Seeding inside the container:

```bash
docker compose exec app cargo run --features ssr --bin seed                   # or: just docker-seed
docker compose exec app sh -c "SEED_RESET=1 cargo run --features ssr --bin seed"  # or: just docker-seed-reset
```

`dev.db` is bind-mounted at the repo root — inspectable with `sqlite3 dev.db`. Other commands: `docker compose logs -f app` (or `just docker-logs`), `docker compose exec app sh` (or `just docker-shell`), `docker compose down` (or `just docker-down`).

## Checking your work

```bash
cargo check --features ssr                              # fast typecheck of the server target
cargo build --features ssr --bin guidance               # build the server binary
cargo build --features ssr --bin seed                    # build the seed binary
cargo build --lib --no-default-features --features hydrate --target wasm32-unknown-unknown   # build the wasm client
cargo test --features ssr                                # unit tests (importer transforms, db queries)
```

These are the same checks run in CI (see `.github/workflows/ci.yml`) on every PR/push to `main`.

## End-to-end tests

```bash
cd end2end && npm install   # first time only
cargo leptos end-to-end
cargo leptos end-to-end --release
```

Cargo-leptos uses Playwright as the end-to-end test tool. Tests live in `end2end/tests`.

## Compiling for release

```bash
cargo leptos build --release
```

Generates the server binary in `target/release` and the site package in `target/site`.

## Running a release build on a remote machine (without the Rust toolchain)

After `cargo leptos build --release`, the minimum files needed are:

1. The server binary, located in `target/release`
2. The `site` directory and everything in it, located in `target/site`

Copy these to your remote server with this layout:

```text
guidance
site/
```

Set the following environment variables (adjust as needed):

```sh
export LEPTOS_OUTPUT_NAME="guidance"
export LEPTOS_SITE_ROOT="site"
export LEPTOS_SITE_PKG_DIR="pkg"
export LEPTOS_SITE_ADDR="127.0.0.1:3000"
export LEPTOS_RELOAD_PORT="3001"
```

Then run the server binary. Note: `Dockerfile.dev` and `compose.yaml` are dev-loop tooling only. Production deployment (Compose file, systemd service, cloud-ready container) is planned but not yet implemented. For now, the manual-binary approach above is the only deployment path.

## License

[The Unlicense](LICENSE).
