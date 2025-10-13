export function PalettePanel() {
  return (
    <div className="panel" style={{ flex: 1 }}>
      <div className="panel-header">Palette</div>
      <div className="panel-content">
        <p style={{ fontSize: "12px", color: "var(--color-text-secondary)" }}>
          Monsters and objects will appear here
        </p>
      </div>
    </div>
  )
}
