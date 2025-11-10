import { spawnSync } from "node:child_process"
import type { IncomingMessage } from "node:http"
import { resolve } from "node:path"
import type { ViteDevServer } from "vite"

const WORKSPACE_ROOT = resolve(__dirname, "../../..")
const RON_TO_JSON = "/api/ron-to-json"
const JSON_TO_RON = "/api/json-to-ron"

async function readRequestBody(req: IncomingMessage) {
  return new Promise((resolveBody, reject) => {
    let body = ""
    req.setEncoding("utf-8")
    req.on("data", (chunk) => {
      body += chunk
    })
    req.on("end", () => resolveBody(body))
    req.on("error", (error) => reject(error))
  })
}

function runBridge(mode: string, payload: string): string {
  const result = spawnSync(
    "cargo",
    ["run", "-q", "-p", "carcinisation", "--bin", "ron_bridge"],
    {
      input: JSON.stringify({ mode, payload }),
      encoding: "utf-8",
      cwd: WORKSPACE_ROOT,
    },
  )

  if (result.error) {
    throw result.error
  }

  if (result.status !== 0) {
    const message = (result.stderr || "ron bridge failed").trim()
    throw new Error(message || "ron bridge failed")
  }

  return result.stdout
}

export default function ronMiddleware() {
  return {
    name: "vite:ron-bridge",
    configureServer(server: ViteDevServer) {
      server.middlewares.use(async (req, res, next) => {
        if (req.method !== "POST" || !req.url) {
          next()
          return
        }

        if (req.url !== RON_TO_JSON && req.url !== JSON_TO_RON) {
          next()
          return
        }

        try {
          const body = await readRequestBody(req)
          if (!body) {
            throw new Error("Request body is empty")
          }

          if (req.url === RON_TO_JSON) {
            const jsonOutput = runBridge("ron-to-json", body)
            res.setHeader("content-type", "application/json")
            res.statusCode = 200
            res.end(jsonOutput)
            return
          }

          const ronOutput = runBridge("json-to-ron", body)
          res.setHeader("content-type", "text/plain")
          res.statusCode = 200
          res.end(ronOutput)
        } catch (error) {
          const message =
            error instanceof Error ? error.message : "Ron bridge failed"
          res.statusCode = 400
          res.setHeader("content-type", "application/json")
          res.end(JSON.stringify({ error: message }))
        }
      })
    },
  }
}
