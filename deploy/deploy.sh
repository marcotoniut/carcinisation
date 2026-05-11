#!/usr/bin/env bash
set -euo pipefail

# Native multiplayer server deploy only. Browser multiplayer is not deployed yet.

REMOTE="${DEPLOY_REMOTE:-sship}"
RELEASE_ID="${DEPLOY_RELEASE_ID:-$(git rev-parse --short=12 HEAD 2>/dev/null || date +%Y%m%d%H%M%S)}"
EXPECTED_ARCH="${DEPLOY_EXPECTED_ARCH:-x86_64}"
SERVER_BINARY="${DEPLOY_SERVER_BINARY:-target/x86_64-unknown-linux-gnu/release/carcinisation_server}"
CTL_BINARY="${DEPLOY_CTL_BINARY:-target/x86_64-unknown-linux-gnu/release/carcinisationctl}"
REMOTE_HELPER="${DEPLOY_REMOTE_HELPER:-/usr/local/sbin/carcinisation-deploy}"

validate_release_id() {
    case "$1" in
        ""|"."|".."|*/*|*\\*) return 1 ;;
    esac
    [[ "$1" =~ ^[A-Za-z0-9._-]+$ ]]
}

if ! validate_release_id "$RELEASE_ID"; then
    echo "unsafe DEPLOY_RELEASE_ID: $RELEASE_ID" >&2
    exit 1
fi

if ! command -v file >/dev/null 2>&1; then
    echo "required command not found: file" >&2
    exit 1
fi

validate_binary() {
    local name="$1" path="$2"
    if [ ! -x "$path" ]; then
        echo "$name binary not found or not executable: $path" >&2
        exit 1
    fi
    local info
    info="$(file "$path")"
    case "$EXPECTED_ARCH:$info" in
        x86_64:*ELF*64-bit*x86-64*|amd64:*ELF*64-bit*x86-64*) ;;
        aarch64:*ELF*64-bit*ARM*aarch64*|arm64:*ELF*64-bit*ARM*aarch64*) ;;
        *)
            echo "$name binary must be a Linux ELF for $EXPECTED_ARCH: $info" >&2
            exit 1
            ;;
    esac
}

validate_binary "server" "$SERVER_BINARY"
validate_binary "ctl" "$CTL_BINARY"

ssh_cmd=(ssh "$REMOTE")
scp_cmd=(scp)
rsync_cmd=(rsync -az --delete --delay-updates)

cleanup_remote_tmp() {
    if [ -n "${REMOTE_TMP:-}" ]; then
        "${ssh_cmd[@]}" "rm -rf '$REMOTE_TMP'" || true
    fi
}
trap cleanup_remote_tmp EXIT

echo "==> Checking remote architecture"
remote_arch="$("${ssh_cmd[@]}" "uname -m")"
case "$EXPECTED_ARCH:$remote_arch" in
    x86_64:x86_64|amd64:x86_64|aarch64:aarch64|arm64:aarch64) ;;
    *)
        echo "remote architecture mismatch: expected $EXPECTED_ARCH, got $remote_arch" >&2
        exit 1
        ;;
esac

echo "==> Checking remote deploy helper"
"${ssh_cmd[@]}" "test -x '$REMOTE_HELPER'"

echo "==> Creating remote staging directory"
REMOTE_TMP="$("${ssh_cmd[@]}" "mktemp -d /tmp/carcinisation-deploy.XXXXXX")"
"${ssh_cmd[@]}" "mkdir -p '$REMOTE_TMP/bin' '$REMOTE_TMP/assets' '$REMOTE_TMP/configs'"

echo "==> Uploading binaries, assets, and config examples"
"${scp_cmd[@]}" "$SERVER_BINARY" "${REMOTE}:${REMOTE_TMP}/bin/carcinisation_server"
"${scp_cmd[@]}" "$CTL_BINARY" "${REMOTE}:${REMOTE_TMP}/bin/carcinisationctl"
"${rsync_cmd[@]}" assets/ "${REMOTE}:${REMOTE_TMP}/assets/"
"${rsync_cmd[@]}" deploy/configs/ "${REMOTE}:${REMOTE_TMP}/configs/"

echo "==> Installing release ${RELEASE_ID}"
"${ssh_cmd[@]}" "sudo '$REMOTE_HELPER' install '$RELEASE_ID' '$REMOTE_TMP'"

echo "==> Deploy complete"
