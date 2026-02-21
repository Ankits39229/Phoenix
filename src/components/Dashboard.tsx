import { useEffect, useState } from 'react'
import { 
  HardDrive,
  Mic
} from 'lucide-react'

interface DriveInfo {
  name: string
  label: string
  used: number
  free: number
  total: number
  usedPercentage: number
}

const Dashboard = () => {
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
    return `${gb.toFixed(2)} GB`
  }

  return (
    <div className="flex-1 relative overflow-hidden">
      {/* Content */}
      <div className="relative h-full flex flex-col items-center justify-center px-12 pt-24 pb-16">
        {/* Main heading */}
        <h1 className="text-5xl font-light text-gray-400 mb-20">
          What I can do for you?
        </h1>
        
        {/* Drives Grid */}
        <div className="w-full max-w-5xl">
          {loading ? (
            <div className="text-center text-gray-400">Loading drives...</div>
          ) : (
            <div className="grid grid-cols-4 gap-5">
              {drives.map((drive) => (
                <button
                  key={drive.name}
                  className="group relative bg-white/50 backdrop-blur-md rounded-[2rem] p-6 hover:bg-white/70 transition-all duration-200 shadow-sm hover:shadow-md flex flex-col items-start justify-between h-36"
                >
                  <div className="text-gray-400 group-hover:text-gray-600 transition-colors">
                    <HardDrive size={24} />
                  </div>
                  <div className="w-full">
                    <p className="text-lg font-semibold text-gray-700 group-hover:text-gray-900 transition-colors text-left mb-1">
                      {drive.name}:
                    </p>
                    <p className="text-xs text-gray-500 group-hover:text-gray-700 transition-colors text-left">
                      {drive.total > 0 ? formatBytes(drive.free) : 'N/A'} free
                    </p>
                    {drive.total > 0 && (
                      <div className="mt-2 w-full bg-gray-200/50 rounded-full h-1.5">
                        <div 
                          className="bg-gradient-to-r from-blue-400 to-purple-500 h-1.5 rounded-full transition-all"
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
