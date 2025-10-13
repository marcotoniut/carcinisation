import "./ConsolePanel.css"

export function ConsolePanel() {
  return (
    <div className="console panel">
      <div className="panel-header">Console</div>
      <div className="console-content">
        <p className="console-message console-message-info">
          Editor v2 ready. Load a .ron file to begin.
        </p>
      </div>
    </div>
  )
}
