#!/usr/bin/env python3
"""
Bevy MCP server - Bevy-specific analysis tools (STDIO).

Bevy game engine tools:
- bevy_version() → Extract Bevy dependency version from Cargo.toml
- find_bevy_system_like_fns() → Locate functions with Bevy system signatures

Configuration:
- ROOT: Repository root (env: TOOLKIT_ROOT, fallback CARCINISATION_ROOT, default: cwd)
- Requires: ripgrep
"""

from __future__ import annotations

import os
import subprocess
from pathlib import Path
from typing import List, Optional

from mcp.server.fastmcp import FastMCP

# --------------------------------------------------------------------------- #
# Configuration
# --------------------------------------------------------------------------- #

ROOT = Path(
    os.environ.get("TOOLKIT_ROOT")
    or os.environ.get("CARCINISATION_ROOT")
    or os.getcwd()
).resolve()

mcp = FastMCP("Bevy MCP")

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

# --------------------------------------------------------------------------- #
# Tools - Bevy-Specific
# --------------------------------------------------------------------------- #

@mcp.tool()
def bevy_version() -> str:
    """Return the Bevy dependency line(s) from Cargo.toml."""
    _require("rg")
    p = _run(["rg", "-n", r"^\s*bevy\s*=", "Cargo.toml"])
    return p.stdout or "(not found)"


@mcp.tool()
def find_bevy_system_like_fns() -> str:
    """Functions that look like Bevy systems (Query/Res/Commands/EventReader/EventWriter)."""
    _require("rg")
    pat = r'''fn\s+\w+\s*\([^)]*(Query<|Res(?:Mut)?<|Commands|EventReader<|EventWriter<)'''
    return _run(["rg", "-n", "--pcre2", pat, "src", "tools", "scripts"]).stdout

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
