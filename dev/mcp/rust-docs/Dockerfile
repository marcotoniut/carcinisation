FROM python:3.13-slim

# Tools for Rust documentation operations
RUN apt-get update && apt-get install -y --no-install-recommends \
    ripgrep make git curl \
    && rm -rf /var/lib/apt/lists/*

# Install Rust (needed for cargo doc)
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
ENV PATH="/root/.cargo/bin:${PATH}"

# Runtime env
ENV PYTHONUNBUFFERED=1 \
    PYTHONIOENCODING=UTF-8 \
    LC_ALL=C.UTF-8 \
    LANG=C.UTF-8 \
    TOOLKIT_ROOT=/app \
    CARCINISATION_ROOT=/app \
    ALLOW_WRITE=0

WORKDIR /app

# Copy only the MCP Rust Docs folder (host repo will be bind-mounted at runtime)
COPY dev/mcp/rust-docs /app/dev/mcp/rust-docs

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
ENTRYPOINT ["python", "-u", "-X", "utf8", "dev/mcp/rust-docs/server.py"]
