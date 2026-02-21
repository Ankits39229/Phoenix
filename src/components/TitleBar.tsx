import { X, Minus, Square } from 'lucide-react'

const TitleBar = () => {
  const handleMinimize = () => {
    if (window.electron) {
      window.electron.minimizeWindow()
    }
  }

  const handleMaximize = () => {
    if (window.electron) {
      window.electron.maximizeWindow()
    }
  }

  const handleClose = () => {
    if (window.electron) {
      window.electron.closeWindow()
    }
  }

  return (
    <div className="h-14 flex items-center justify-between px-6 drag-region absolute top-0 left-0 right-0 z-10">
      <div className="flex items-center gap-3">
        <div className="w-10 h-10 rounded-full bg-white/50 backdrop-blur-sm flex items-center justify-center shadow-sm">
          <div className="w-5 h-5 rounded-full border-2 border-gray-300"></div>
        </div>
      </div>
      
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
