import { useEffect, useState } from 'react'
import { 
  HardDrive,
  Mic,
  Lock
} from 'lucide-react'

interface DashboardProps {
  onDriveClick: (drive: DriveInfo) => void
}

const Dashboard = ({ onDriveClick }: DashboardProps) => {
  const [drives, setDrives] = useState<DriveInfo[]>([])
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    const fetchDrives = async () => {
      try {
        if (window.electron?.getDrives) {
          const driveData = await window.electron.getDrives()
          setDrives(driveData)
        }
      } catch (error) {
        console.error('Error fetching drives:', error)
      } finally {
        setLoading(false)
      }
    }

    fetchDrives()
  }, [])

  const formatBytes = (bytes: number) => {
    if (bytes === 0) return '0 GB'
    const gb = bytes / (1024 ** 3)
    if (gb >= 1) return `${gb.toFixed(1)} GB`
    const mb = bytes / (1024 ** 2)
    return `${mb.toFixed(0)} MB`
  }

  return (
    <div className="flex-1 relative overflow-hidden">
      {/* Content */}
      <div className="relative h-full flex flex-col items-center justify-center px-12 pt-24 pb-16">
        {/* Main heading */}
        <h1 className="text-5xl font-light text-gray-400 mb-20">
          Select a drive to scan
        </h1>
        
        {/* Drives Grid */}
        <div className="w-full max-w-5xl">
          {loading ? (
            <div className="flex flex-col items-center gap-4">
              <div className="w-10 h-10 border-4 border-purple-300 border-t-purple-600 rounded-full animate-spin"></div>
              <p className="text-gray-400">Detecting drives...</p>
            </div>
          ) : (
            <div className="grid grid-cols-4 gap-5">
              {drives.map((drive) => (
                <button
                  key={drive.name}
                  onClick={() => onDriveClick(drive)}
                  className="group relative bg-white/50 backdrop-blur-md rounded-[2rem] p-6 hover:bg-white/70 hover:scale-[1.02] transition-all duration-200 shadow-sm hover:shadow-md flex flex-col items-start justify-between h-36"
                >
                  <div className="flex items-center justify-between w-full">
                    <div className="text-gray-400 group-hover:text-purple-500 transition-colors">
                      <HardDrive size={24} />
                    </div>
                    <div className="flex items-center gap-1">
                      {drive.isBitlocker && (
                        <Lock size={14} className="text-yellow-500" />
                      )}
                      {drive.filesystem !== 'Unknown' && (
                        <span className="text-[10px] text-gray-400 bg-gray-100/60 px-1.5 py-0.5 rounded-md">
                          {drive.filesystem}
                        </span>
                      )}
                    </div>
                  </div>
                  <div className="w-full">
                    <p className="text-lg font-semibold text-gray-700 group-hover:text-gray-900 transition-colors text-left mb-0.5">
                      {drive.name}: <span className="text-sm font-normal text-gray-400">{drive.label}</span>
                    </p>
                    <p className="text-xs text-gray-500 group-hover:text-gray-700 transition-colors text-left">
                      {drive.totalSpace > 0 ? `${formatBytes(drive.freeSpace)} free of ${formatBytes(drive.totalSpace)}` : 'N/A'}
                    </p>
                    {drive.totalSpace > 0 && (
                      <div className="mt-2 w-full bg-gray-200/50 rounded-full h-1.5">
                        <div 
                          className={`h-1.5 rounded-full transition-all ${
                            drive.usedPercentage > 90 ? 'bg-red-400' : 
                            drive.usedPercentage > 70 ? 'bg-yellow-400' :
                            'bg-gradient-to-r from-blue-400 to-purple-500'
                          }`}
                          style={{ width: `${drive.usedPercentage}%` }}
                        ></div>
                      </div>
                    )}
                  </div>
                </button>
              ))}
            </div>
          )}
        </div>
        
        {/* Voice input hint */}
        <div className="absolute bottom-12 right-16 flex items-center gap-3 text-gray-400 text-sm">
          <span>Press and hold</span>
          <kbd className="px-3 py-1.5 bg-white/50 backdrop-blur-sm rounded-xl text-gray-500 font-medium text-xs">
            S
          </kbd>
          <span>to speak</span>
          <Mic size={18} className="text-gray-400" />
        </div>
      </div>
    </div>
  )
}

export default Dashboard
