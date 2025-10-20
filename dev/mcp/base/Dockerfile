FROM python:3.13-slim

# Tools for generic repo operations
RUN apt-get update && apt-get install -y --no-install-recommends \
    ripgrep git \
    && rm -rf /var/lib/apt/lists/*

# Runtime env
ENV PYTHONUNBUFFERED=1 \
    PYTHONIOENCODING=UTF-8 \
    LC_ALL=C.UTF-8 \
    LANG=C.UTF-8 \
    TOOLKIT_ROOT=/app \
    CARCINISATION_ROOT=/app

WORKDIR /app

# Copy only the MCP Base folder (host repo will be bind-mounted read-only at runtime)
COPY dev/mcp/base /app/dev/mcp/base

# Install the official MCP SDK
RUN pip install --no-cache-dir "mcp==1.16.0"

# Sanity check at build time
RUN python - <<'PY'
import mcp, sys
try:
    from mcp.server.fastmcp import FastMCP
    print("✅ mcp SDK OK, FastMCP available, version:", getattr(mcp, "__version__", "unknown"))
except Exception as e:
    print("❌ mcp import failed:", e, file=sys.stderr)
    raise
PY

# Run the server over stdio; keep stdout clean for JSON-RPC
ENTRYPOINT ["python", "-u", "-X", "utf8", "dev/mcp/base/server.py"]
