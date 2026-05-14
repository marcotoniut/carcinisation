#!/usr/bin/env bash
set -euo pipefail

# Install WASM toolchain and CLI tools.
# wasm-bindgen-cli MUST match the version in Cargo.lock to avoid
# "it looks like the Rust project used to create this wasm file was
# linked against version X of wasm-bindgen, but this binary uses Y"
# errors.

WASM_BINDGEN_VERSION=0.2.120

# Install wasm-opt via binaryen package if not present.
if ! command -v wasm-opt &>/dev/null; then
	echo "wasm-opt not found, installing binaryen..."
	case "$(uname -s)" in
	Darwin)
		if command -v brew &>/dev/null; then
			brew install binaryen
		else
			echo "Homebrew not found. Install binaryen manually: brew install binaryen" >&2
			exit 1
		fi
		;;
	Linux)
		if command -v apt-get &>/dev/null; then
			sudo apt-get update -qq && sudo apt-get install -y -qq binaryen
		else
			echo "apt-get not found. Install binaryen via your package manager." >&2
			exit 1
		fi
		;;
	*)
		echo "Unsupported platform $(uname -s). Install binaryen manually." >&2
		exit 1
		;;
	esac
else
	echo "wasm-opt already installed: $(wasm-opt --version)"
fi

rustup target install wasm32-unknown-unknown
cargo install wasm-server-runner
cargo install -f "wasm-bindgen-cli@${WASM_BINDGEN_VERSION}"
