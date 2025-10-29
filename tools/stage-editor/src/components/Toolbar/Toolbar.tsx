import { useEditorStore } from "../../state/store"
import { openRonFile, saveRonFile } from "../../utils/fileSystem"
import "./Toolbar.css"

export function Toolbar() {
  const { fileName, fileContent, isDirty, loadFile, markClean } =
    useEditorStore()

  const handleLoad = async () => {
    const file = await openRonFile()
    if (file) {
      loadFile(file.name, file.content)
    }
  }

  const handleSave = async () => {
    if (!fileName || !fileContent) return

    const success = await saveRonFile(fileContent, fileName)
    if (success) {
      markClean()
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
          disabled={!fileContent || !isDirty}
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
