import { useState } from 'react'
import { Zap, ScanSearch, HardDrive, X, ChevronRight } from 'lucide-react'
import TitleBar, { NavPage } from './components/TitleBar'
import Dashboard from './components/Dashboard'
import ScanView from './components/ScanView'
import Settings from './components/Settings'
import About from './components/About'
import { ThemeProvider } from './context/ThemeContext'

type View = 'dashboard' | 'scanning'
export type ScanMode = 'quick' | 'deep'

interface PreScanModalProps {
  drive: DriveInfo
  onConfirm: (mode: ScanMode) => void
  onCancel: () => void
}

function PreScanModal({ drive, onConfirm, onCancel }: PreScanModalProps) {
  const [selected, setSelected] = useState<ScanMode>('quick')

  const modes: { id: ScanMode; label: string; tagline: string; time: string; icon: React.ReactNode; bullets: string[]; color: string; border: string; activeBg: string }[] = [
    {
      id: 'quick',
      label: 'Quick Scan',
      tagline: 'Recently deleted files',
      time: 'Seconds – minutes',
      icon: <Zap size={20} />,
      bullets: [
        'Reads up to 100,000 MFT records',
        'Finds deleted files with intact directory entries',
        'Preserves original file names and paths',
        'Best for accidental deletes & Recycle Bin recovery',
      ],
      color: 'text-blue-500',
      border: 'border-blue-400',
      activeBg: 'bg-blue-50/80',
    },
    {
      id: 'deep',
      label: 'Deep Scan',
      tagline: 'Formatted or long-deleted files',
      time: 'Minutes – hours',
      icon: <ScanSearch size={20} />,
      bullets: [
        'Reads up to 500,000 MFT records (5× more)',
        'Sector-by-sector file signature carving',
        'Recovers files with no MFT trace remaining',
        'Best for formatted drives or old deletions',
      ],
      color: 'text-purple-500',
      border: 'border-purple-400',
      activeBg: 'bg-purple-50/80',
    },
  ]

  return (
    <div className="absolute inset-0 z-50 flex items-center justify-center theme-overlay">
      <div className="w-[520px] rounded-2xl theme-modal backdrop-blur-md shadow-2xl overflow-hidden">
        {/* Header */}
        <div className="px-6 py-4 flex items-center justify-between border-b border-gray-100">
          <div className="flex items-center gap-3">
            <div className="w-9 h-9 rounded-xl bg-blue-50 flex items-center justify-center">
              <HardDrive size={18} className="text-blue-500" />
            </div>
            <div>
              <p className="text-sm font-bold text-gray-800">
                {drive.label || 'Local Disk'}{drive.name.length <= 2 ? ` (${drive.name}:)` : ''}
              </p>
              <p className="text-[11px] text-gray-400">{drive.filesystem || 'NTFS'}</p>
            </div>
          </div>
          <button onClick={onCancel} className="w-8 h-8 flex items-center justify-center rounded-full hover:bg-gray-100 text-gray-400 hover:text-gray-600 transition-colors">
            <X size={16} />
          </button>
        </div>

        {/* Body */}
        <div className="px-6 py-5">
          <p className="text-xs text-gray-500 mb-4 font-medium uppercase tracking-wide">Choose scan mode</p>
          <div className="flex flex-col gap-3">
            {modes.map((m) => (
              <button
                key={m.id}
                onClick={() => setSelected(m.id)}
                className={`text-left rounded-xl border-2 p-4 transition-all ${
                  selected === m.id ? `${m.activeBg} ${m.border}` : 'border-gray-200 hover:border-gray-300 bg-white/60'
                }`}
              >
                <div className="flex items-start gap-3">
                  <div className={`mt-0.5 shrink-0 ${selected === m.id ? m.color : 'text-gray-400'}`}>{m.icon}</div>
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2 mb-0.5">
                      <span className={`text-sm font-bold ${selected === m.id ? m.color : 'text-gray-700'}`}>{m.label}</span>
                      <span className="text-[10px] bg-gray-100 text-gray-500 px-1.5 py-0.5 rounded-full">{m.time}</span>
                    </div>
                    <p className="text-xs text-gray-500 mb-2">{m.tagline}</p>
                    <ul className="flex flex-col gap-1">
                      {m.bullets.map((b) => (
                        <li key={b} className="flex items-start gap-1.5 text-[11px] text-gray-500">
                          <span className={`mt-0.5 shrink-0 ${selected === m.id ? m.color : 'text-gray-300'}`}>•</span>
                          {b}
                        </li>
                      ))}
                    </ul>
                  </div>
                </div>
              </button>
            ))}
          </div>
        </div>

        {/* Footer */}
        <div className="px-6 pb-5 flex gap-3">
          <button
            onClick={onCancel}
            className="flex-1 py-2.5 rounded-xl border border-gray-200 text-gray-500 text-sm font-medium hover:bg-gray-50 transition-all"
          >
            Cancel
          </button>
          <button
            onClick={() => onConfirm(selected)}
            className="flex-1 py-2.5 rounded-xl bg-gradient-to-r from-blue-500 to-purple-500 text-white text-sm font-semibold hover:shadow-md hover:scale-[1.01] active:scale-95 transition-all flex items-center justify-center gap-1.5"
          >
            Start {selected === 'quick' ? 'Quick' : 'Deep'} Scan
            <ChevronRight size={15} />
          </button>
        </div>
      </div>
    </div>
  )
}

function App() {
  const [view, setView] = useState<View>('dashboard')
  const [page, setPage] = useState<NavPage>('dashboard')
  const [selectedDrive, setSelectedDrive] = useState<DriveInfo | null>(null)
  const [pendingDrive, setPendingDrive] = useState<DriveInfo | null>(null)
  const [scanMode, setScanMode] = useState<ScanMode>('quick')

  const handleNavigate = (dest: NavPage) => {
    // Don't allow navigating away mid-scan
    if (view === 'scanning') return
    setPage(dest)
  }

  const handleDriveClick = (drive: DriveInfo) => {
    setPendingDrive(drive)
  }

  const handleConfirmScan = (mode: ScanMode) => {
    setScanMode(mode)
    setSelectedDrive(pendingDrive)
    setPendingDrive(null)
    setView('scanning')
  }

  const handleCancelModal = () => {
    setPendingDrive(null)
  }

  const handleBackToDashboard = () => {
    setView('dashboard')
    setSelectedDrive(null)
  }

  const activePage: NavPage = view === 'scanning' ? 'dashboard' : page

  return (
    <ThemeProvider>
    <div className="h-screen w-screen relative overflow-hidden app-bg">
      <TitleBar currentPage={activePage} onNavigate={handleNavigate} />
      <div className="flex h-full overflow-hidden">
        {view === 'scanning' ? (
          <ScanView drive={selectedDrive!} scanMode={scanMode} onBack={handleBackToDashboard} />
        ) : page === 'settings' ? (
          <Settings />
        ) : page === 'about' ? (
          <About />
        ) : (
          <Dashboard onDriveClick={handleDriveClick} />
        )}
      </div>
      {pendingDrive && (
        <PreScanModal
          drive={pendingDrive}
          onConfirm={handleConfirmScan}
          onCancel={handleCancelModal}
        />
      )}
    </div>
    </ThemeProvider>
  )
}

export default App
