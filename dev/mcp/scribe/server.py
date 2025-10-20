#!/usr/bin/env python3
from __future__ import annotations
from mcp.server.fastmcp import FastMCP
import json, os, sys
from typing import Dict
from urllib import request
from urllib.error import URLError

m = FastMCP("scribe")  # server name; tools appear under "Scribe (Docs & Commits)"
MODEL = os.environ.get("SCRIBE_MODEL", "llama3.1:8b-instruct-q8_0")
OLLAMA_HOST = os.environ.get("OLLAMA_HOST", "http://host.docker.internal:11434")

def _ollama_run(prompt: str) -> str:
    # Call Ollama via HTTP API; no external dependencies needed
    try:
        payload = json.dumps({
            "model": MODEL,
            "prompt": prompt,
            "stream": False,
            "format": "json"
        }).encode('utf-8')

        req = request.Request(
            f"{OLLAMA_HOST}/api/generate",
            data=payload,
            headers={"Content-Type": "application/json"},
            method="POST"
        )

        with request.urlopen(req, timeout=60) as response:
            result = json.loads(response.read().decode('utf-8'))
            return result.get("response", "").strip()
    except URLError as e:
        print(f"[scribe] ollama connection error: {e}", file=sys.stderr)
        return ""
    except Exception as e:
        print(f"[scribe] ollama error: {e}", file=sys.stderr)
        return ""

def _json_prompt(instruction: str, schema_hint: str) -> Dict[str, str]:
    prompt = f"""You are a precise writing assistant for developers.
Return STRICT JSON only. Do not include backticks or extra text.
JSON schema example:
{schema_hint}

Instruction:
{instruction}
"""
    out = _ollama_run(prompt)
    # Try parse once; if it fails, wrap raw
    try:
        data = json.loads(out)
        data["raw"] = out
        return data
    except Exception:
        return {"raw": out}

@m.tool()
def draft_commit(diff: str, style: str = "conventional") -> Dict[str, str]:
    schema = '{"subject": "string", "body": "string"}'
    instr = f"""Write a commit message from this unified diff.
Style: {style}. Subject <= 72 chars, imperative mood. Body: why/how, wrapped.
Diff:
{diff}
"""
    return _json_prompt(instr, schema)

@m.tool()
def draft_pr(context: str, style: str = "concise") -> Dict[str, str]:
    schema = '{"title": "string", "body": "string"}'
    instr = f"""Write a PR title and body with sections:
Summary, Changes, Rationale, Risks/Rollback, Test Plan.
Style: {style}. Output Markdown in 'body'.
Context:
{context}
"""
    return _json_prompt(instr, schema)

@m.tool()
def summarize(text: str, goal: str = "developer summary") -> Dict[str, str]:
    schema = '{"tldr": "string", "bullets": ["string"]}'
    instr = f"""Summarize for: {goal}. Provide a one-line 'tldr' and 5 bullet points.
Text:
{text}
"""
    return _json_prompt(instr, schema)

if __name__ == "__main__":
    # stderr-only startup note
    print(f"[scribe] using model: {MODEL}", file=sys.stderr)
    print(f"[scribe] ollama host: {OLLAMA_HOST}", file=sys.stderr)
    m.run()
