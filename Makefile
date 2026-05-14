# Carcinisation now uses `just` as the command runner.
# See `justfile` for all available recipes.
# Install just: https://github.com/casey/just (or `proto install just`)

%:
	@command -v just >/dev/null 2>&1 || { echo "Error: 'just' not found. Install: cargo install just" >&2; exit 1; }
	just $@

.DEFAULT_GOAL := help
help:
	@echo "Carcinisation uses 'just' instead of 'make'. Run 'just --list' for recipes."
