#!/usr/bin/env python3
"""
Toolkit Base MCP server - Generic repository tools (STDIO).

Provides a sandboxed `run_shell` helper plus basic environment metadata so
Continue can execute read-only commands in a reproducible container.

Configuration:
- ROOT: Repository root (env: TOOLKIT_ROOT, fallback CARCINISATION_ROOT, default: cwd)
- Read-only by design, containerized for safety
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

mcp = FastMCP("Toolkit Base MCP")

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


def _resolve_path(rel_path: str | os.PathLike[str]) -> Path:
    """Resolve a path inside the repo root and guard against escapes."""
    candidate = (ROOT / Path(rel_path)).resolve()
    if not str(candidate).startswith(str(ROOT)):
        raise ValueError("Path escapes repository root")
    return candidate

# --------------------------------------------------------------------------- #
# Tools - Repository Utilities
# --------------------------------------------------------------------------- #
@mcp.tool()
def run_shell(command: str, cwd: str = ".") -> Dict[str, str]:
    """
    Execute a read-only shell command (stderr preserved).

    Args:
        command: Shell command to execute
        cwd: Working directory, relative to repo root (default ".")

    Returns:
        Dict with stdout, stderr, returncode (as strings)
    """
    workdir = _resolve_path(cwd)
    result = _run(["bash", "-lc", command], cwd=workdir)
    return {
        "stdout": result.stdout,
        "stderr": result.stderr,
        "returncode": str(result.returncode),
    }


@mcp.tool()
def env_info() -> Dict[str, str]:
    """Return server environment info."""
    return {"root": str(ROOT)}

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
