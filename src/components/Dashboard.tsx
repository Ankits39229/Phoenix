import { useEffect, useState } from 'react'
import { Lock, Monitor, Download, FolderOpen, HardDrive } from 'lucide-react'

interface DashboardProps {
  onDriveClick: (drive: DriveInfo) => void
}

const Dashboard = ({ onDriveClick }: DashboardProps) => {
  const [drives, setDrives] = useState<DriveInfo[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [specialFolders, setSpecialFolders] = useState<{ desktop: string; downloads: string } | null>(null)

  const init = async () => {
    setLoading(true)
    setError(null)
    try {
      if (window.electron?.getDrives) {
        const [driveData, folders] = await Promise.all([
          window.electron.getDrives(),
          window.electron.getSpecialFolders(),
        ])
        setDrives(driveData)
        setSpecialFolders(folders)
      }
    } catch (err) {
      console.error('Error during init:', err)
      setError(err instanceof Error ? err.message : 'Failed to detect drives.')
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => { init() }, [])

  const formatBytes = (bytes: number) => {
    if (bytes === 0) return '0 GB'
    const gb = bytes / 1024 ** 3
    if (gb >= 1) return `${gb.toFixed(2)} GB`
    const mb = bytes / 1024 ** 2
    return `${mb.toFixed(0)} MB`
  }

  const handleSelectFolder = async () => {
    const folderPath = await window.electron.selectFolder()
    if (!folderPath) return
    const parts = folderPath.replace(/\\/g, '/').split('/')
    const folderName = parts[parts.length - 1] || folderPath
    onDriveClick({
      name: folderName,
      letter: folderPath,
      label: folderPath,
      totalSpace: 0,
      freeSpace: 0,
      usedSpace: 0,
      usedPercentage: 0,
      isBitlocker: false,
      isLocked: false,
      filesystem: 'Folder',
    })
  }

  const makeQuickAccessDrive = (name: string, folderPath: string): DriveInfo => ({
    name,
    letter: folderPath,
    label: folderPath,
    totalSpace: 0,
    freeSpace: 0,
    usedSpace: 0,
    usedPercentage: 0,
    isBitlocker: false,
    isLocked: false,
    filesystem: 'Folder',
  })

  const SectionHeader = ({ title, count }: { title: string; count?: number }) => (
    <div className="flex items-center gap-2 mb-4">
      <h2 className="text-sm font-semibold text-blue-500 tracking-wide">
        {title}
        {count !== undefined && (
          <span className="text-blue-400 font-normal">({count})</span>
        )}
      </h2>
      <div className="flex-1 h-px bg-blue-100/60" />
    </div>
  )

  return (
    <div className="flex-1 overflow-y-auto">
      <div className="max-w-4xl mx-auto px-10 pt-24 pb-16">
        {/* Main heading */}
        <h1 className="text-4xl font-light text-gray-400 mb-10 text-center">
          Select a location to recover files
        </h1>

        {loading ? (
          <div className="flex flex-col items-center gap-4 py-20">
            <div className="w-10 h-10 border-4 border-purple-300 border-t-purple-600 rounded-full animate-spin" />
            <p className="text-gray-400 text-sm">Detecting drives...</p>
          </div>
        ) : error ? (
          <div className="flex flex-col items-center gap-4 py-20">
            <div className="w-full max-w-md bg-red-50 border border-red-200 rounded-2xl px-6 py-5 flex flex-col items-center gap-3 text-center">
              <p className="text-sm font-semibold text-red-600">Could not detect drives</p>
              <p className="text-xs text-red-400">{error}</p>
              <button
                onClick={init}
                className="mt-1 px-4 py-1.5 rounded-lg bg-red-500 text-white text-xs font-semibold hover:bg-red-600 transition-colors"
              >
                Retry
              </button>
            </div>
          </div>
        ) : (
          <div className="flex flex-col gap-8">
            {/* â”€â”€ Hard Disk Drives â”€â”€ */}
            <section>
              <SectionHeader title="Hard Disk Drives" count={drives.length} />
              <div className="grid grid-cols-3 gap-4">
                {drives.length === 0 && (
                  <div className="col-span-3 py-8 flex flex-col items-center gap-2 text-center">
                    <HardDrive size={28} className="text-gray-300" />
                    <p className="text-sm text-gray-400">No drives detected.</p>
                    <button onClick={init} className="text-xs text-blue-400 hover:text-blue-600 underline">Retry</button>
                  </div>
                )}
                {drives.map((drive) => (
                  <button
                    key={drive.name}
                    onClick={() => onDriveClick(drive)}
                    className="group bg-white/55 backdrop-blur-md rounded-2xl p-4 hover:bg-white/75 hover:scale-[1.02] transition-all duration-200 shadow-sm hover:shadow-md flex items-center gap-4 text-left"
                  >
                    {/* HDD icon */}
                    <div className="shrink-0 w-12 h-12 rounded-xl bg-blue-50/80 flex items-center justify-center group-hover:bg-blue-100/80 transition-colors">
                      <HardDrive size={24} className="text-blue-400 group-hover:text-blue-500" />
                    </div>

                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-1.5 mb-0.5">
                        <p className="text-sm font-semibold text-gray-700 truncate">
                          {drive.label || 'Local Disk'} ({drive.name}:)
                        </p>
                        {drive.isBitlocker && (
                          <Lock size={12} className="shrink-0 text-yellow-500" />
                        )}
                      </div>

                      {drive.totalSpace > 0 ? (
                        <>
                          {/* Usage bar */}
                          <div className="w-full bg-gray-200/60 rounded-full h-1.5 mb-1">
                            <div
                              className={`h-1.5 rounded-full transition-all ${
                                drive.usedPercentage > 90
                                  ? 'bg-red-400'
                                  : drive.usedPercentage > 70
                                  ? 'bg-yellow-400'
                                  : 'bg-gradient-to-r from-blue-400 to-blue-500'
                              }`}
                              style={{ width: `${drive.usedPercentage}%` }}
                            />
                          </div>
                          <p className="text-[11px] text-gray-400">
                            {formatBytes(drive.freeSpace)} / {formatBytes(drive.totalSpace)}
                          </p>
                        </>
                      ) : (
                        <p className="text-[11px] text-gray-400">{drive.filesystem}</p>
                      )}
                    </div>
                  </button>
                ))}
              </div>
            </section>

            {/* â”€â”€ Quick Access â”€â”€ */}
            <section>
              <SectionHeader title="Quick Access" />
              <div className="grid grid-cols-3 gap-4">
                {/* Desktop */}
                <button
                  onClick={() =>
                    specialFolders && onDriveClick(makeQuickAccessDrive('Desktop', specialFolders.desktop))
                  }
                  disabled={!specialFolders}
                  className="group bg-white/55 backdrop-blur-md rounded-2xl p-4 hover:bg-white/75 hover:scale-[1.02] transition-all duration-200 shadow-sm hover:shadow-md flex items-center gap-4 text-left disabled:opacity-50 disabled:cursor-not-allowed"
                >
                  <div className="shrink-0 w-12 h-12 rounded-xl bg-purple-50/80 flex items-center justify-center group-hover:bg-purple-100/80 transition-colors">
                    <Monitor size={24} className="text-purple-400 group-hover:text-purple-500" />
                  </div>
                  <div>
                    <p className="text-sm font-semibold text-gray-700">Desktop</p>
                    <p className="text-[11px] text-gray-400 truncate max-w-[140px]">
                      {specialFolders?.desktop || 'Loading...'}
                    </p>
                  </div>
                </button>

                {/* Downloads */}
                <button
                  onClick={() =>
                    specialFolders && onDriveClick(makeQuickAccessDrive('Downloads', specialFolders.downloads))
                  }
                  disabled={!specialFolders}
                  className="group bg-white/55 backdrop-blur-md rounded-2xl p-4 hover:bg-white/75 hover:scale-[1.02] transition-all duration-200 shadow-sm hover:shadow-md flex items-center gap-4 text-left disabled:opacity-50 disabled:cursor-not-allowed"
                >
                  <div className="shrink-0 w-12 h-12 rounded-xl bg-green-50/80 flex items-center justify-center group-hover:bg-green-100/80 transition-colors">
                    <Download size={24} className="text-green-500 group-hover:text-green-600" />
                  </div>
                  <div>
                    <p className="text-sm font-semibold text-gray-700">Downloads</p>
                    <p className="text-[11px] text-gray-400 truncate max-w-[140px]">
                      {specialFolders?.downloads || 'Loading...'}
                    </p>
                  </div>
                </button>

                {/* Select Folder */}
                <button
                  onClick={handleSelectFolder}
                  className="group bg-white/55 backdrop-blur-md rounded-2xl p-4 hover:bg-white/75 hover:scale-[1.02] transition-all duration-200 shadow-sm hover:shadow-md flex items-center gap-4 text-left border-2 border-dashed border-gray-200/80 hover:border-purple-300"
                >
                  <div className="shrink-0 w-12 h-12 rounded-xl bg-orange-50/80 flex items-center justify-center group-hover:bg-orange-100/80 transition-colors">
                    <FolderOpen size={24} className="text-orange-400 group-hover:text-orange-500" />
                  </div>
                  <div>
                    <p className="text-sm font-semibold text-gray-700">Select Folder</p>
                    <p className="text-[11px] text-gray-400">Browse for a folder</p>
                  </div>
                </button>
              </div>
            </section>
          </div>
        )}
      </div>
    </div>
  )
}

export default Dashboard
