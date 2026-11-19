# Spektrum development tasks. `just` lists them; `just check` is what CI runs.

web_apps := "frontend admin_panel"

# List the recipes
default:
    @just --list

# Install the web apps' dependencies from their lockfiles
install:
    for app in {{ web_apps }}; do (cd $app && npm ci); done

# Format everything
fmt:
    cd server && cargo fmt
    cd stress_test && cargo fmt
    for app in {{ web_apps }}; do (cd $app && npm run format); done

# Run every check CI runs, except the browser e2e tests
check: check-server check-stress-test check-frontend check-admin

# Format, lint and test the server; also fails if the generated TypeScript types are stale
check-server:
    cd server && cargo fmt --check
    cd server && cargo clippy --all --benches --tests --examples --all-features -- -D warnings
    cd server && cargo test

check-stress-test:
    cd stress_test && cargo fmt --check
    cd stress_test && cargo clippy --all --benches --tests --examples --all-features -- -D warnings

check-frontend: (check-web "frontend")

check-admin: (check-web "admin_panel")

# Browser end-to-end tests for both web apps
e2e: (web "frontend" "npm run test:e2e") (web "admin_panel" "npm run test:e2e")

# Regenerate the TypeScript types the web apps share with the server
bindings:
    cd server && (cargo test bindings -q || cargo test bindings -q)

# Run the server locally with the settings in server/.env (start from server/env.example)
dev-server:
    #!/usr/bin/env bash
    set -euo pipefail
    cd server
    if [ -f .env ]; then
        set -a
        source ./.env
        set +a
    fi
    cargo run

dev-frontend:
    cd frontend && npm run dev -- --host

dev-admin:
    cd admin_panel && npm run dev -- --host

# Type-check, lint, unit test and build one web app
check-web app: (web app "npm run check") (web app "npm run lint") (web app "npx vitest run") (web app "npm run build")

# Run a command in a web app; without a .env there, env.example's placeholders are used
web app cmd:
    #!/usr/bin/env bash
    set -euo pipefail
    cd {{ app }}
    if ! compgen -G ".env*" > /dev/null; then
        set -a
        source ./env.example
        set +a
    fi
    {{ cmd }}
