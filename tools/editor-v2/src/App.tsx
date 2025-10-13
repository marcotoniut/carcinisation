import { Toolbar } from './components/Toolbar/Toolbar'
import { Viewport } from './components/Viewport/Viewport'
import { Timeline } from './components/Timeline/Timeline'
import { ScenesPanel } from './components/Scenes/ScenesPanel'
import { PalettePanel } from './components/Palette/PalettePanel'
import { HierarchyPanel } from './components/Hierarchy/HierarchyPanel'
import { InspectorPanel } from './components/Inspector/InspectorPanel'
import { ConsolePanel } from './components/Console/ConsolePanel'
import './styles/App.css'

function App() {
  return (
    <div className="editor-root">
      <Toolbar />
      <div className="editor-main">
        <div className="editor-left-sidebar">
          <ScenesPanel />
          <PalettePanel />
        </div>
        <div className="editor-center">
          <Viewport />
          <Timeline />
        </div>
        <div className="editor-right-sidebar">
          <HierarchyPanel />
          <InspectorPanel />
        </div>
      </div>
      <ConsolePanel />
    </div>
  )
}

export default App
