import "./Timeline.css"

export function Timeline() {
  return (
    <div className="timeline panel">
      <div className="panel-header">Timeline</div>
      <div className="timeline-content">
        <div className="timeline-controls">
          <label htmlFor="screen-slider">
            Screen Position:
            <input
              type="range"
              min="0"
              max="1000"
              defaultValue="0"
              className="timeline-slider"
            />
          </label>
          <span className="timeline-value">0.0s</span>
        </div>
      </div>
    </div>
  )
}
