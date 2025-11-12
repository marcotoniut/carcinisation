import { useEditorStore } from "../../state/store"
import { openRonFile, saveRonFile } from "../../utils/fileSystem"
import { showToast } from "../Toast/Toast"
import * as styles from "./Toolbar.css"

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
      } else {
        showToast("Save cancelled", "File save was cancelled", "info")
      }
    } catch (error) {
      console.error("Failed to save RON:", error)
      // Toast already shown by store, just log
    }
  }

  return (
    <div className={styles.toolbar}>
      <div className={styles.toolbarSection}>
        <h1 className={styles.toolbarTitle}>Carcinisation Stage Editor</h1>
        {fileName && (
          <span className={styles.toolbarFilename}>
            {fileName}
            {isDirty && <span className={styles.toolbarDirty}> *</span>}
          </span>
        )}
      </div>
      <div className={styles.toolbarSection}>
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
      <div className={styles.toolbarSection}>
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
