import { X, Minus, Square, HardDrive, Settings, Info } from 'lucide-react'

export type NavPage = 'dashboard' | 'settings' | 'about'

interface TitleBarProps {
  currentPage?: NavPage
  onNavigate?: (page: NavPage) => void
}

const navItems: { id: NavPage; label: string; icon: React.ReactNode }[] = [
  { id: 'dashboard', label: 'Recover', icon: <HardDrive size={14} /> },
  { id: 'settings', label: 'Settings', icon: <Settings size={14} /> },
  { id: 'about',    label: 'About',    icon: <Info size={14} /> },
]

const TitleBar = ({ currentPage = 'dashboard', onNavigate }: TitleBarProps) => {
  const handleMinimize = () => {
    if (window.electron) window.electron.minimizeWindow()
  }
  const handleMaximize = () => {
    if (window.electron) window.electron.maximizeWindow()
  }
  const handleClose = () => {
    if (window.electron) window.electron.closeWindow()
  }

  return (
    <div className="h-14 flex items-center justify-between px-6 drag-region absolute top-0 left-0 right-0 z-10">
      {/* Logo */}
      <div className="flex items-center gap-3">
        <div className="w-10 h-10 rounded-full bg-white/50 backdrop-blur-sm flex items-center justify-center shadow-sm">
          <div className="w-5 h-5 rounded-full border-2 border-gray-300"></div>
        </div>
      </div>

      {/* Nav tabs */}
      <div className="flex items-center gap-1 no-drag theme-nav-pill rounded-full px-1.5 py-1 shadow-sm">
        {navItems.map((item) => {
          const active = currentPage === item.id
          return (
            <button
              key={item.id}
              onClick={() => onNavigate?.(item.id)}
              className={`flex items-center gap-1.5 px-3.5 py-1.5 rounded-full text-xs font-medium transition-all duration-150 ${
                active
                  ? 'theme-nav-active'
                  : 'hover:bg-white/50'
              }`}
              style={active ? undefined : { color: 'var(--nav-text)' }}
            >
              {item.icon}
              {item.label}
            </button>
          )
        })}
      </div>

      {/* Window controls */}
      <div className="flex items-center gap-3 no-drag">
        <button
          onClick={handleMinimize}
          className="w-10 h-10 flex items-center justify-center hover:bg-white/60 rounded-full transition-colors"
          title="Minimize"
        >
          <Minus size={18} className="text-gray-600" />
        </button>
        <button
          onClick={handleMaximize}
          className="w-10 h-10 flex items-center justify-center hover:bg-white/60 rounded-full transition-colors"
          title="Maximize"
        >
          <Square size={16} className="text-gray-600" />
        </button>
        <button
          onClick={handleClose}
          className="w-10 h-10 flex items-center justify-center hover:bg-red-500/80 rounded-full transition-colors group"
          title="Close"
        >
          <X size={18} className="text-gray-600 group-hover:text-white" />
        </button>
      </div>
    </div>
  )
}

export default TitleBar
