export function InspectorPanel() {
  return (
    <div className="panel" style={{ flex: 1 }}>
      <div className="panel-header">Inspector</div>
      <div className="panel-content">
        <p style={{ fontSize: '12px', color: 'var(--color-text-secondary)' }}>
          Select an entity to edit properties
        </p>
      </div>
    </div>
  )
}
