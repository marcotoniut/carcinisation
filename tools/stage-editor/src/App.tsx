import { ConsolePanel } from "./components/Console/ConsolePanel"
import { InspectorPanel } from "./components/Inspector/InspectorPanel"
import { PalettePanel } from "./components/Palette/PalettePanel"
import { ResizeHandle } from "./components/ResizeHandle/ResizeHandle"
import { ScenesPanel } from "./components/Scenes/ScenesPanel"
import { Timeline } from "./components/Timeline/Timeline"
import { ToastProvider } from "./components/Toast/Toast"
import { Toolbar } from "./components/Toolbar/Toolbar"
import { Viewport } from "./components/Viewport/Viewport"
import { useResizable } from "./hooks/useResizable"
import * as styles from "./styles/App.css"
import "@/theme/global.css"

function App() {
  const rightSidebar = useResizable({
    storageKey: "editor-right-sidebar-width",
    defaultSize: 280,
    minSize: 200,
    maxSize: 500,
    direction: "horizontal",
  })

  const bottomPanel = useResizable({
    storageKey: "editor-bottom-panel-height",
    defaultSize: 200,
    minSize: 100,
    maxSize: 400,
    direction: "vertical",
  })

  const consoleWidth = useResizable({
    storageKey: "editor-console-width",
    defaultSize: 50, // 50% as default
    minSize: 20,
    maxSize: 80,
    direction: "horizontal",
  })

  return (
    <ToastProvider>
      <div className={styles.editorRoot}>
        <Toolbar />
        <div className={styles.editorMain}>
          <div className={styles.editorCenter}>
            <Viewport />
            <Timeline />
          </div>
          <ResizeHandle
            direction="horizontal"
            onMouseDown={rightSidebar.handleMouseDown}
            isResizing={rightSidebar.isResizing}
          />
          <div
            className={styles.editorRightSidebar}
            style={{ width: `${rightSidebar.size}px` }}
          >
            <ScenesPanel />
            <PalettePanel />
          </div>
        </div>
        <ResizeHandle
          direction="vertical"
          onMouseDown={bottomPanel.handleMouseDown}
          isResizing={bottomPanel.isResizing}
        />
        <div
          className={styles.editorBottom}
          style={{ height: `${bottomPanel.size}px` }}
        >
          <div style={{ width: `${consoleWidth.size}%`, minWidth: 0 }}>
            <ConsolePanel />
          </div>
          <ResizeHandle
            direction="horizontal"
            onMouseDown={consoleWidth.handleMouseDown}
            isResizing={consoleWidth.isResizing}
          />
          <div style={{ flex: 1, minWidth: 0 }}>
            <InspectorPanel />
          </div>
        </div>
      </div>
    </ToastProvider>
  )
}

export default App
