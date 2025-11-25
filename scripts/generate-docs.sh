#!/usr/bin/env bash
# =============================================================================
# generate-docs.sh
# =============================================================================
# Build local API documentation for every crate in the Carcinisation workspace.
# The output lives in target/doc and is ignored by git.
#
# Usage:
#   scripts/generate-docs.sh          # build docs once
#   DOCS_OFFLINE=1 scripts/generate-docs.sh
#   scripts/generate-docs.sh --serve  # build, then serve via python http.server
#
# Env vars:
#   DOCS_OFFLINE=1         Build without touching the network (cargo doc --offline)
#   DOCS_PRIVATE=0         Skip --document-private-items if you only want public APIs
#   DOCS_PORT=7878         Port used when running with --serve
# =============================================================================
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

DOC_FLAGS=(--workspace --all-features)
if [[ "${DOCS_PRIVATE:-1}" != "0" ]]; then
  DOC_FLAGS+=(--document-private-items)
fi
if [[ "${DOCS_OFFLINE:-0}" != "0" ]]; then
  DOC_FLAGS+=(--offline)
fi

printf '📚 Generating workspace docs...\n'
cargo doc "${DOC_FLAGS[@]}"
printf '✅ Docs available under %s/target/doc\n' "$REPO_ROOT"

if [[ "${1:-}" == "--serve" ]]; then
  PORT="${DOCS_PORT:-7878}"
  printf '🌐 Serving docs at http://localhost:%s (Ctrl+C to stop)\n' "$PORT"
  cd target/doc
  python3 -m http.server "$PORT"
fi
