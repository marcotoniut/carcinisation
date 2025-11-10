import { create } from "zustand"
import type { StageData } from "@/types/generated/StageData"
import { parseStageFile } from "../utils/parseStageFile"

interface EditorState {
  fileName: string | null
  fileContent: string | null
  fileHandle: FileSystemFileHandle | null
  parsedData: StageData | null
  parseError: string | null
  consoleMessage: string | null
  isDirty: boolean

  loadFile: (
    name: string,
    content: string,
    handle?: FileSystemFileHandle,
  ) => Promise<void>
  updateContent: (content: string) => void
  markClean: () => void
  markSaved: (content: string) => void
  saveToRon: () => Promise<string>
}

export const useEditorStore = create<EditorState>((set, get) => ({
  fileName: null,
  fileContent: null,
  fileHandle: null,
  parsedData: null,
  parseError: null,
  consoleMessage: "Stage Editor ready. Load a .ron file to begin.",
  isDirty: false,

  loadFile: async (name, content, handle) => {
    try {
      const parsedData = await parseStageFile(content)
      set({
        fileName: name,
        fileContent: content,
        fileHandle: handle || null,
        parsedData,
        parseError: null,
        consoleMessage: `Loaded ${name} successfully.`,
        isDirty: false,
      })
      console.log("Loaded stage data", { fileName: name, parsedData })
    } catch (error) {
      const message =
        error instanceof Error
          ? error.message
          : error
            ? String(error)
            : "Unknown error"
      set({
        parsedData: null,
        parseError: message,
        consoleMessage: `Parse error: ${message}`,
        isDirty: false,
      })
      throw error
    }
  },

  updateContent: (content) =>
    set({
      fileContent: content,
      isDirty: true,
    }),

  markClean: () => set({ isDirty: false }),

  markSaved: (content) => {
    const { fileName } = get()
    set({
      fileContent: content,
      isDirty: false,
      consoleMessage: `Saved ${fileName ?? "stage data"} successfully.`,
    })
  },

  saveToRon: async () => {
    const { parsedData } = get()
    if (!parsedData) {
      const message = "No parsed stage data available to save."
      set({ consoleMessage: message })
      throw new Error(message)
    }

    const response = await fetch("/api/json-to-ron", {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify(parsedData),
    })

    const ronText = await response.text()

    if (!response.ok) {
      const message = ronText || response.statusText
      set({ consoleMessage: `Save failed: ${message}` })
      throw new Error(message)
    }

    return ronText
  },
}))
