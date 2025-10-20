#!/usr/bin/env bash
# Wrapper that builds & runs the Scribe MCP server inside Docker but exposes stdio to Continue.

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../../.." && pwd)"
IMAGE_NAME="carcinisation-mcp-scribe"

echo "🐋 Bootstrapping Scribe MCP (Docker)" >&2

# Build image if needed
docker build -q -t "${IMAGE_NAME}" -f "${SCRIPT_DIR}/Dockerfile.scribe" "${REPO_ROOT}" >&2

# Default instruct model for prose (can be overridden)
export SCRIBE_MODEL="${SCRIBE_MODEL:-llama3.1:8b-instruct-q8_0}"
export OLLAMA_HOST="${OLLAMA_HOST:-http://host.docker.internal:11434}"

# Run container interactively with stdio forwarding.
# `--rm` cleans up after exit; `-i` keeps stdin open for Continue's JSON-RPC.
# `--add-host=host.docker.internal:host-gateway` ensures Docker can reach the host's Ollama
exec docker run --rm -i \
  --add-host=host.docker.internal:host-gateway \
  -e SCRIBE_MODEL="${SCRIBE_MODEL}" \
  -e OLLAMA_HOST="${OLLAMA_HOST}" \
  "${IMAGE_NAME}"
