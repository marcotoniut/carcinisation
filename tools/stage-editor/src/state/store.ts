import { create } from "zustand"
import { persist } from "zustand/middleware"
import type { StageData } from "@/types/generated/StageData"
import { showToast } from "../components/Toast/Toast"
import { parseStageFile } from "../utils/parseStageFile"

export type SpawnId = {
  type: "enemy" | "object" | "pickup" | "destructible"
  index: number
}

export type EntityAnimationState = {
  spawnId: SpawnId
  currentAnimation: string
}

interface EditorState {
  fileName: string | null
  fileContent: string | null
  fileHandle: FileSystemFileHandle | null
  parsedData: StageData | null
  parseError: string | null
  consoleMessage: string | null
  isDirty: boolean
  timelinePosition: number // Current time in seconds
  debugMode: boolean
  selectedSpawn: SpawnId | null
  entityAnimations: Map<string, string> // Map of "type:index" to animation name

  loadFile: (
    name: string,
    content: string,
    handle?: FileSystemFileHandle,
  ) => Promise<void>
  updateContent: (content: string) => void
  markClean: () => void
  markSaved: (content: string) => void
  saveToRon: () => Promise<string>
  setTimelinePosition: (position: number) => void
  toggleDebugMode: () => void
  selectSpawn: (spawnId: SpawnId | null) => void
  setEntityAnimation: (spawnId: SpawnId, animation: string) => void
  getEntityAnimation: (spawnId: SpawnId) => string | undefined
}

export const useEditorStore = create<EditorState>()(
  persist(
    (set, get) => ({
      fileName: null,
      fileContent: null,
      fileHandle: null,
      parsedData: null,
      parseError: null,
      consoleMessage: "Stage Editor ready. Load a .ron file to begin.",
      isDirty: false,
      timelinePosition: 0,
      debugMode: true,
      selectedSpawn: null,
      entityAnimations: new Map(),

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
          showToast(`Loaded ${name}`, undefined, "success")
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
          showToast("Failed to load file", message, "error")
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
        showToast(`Saved ${fileName ?? "stage"}`, undefined, "success")
      },

      saveToRon: async () => {
        const { parsedData } = get()
        if (!parsedData) {
          const message = "No parsed stage data available to save."
          set({ consoleMessage: message })
          showToast("Cannot save", message, "error")
          throw new Error(message)
        }

        try {
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
            showToast("Failed to save", message, "error")
            throw new Error(message)
          }

          return ronText
        } catch (error) {
          const message =
            error instanceof Error ? error.message : "Unknown error"
          showToast("Failed to save", message, "error")
          throw error
        }
      },

      setTimelinePosition: (position) => set({ timelinePosition: position }),

      toggleDebugMode: () => set((state) => ({ debugMode: !state.debugMode })),

      selectSpawn: (spawnId) => set({ selectedSpawn: spawnId }),

      setEntityAnimation: (spawnId, animation) =>
        set((state) => {
          const key = `${spawnId.type}:${spawnId.index}`
          const newAnimations = new Map(state.entityAnimations)
          newAnimations.set(key, animation)
          return { entityAnimations: newAnimations }
        }),

      getEntityAnimation: (spawnId) => {
        const key = `${spawnId.type}:${spawnId.index}`
        return get().entityAnimations.get(key)
      },
    }),
    {
      name: "stage-editor-storage",
      version: 1,
      partialize: (state) => ({
        fileName: state.fileName,
        fileContent: state.fileContent,
        parsedData: state.parsedData,
        // Exclude fileHandle (not serializable), parseError, consoleMessage, isDirty
      }),
      onRehydrateStorage: () => (state) => {
        if (state?.parsedData && state?.fileName) {
          state.consoleMessage = `Restored ${state.fileName} from previous session.`
          console.log("Restored from localStorage:", {
            fileName: state.fileName,
            parsedData: state.parsedData,
          })
          showToast(
            "Session restored",
            `Loaded ${state.fileName} from previous session`,
            "info",
          )
        }
      },
    },
  ),
)
