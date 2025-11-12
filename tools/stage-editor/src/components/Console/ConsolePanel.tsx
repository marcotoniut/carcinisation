import { useEditorStore } from "../../state/store"
import * as styles from "./ConsolePanel.css"

export function ConsolePanel() {
  const { consoleMessage, parseError } = useEditorStore()
  const message =
    parseError ||
    consoleMessage ||
    "Stage Editor ready. Load a .ron file to begin."
  const variant = parseError ? "error" : "info"

  const messageClassName = `${styles.consoleMessage} ${
    variant === "error" ? styles.consoleMessageError : styles.consoleMessageInfo
  }`

  return (
    <div className={`${styles.console} panel`}>
      <div className="panel-header">Console</div>
      <div className={styles.consoleContent}>
        <p className={messageClassName}>{message}</p>
      </div>
    </div>
  )
}
