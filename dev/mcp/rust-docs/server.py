#!/usr/bin/env python3
"""
Rust Docs MCP server - Rust documentation tools (STDIO).

Rust documentation analysis and generation:
- list_rust_files() → List all Rust files (src/, tools/, scripts/)
- find_module_doc_gaps() → Identify files missing module-level docs (//! or /*!)
- find_public_item_doc_gaps() → Find public items missing rustdoc comments (///)
- cargo_doc(all_features, no_deps) → Generate local API documentation
- docs_index_path() → Return path to generated docs/index.html
- make(target, env) → Execute Make targets
- insert_module_header(filepath, header) → Write module documentation (requires ALLOW_WRITE=1)

Configuration:
- ROOT: Repository root (env: TOOLKIT_ROOT, fallback CARCINISATION_ROOT, default: cwd)
- ALLOW_WRITE: Enable write operations (env: ALLOW_WRITE, default: "0")
- Requires: ripgrep, make, cargo
"""

from __future__ import annotations

import os
import subprocess
from pathlib import Path
from typing import Dict, List, Optional

from mcp.server.fastmcp import FastMCP

# --------------------------------------------------------------------------- #
# Configuration
# --------------------------------------------------------------------------- #

ROOT = Path(
    os.environ.get("TOOLKIT_ROOT")
    or os.environ.get("CARCINISATION_ROOT")
    or os.getcwd()
).resolve()
ALLOW_WRITE = os.environ.get("ALLOW_WRITE", "0").lower() in ("1", "true", "yes")

mcp = FastMCP("Rust Docs MCP")

# --------------------------------------------------------------------------- #
# Helpers
# --------------------------------------------------------------------------- #

def _run(
    cmd: List[str] | str,
    cwd: Optional[Path] = None,
    shell: bool = False,
) -> subprocess.CompletedProcess[str]:
    """Run a command and capture output without raising on non-zero exit."""
    return subprocess.run(
        cmd,
        cwd=str(cwd or ROOT),
        shell=shell,
        text=True,
        capture_output=True,
        check=False,
    )


def _require(bin_name: str) -> None:
    """Ensure a binary exists in PATH; raise RuntimeError otherwise."""
    chk = _run(["bash", "-lc", f"command -v {bin_name} >/dev/null 2>&1"])
    if chk.returncode != 0:
        raise RuntimeError(f"Missing tool: {bin_name}")


def _resolve_path(rel_path: str | os.PathLike[str]) -> Path:
    """Resolve a path inside the repo root and guard against escapes."""
    candidate = (ROOT / Path(rel_path)).resolve()
    if not str(candidate).startswith(str(ROOT)):
        raise ValueError("Path escapes repository root")
    return candidate

# --------------------------------------------------------------------------- #
# Tools - Rust Documentation
# --------------------------------------------------------------------------- #

@mcp.tool()
def list_rust_files() -> List[str]:
    """List Rust files under src/, tools/, scripts/ (excluding target/generated)."""
    _require("rg")
    p = _run(
        [
            "rg",
            "--files",
            "--glob", "src/**/*.rs",
            "--glob", "tools/**/*.rs",
            "--glob", "scripts/**/*.rs",
            "--glob", "!**/target/**",
            "--glob", "!**/generated/**",
        ]
    )
    return [x for x in p.stdout.splitlines() if x]


@mcp.tool()
def find_module_doc_gaps() -> List[str]:
    """Files likely missing a top-of-file module doc (`//!` or `/*!`)."""
    _require("rg")
    all_rs = _run(
        ["rg", "--files", "--glob", "src/**/*.rs", "--glob", "tools/**/*.rs", "--glob", "scripts/**/*.rs"]
    )
    has = _run(r'''bash -lc 'rg -l --pcre2 "^(//!|/\*\!)" src tools scripts' ''', shell=True)
    aset, dset = set(all_rs.stdout.splitlines()), set(has.stdout.splitlines())
    return sorted([p for p in aset if p and p not in dset])


@mcp.tool()
def find_public_item_doc_gaps() -> str:
    """Public items missing leading `///` rustdoc (heuristic)."""
    _require("rg")
    cmd = r'''rg -n --pcre2 '(?m)^(?!\s*///)\s*pub\s+(struct|enum|trait|type|fn|const)\s+\w' src tools scripts'''
    return _run(cmd, shell=True).stdout


@mcp.tool()
def cargo_doc(all_features: bool = True, no_deps: bool = False) -> str:
    """Generate local API docs (target/doc/index.html)."""
    _require("cargo")
    args = ["cargo", "doc"]
    if all_features:
        args.append("--all-features")
    if no_deps:
        args.append("--no-deps")
    p = _run(args)
    out = p.stdout + ("\n" + p.stderr if p.stderr else "")
    out += "\nDocs index (if built): target/doc/index.html"
    return out


@mcp.tool()
def docs_index_path() -> str:
    """Return docs index path if it exists."""
    idx = ROOT / "target" / "doc" / "index.html"
    return str(idx) if idx.exists() else "not found; run cargo_doc() first"


@mcp.tool()
def make(target: str, env: Optional[str] = None) -> str:
    """Run a Make target (check, lint, test, build-web, release-wasm)."""
    _require("make")
    env_map = os.environ.copy()
    if env:
        for kv in env.split():
            if "=" in kv:
                k, v = kv.split("=", 1)
                env_map[k] = v
    p = subprocess.run(["make", target], cwd=str(ROOT), env=env_map, text=True, capture_output=True)
    return p.stdout + ("\n" + p.stderr if p.stderr else "")


@mcp.tool()
def insert_module_header(filepath: str, header: str) -> str:
    """
    Insert module-level documentation at the top of a Rust file.

    Requires ALLOW_WRITE=1 environment variable.

    Args:
        filepath: Path to the Rust file (relative to repo root)
        header: Module documentation header (should start with //! or /*!)

    Returns:
        Success/error message
    """
    if not ALLOW_WRITE:
        raise RuntimeError("Write operations disabled; set ALLOW_WRITE=1")

    target = _resolve_path(filepath)
    if not target.exists():
        raise FileNotFoundError(f"File not found: {filepath}")

    content = target.read_text(encoding="utf-8")
    new_content = header.rstrip() + "\n\n" + content
    target.write_text(new_content, encoding="utf-8")

    return f"✅ Module header inserted in {filepath}"


@mcp.tool()
def env_info() -> Dict[str, str]:
    """Return server environment info."""
    return {"root": str(ROOT), "allow_write": str(ALLOW_WRITE)}

# --------------------------------------------------------------------------- #
# Entry point (STDIO)
# --------------------------------------------------------------------------- #

if __name__ == "__main__":
    try:
        mcp.run()
    except Exception:
        import traceback, sys
        traceback.print_exc()
        sys.exit(1)
