export function ScenesPanel() {
  return (
    <div className="panel" style={{ flex: 1 }}>
      <div className="panel-header">Scenes</div>
      <div className="panel-content">
        <p style={{ fontSize: "12px", color: "var(--color-text-secondary)" }}>
          No scene loaded
        </p>
      </div>
    </div>
  )
}
