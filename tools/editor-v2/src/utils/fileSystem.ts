/**
 * File System utilities for loading and saving .ron files
 * Uses File System Access API when available, falls back to file input/download
 */

export interface FileHandle {
  name: string
  content: string
  handle?: FileSystemFileHandle
}

/**
 * Open a file picker and load a .ron file
 */
export async function openRonFile(): Promise<FileHandle | null> {
  try {
    // Try File System Access API (Chrome/Edge)
    if (window.showOpenFilePicker) {
      const [fileHandle] = await window.showOpenFilePicker({
        types: [
          {
            description: "RON Files",
            accept: {
              "text/plain": [".ron"],
            },
          },
        ],
        multiple: false,
      })

      const file = await fileHandle.getFile()
      const content = await file.text()

      return {
        name: file.name,
        content,
        handle: fileHandle,
      }
    }

    // Fallback: traditional file input
    return await openFileWithInput()
  } catch (error) {
    if ((error as Error).name !== "AbortError") {
      console.error("Failed to open file:", error)
    }
    return null
  }
}

/**
 * Fallback file picker using <input type="file">
 */
function openFileWithInput(): Promise<FileHandle | null> {
  return new Promise((resolve) => {
    const input = document.createElement("input")
    input.type = "file"
    input.accept = ".ron"

    input.onchange = async () => {
      const file = input.files?.[0]
      if (!file) {
        resolve(null)
        return
      }

      const content = await file.text()
      resolve({
        name: file.name,
        content,
      })
    }

    input.oncancel = () => resolve(null)
    input.click()
  })
}

/**
 * Save content to a file
 * Uses File System Access API when available, falls back to download
 */
export async function saveRonFile(
  content: string,
  fileName: string,
  existingHandle?: FileSystemFileHandle,
): Promise<boolean> {
  try {
    // Try File System Access API (Chrome/Edge)
    if (window.showSaveFilePicker) {
      const handle =
        existingHandle ||
        (await window.showSaveFilePicker({
          suggestedName: fileName,
          types: [
            {
              description: "RON Files",
              accept: {
                "text/plain": [".ron"],
              },
            },
          ],
        }))

      const writable = await handle.createWritable()
      await writable.write(content)
      await writable.close()

      return true
    }

    // Fallback: trigger download
    downloadFile(content, fileName)
    return true
  } catch (error) {
    if ((error as Error).name !== "AbortError") {
      console.error("Failed to save file:", error)
    }
    return false
  }
}

/**
 * Fallback save using download
 */
function downloadFile(content: string, fileName: string) {
  const blob = new Blob([content], { type: "text/plain" })
  const url = URL.createObjectURL(blob)
  const a = document.createElement("a")
  a.href = url
  a.download = fileName
  a.click()
  URL.revokeObjectURL(url)
}

/**
 * Check if File System Access API is supported
 */
export function isFileSystemAccessSupported(): boolean {
  return "showOpenFilePicker" in window && "showSaveFilePicker" in window
}
