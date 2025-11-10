import { useEditorStore } from "../../state/store"
import { openRonFile, saveRonFile } from "../../utils/fileSystem"
import "./Toolbar.css"

export function Toolbar() {
  const {
    fileName,
    isDirty,
    fileHandle,
    loadFile,
    saveToRon,
    markSaved,
    parsedData,
  } = useEditorStore()

  const handleLoad = async () => {
    const file = await openRonFile()
    if (file) {
      try {
        await loadFile(file.name, file.content, file.handle)
      } catch {
        // error logged to console panel via store
      }
    }
  }

  const handleSave = async () => {
    if (!fileName) return

    try {
      const ronText = await saveToRon()
      const saved = await saveRonFile(
        ronText,
        fileName,
        fileHandle || undefined,
      )
      if (saved) {
        markSaved(ronText)
      }
    } catch (error) {
      console.error("Failed to save RON:", error)
    }
  }

  return (
    <div className="toolbar">
      <div className="toolbar-section">
        <h1 className="toolbar-title">Carcinisation Stage Editor</h1>
        {fileName && (
          <span className="toolbar-filename">
            {fileName}
            {isDirty && <span className="toolbar-dirty"> *</span>}
          </span>
        )}
      </div>
      <div className="toolbar-section">
        <button type="button" onClick={handleLoad}>
          Load
        </button>
        <button
          type="button"
          onClick={handleSave}
          disabled={!fileName || !parsedData}
        >
          Save
        </button>
      </div>
      <div className="toolbar-section">
        <button type="button" disabled>
          Undo
        </button>
        <button type="button" disabled>
          Redo
        </button>
      </div>
    </div>
  )
}
