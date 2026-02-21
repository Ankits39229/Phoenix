import TitleBar from './components/TitleBar'
import Dashboard from './components/Dashboard'
import Sidebar from './components/Sidebar'

function App() {
  return (
    <div className="h-screen w-screen relative overflow-hidden bg-gradient-to-br from-blue-200 via-purple-100 to-pink-200">
      <TitleBar />
      <div className="flex h-full overflow-hidden">
        <Sidebar />
        <Dashboard />
      </div>
    </div>
  )
}

export default App
