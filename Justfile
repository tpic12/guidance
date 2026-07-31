# List available recipes
default:
    @just --list

# --- Bare-metal (needs the Rust/Node toolchain locally — see README Prerequisites) ---

# Run the dev server with hot reload
dev:
    cargo leptos watch

# Fast typecheck of the ssr target
check:
    cargo check --features ssr

# Run the unit test suite
test:
    cargo test --features ssr

# Production build (binary in target/release, assets in target/site)
build:
    cargo leptos build --release

# Apply any pending migrations to dev.db (no seeding)
migrate:
    cargo run --features ssr --bin migrate

# Seed dev.db from fixtures/ (your real, gitignored rulebook content)
seed:
    cargo run --features ssr --bin seed

# Wipe dev.db's tables and reseed from fixtures/
seed-reset:
    SEED_RESET=1 cargo run --features ssr --bin seed

# Seed e2e.db from the committed mock content in test-fixtures/
seed-e2e:
    FIXTURES_DIR=test-fixtures DATABASE_URL=sqlite://e2e.db SEED_RESET=1 cargo run --features ssr --bin seed

# Install Playwright's Node dependencies (first time only)
e2e-install:
    cd end2end && npm install

# Run the Playwright end-to-end suite (run `just e2e-install` first)
e2e:
    cargo leptos end-to-end

# --- Docker dev container (OrbStack/Docker Compose — no local toolchain needed) ---

# Build (if needed) and start the dev container with hot reload, in the background
docker-up:
    docker compose up -d --build

# Stop and remove the dev container
docker-down:
    docker compose down

# Follow the dev container's logs
docker-logs:
    docker compose logs -f app

# Open a shell in the running dev container
docker-shell:
    docker compose exec app sh

# Typecheck the ssr target inside the dev container
docker-check:
    docker compose exec app cargo check --features ssr

# Run the unit test suite inside the dev container
docker-test:
    docker compose exec app cargo test --features ssr

# Apply any pending migrations to dev.db inside the dev container (no seeding)
docker-migrate:
    docker compose exec app cargo run --features ssr --bin migrate

# Seed dev.db from fixtures/ inside the dev container
docker-seed:
    docker compose exec app cargo run --features ssr --bin seed

# Wipe dev.db's tables and reseed from fixtures/ inside the dev container
docker-seed-reset:
    docker compose exec app sh -c "SEED_RESET=1 cargo run --features ssr --bin seed"

# Seed e2e.db from test-fixtures/ inside a one-off container (run before docker-e2e)
docker-seed-e2e:
    docker compose run --rm --no-deps app sh -c "FIXTURES_DIR=test-fixtures DATABASE_URL=sqlite://e2e.db SEED_RESET=1 cargo run --features ssr --bin seed"

# Run the Playwright end-to-end suite in a one-off container against e2e.db
docker-e2e:
    docker compose run --rm --no-deps app sh -c "DATABASE_URL=sqlite://e2e.db cargo leptos end-to-end"
