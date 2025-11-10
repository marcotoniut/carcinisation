import { useEditorStore } from "../../state/store"
import "./ConsolePanel.css"

export function ConsolePanel() {
  const { consoleMessage, parseError } = useEditorStore()
  const message =
    parseError ||
    consoleMessage ||
    "Stage Editor ready. Load a .ron file to begin."
  const variant = parseError ? "error" : "info"

  return (
    <div className="console panel">
      <div className="panel-header">Console</div>
      <div className="console-content">
        <p className={`console-message console-message-${variant}`}>
          {message}
        </p>
      </div>
    </div>
  )
}
