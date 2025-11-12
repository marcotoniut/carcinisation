import { ConsolePanel } from "./components/Console/ConsolePanel"
import { HierarchyPanel } from "./components/Hierarchy/HierarchyPanel"
import { InspectorPanel } from "./components/Inspector/InspectorPanel"
import { PalettePanel } from "./components/Palette/PalettePanel"
import { ScenesPanel } from "./components/Scenes/ScenesPanel"
import { Timeline } from "./components/Timeline/Timeline"
import { ToastProvider } from "./components/Toast/Toast"
import { Toolbar } from "./components/Toolbar/Toolbar"
import { Viewport } from "./components/Viewport/Viewport"
import * as styles from "./styles/App.css"
import "@/theme/global.css"

function App() {
  return (
    <ToastProvider>
      <div className={styles.editorRoot}>
        <Toolbar />
        <div className={styles.editorMain}>
          <div className={styles.editorLeftSidebar}>
            <ScenesPanel />
            <PalettePanel />
          </div>
          <div className={styles.editorCenter}>
            <Viewport />
            <Timeline />
          </div>
          <div className={styles.editorRightSidebar}>
            <HierarchyPanel />
            <InspectorPanel />
          </div>
        </div>
        <ConsolePanel />
      </div>
    </ToastProvider>
  )
}

export default App
