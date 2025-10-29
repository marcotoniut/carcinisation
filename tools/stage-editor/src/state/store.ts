import { create } from "zustand"

interface EditorState {
  // File state
  fileName: string | null
  fileContent: string | null
  isDirty: boolean

  // Actions
  loadFile: (name: string, content: string) => void
  updateContent: (content: string) => void
  markClean: () => void
  reset: () => void
}

export const useEditorStore = create<EditorState>((set) => ({
  fileName: null,
  fileContent: null,
  isDirty: false,

  loadFile: (name, content) =>
    set({
      fileName: name,
      fileContent: content,
      isDirty: false,
    }),

  updateContent: (content) =>
    set({
      fileContent: content,
      isDirty: true,
    }),

  markClean: () => set({ isDirty: false }),

  reset: () =>
    set({
      fileName: null,
      fileContent: null,
      isDirty: false,
    }),
}))
