#!/usr/bin/env bash
# Bootstrap the Rust Docs MCP via Docker or a local virtualenv fallback.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../../.." && pwd)"
IMAGE_NAME="carcinisation-mcp-rust-docs"
VENV_DIR="${SCRIPT_DIR}/.venv"
PYTHON_BIN="${PYTHON:-python3}"

log() {
  printf '[rust-docs-mcp] %s\n' "$*" >&2
}

launch_docker() {
  log "Bootstrapping Rust Docs MCP inside Docker"
  docker build -q -t "${IMAGE_NAME}" -f "${SCRIPT_DIR}/Dockerfile" "${REPO_ROOT}" >&2
  exec docker run --rm -i \
    -e TOOLKIT_ROOT="/app" \
    -e CARCINISATION_ROOT="/app" \
    -e ALLOW_WRITE="${ALLOW_WRITE:-0}" \
    -v "${REPO_ROOT}:/app:ro" \
    "${IMAGE_NAME}"
}

launch_local() {
  log "Falling back to local Python environment"
  if ! command -v "${PYTHON_BIN}" >/dev/null 2>&1; then
    log "error: ${PYTHON_BIN} not found; install Python 3.9+ or expose PYTHON env var"
    exit 1
  fi

  if [ ! -d "${VENV_DIR}" ]; then
    log "Creating virtualenv at ${VENV_DIR}"
    "${PYTHON_BIN}" -m venv "${VENV_DIR}"
  fi

  # shellcheck disable=SC1090
  source "${VENV_DIR}/bin/activate"

  if [ ! -f "${VENV_DIR}/.deps-ok" ]; then
    log "Installing Python dependencies"
    if pip install -r "${SCRIPT_DIR}/requirements.txt"; then
      touch "${VENV_DIR}/.deps-ok"
    else
      log "dependency installation failed; enable Docker or install 'mcp==1.16.0' manually"
      exit 1
    fi
  fi

  export TOOLKIT_ROOT="${REPO_ROOT}"
  export CARCINISATION_ROOT="${REPO_ROOT}"
  exec python -u "${SCRIPT_DIR}/server.py"
}

if command -v docker >/dev/null 2>&1 && docker info >/dev/null 2>&1; then
  launch_docker
else
  log "Docker unavailable; attempting local execution"
  launch_local
fi
