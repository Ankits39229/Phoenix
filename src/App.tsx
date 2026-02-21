import { useState } from 'react'
import TitleBar from './components/TitleBar'
import Dashboard from './components/Dashboard'
import ScanView from './components/ScanView'

type View = 'dashboard' | 'scanning'

function App() {
  const [view, setView] = useState<View>('dashboard')
  const [selectedDrive, setSelectedDrive] = useState<DriveInfo | null>(null)

  const handleDriveClick = (drive: DriveInfo) => {
    setSelectedDrive(drive)
    setView('scanning')
  }

  const handleBackToDashboard = () => {
    setView('dashboard')
    setSelectedDrive(null)
  }

  return (
    <div className="h-screen w-screen relative overflow-hidden bg-gradient-to-br from-blue-200 via-purple-100 to-pink-200">
      <TitleBar />
      <div className="flex h-full overflow-hidden">
        {view === 'dashboard' ? (
          <Dashboard onDriveClick={handleDriveClick} />
        ) : (
          <ScanView drive={selectedDrive!} onBack={handleBackToDashboard} />
        )}
      </div>
    </div>
  )
}

export default App
