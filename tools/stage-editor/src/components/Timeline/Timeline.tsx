import "./Timeline.css"

export function Timeline() {
  return (
    <div className="timeline panel">
      <div className="timeline-content">
        <div className="timeline-controls">
          <label htmlFor="screen-slider">
            <header className="timeline-header">
              <span>Timeline</span>
              <span>0.0s</span>
            </header>
            <input
              type="range"
              min="0"
              max="1000"
              defaultValue="0"
              className="timeline-slider"
            />
          </label>
        </div>
      </div>
    </div>
  )
}
