#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

run_check() {
	echo "Running: $*"
	"$@"
}

run_check cargo fmt --all -- --check
run_check pnpm lint
run_check proto run ruff -- check
run_check cargo clippy --workspace --all-targets --all-features -- -D warnings
run_check make lint
CARGO_TERM_PROGRESS_WHEN=always CARGO_TERM_PROGRESS_WIDTH=60 run_check make test

echo "All pre-push checks passed."
