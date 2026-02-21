import { useEffect, useState, useRef } from 'react'
import {
  ChevronLeft,
  ChevronRight,
  LayoutGrid,
  List,
  AlignJustify,
  SlidersHorizontal,
  Search,
  Image,
  Video,
  Music,
  FileText,
  Mail,
  Database,
  Globe,
  Archive,
  FolderOpen,
  File,
  Gamepad2,
  FileCode,
  Pause,
  Square,
} from 'lucide-react'

// â”€â”€â”€ Types â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
type FileCategory =
  | 'Photo'
  | 'Video'
  | 'Audio'
  | 'Document'
  | 'Email'
  | 'Database'
  | 'Webfiles'
  | 'Archive'
  | 'Others'
  | 'Unsaved'
  | 'Game'

type SidebarTab = 'location' | 'type'
type ViewMode = 'grid' | 'list' | 'detail'

interface ScanViewProps {
  drive: DriveInfo
  onBack: () => void
}

// â”€â”€â”€ Category config â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
const CATEGORIES: {
  name: FileCategory
  icon: React.ReactNode
  exts: string[]
}[] = [
  { name: 'Photo',    icon: <Image size={16} />,    exts: ['jpg','jpeg','png','gif','bmp','svg','webp','ico','tiff','raw','cr2','nef'] },
  { name: 'Video',    icon: <Video size={16} />,    exts: ['mp4','avi','mkv','mov','wmv','flv','m4v','webm','rmvb'] },
  { name: 'Audio',    icon: <Music size={16} />,    exts: ['mp3','wav','flac','aac','ogg','wma','m4a','opus'] },
  { name: 'Document', icon: <FileText size={16} />, exts: ['doc','docx','pdf','txt','rtf','csv','xls','xlsx','ppt','pptx','odt','ods','odp'] },
  { name: 'Email',    icon: <Mail size={16} />,     exts: ['eml','msg','pst','mbox','ost'] },
  { name: 'Database', icon: <Database size={16} />, exts: ['db','sqlite','sqlite3','mdb','accdb','sql','frm','ibd'] },
  { name: 'Webfiles', icon: <Globe size={16} />,    exts: ['html','htm','css','js','ts','php','asp','aspx','xml','json','jsx','tsx'] },
  { name: 'Archive',  icon: <Archive size={16} />,  exts: ['zip','rar','7z','tar','gz','bz2','xz','iso','cab'] },
  { name: 'Others',   icon: <File size={16} />,     exts: [] },
  { name: 'Unsaved',  icon: <FileCode size={16} />, exts: [] },
  { name: 'Game',     icon: <Gamepad2 size={16} />, exts: ['pak','unity3d','big','bsa','esm','esp','gcf','vpk'] },
]

// â”€â”€â”€ Helpers â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
const formatBytes = (bytes: number): string => {
  if (!bytes || bytes === 0) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  const i = Math.floor(Math.log(bytes) / Math.log(1024))
  return `${(bytes / Math.pow(1024, i)).toFixed(2)} ${units[i]}`
}

const getCategory = (file: RecoverableFile): FileCategory => {
  const ext = (file.extension || '').toLowerCase().replace('.', '')
  for (const cat of CATEGORIES) {
    if (cat.exts.length > 0 && cat.exts.includes(ext)) return cat.name
  }
  return 'Others'
}

// â”€â”€â”€ Circular Progress ring â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
const CircularProgress = ({ pct }: { pct: number }) => {
  const r = 20
  const circ = 2 * Math.PI * r
  const dash = ((pct || 0) / 100) * circ
  return (
    <svg width="52" height="52" viewBox="0 0 52 52" className="shrink-0">
      <circle cx="26" cy="26" r={r} fill="none" stroke="#e5e7eb" strokeWidth="5" />
      <circle
        cx="26" cy="26" r={r}
        fill="none"
        stroke="url(#cpg)"
        strokeWidth="5"
        strokeDasharray={`${dash} ${circ - dash}`}
        strokeLinecap="round"
        transform="rotate(-90 26 26)"
      />
      <defs>
        <linearGradient id="cpg" x1="0%" y1="0%" x2="100%" y2="0%">
          <stop offset="0%" stopColor="#60a5fa" />
          <stop offset="100%" stopColor="#a78bfa" />
        </linearGradient>
      </defs>
      <text x="26" y="30" textAnchor="middle" fontSize="9" fill="#374151" fontWeight="700">
        {Math.round(pct || 0)}%
      </text>
    </svg>
  )
}

// â”€â”€â”€ Thumb color map â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
const THUMB: Record<FileCategory, string> = {
  Photo:    'bg-green-100 text-green-500',
  Video:    'bg-blue-100 text-blue-500',
  Audio:    'bg-pink-100 text-pink-500',
  Document: 'bg-orange-100 text-orange-500',
  Email:    'bg-yellow-100 text-yellow-600',
  Database: 'bg-cyan-100 text-cyan-600',
  Webfiles: 'bg-teal-100 text-teal-600',
  Archive:  'bg-violet-100 text-violet-500',
  Others:   'bg-gray-100 text-gray-400',
  Unsaved:  'bg-gray-100 text-gray-300',
  Game:     'bg-red-100 text-red-400',
}

// â”€â”€â”€ File Card â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
const FileCard = ({
  file, checked, onToggle,
}: { file: RecoverableFile; checked: boolean; onToggle: () => void }) => {
  const cat = getCategory(file)
  const catCfg = CATEGORIES.find((c) => c.name === cat)!
  return (
    <div
      className={`group relative rounded-xl overflow-hidden bg-white/60 backdrop-blur-sm shadow-sm hover:shadow-md transition-all flex flex-col cursor-pointer border-2 ${
        checked ? 'border-blue-400' : 'border-transparent hover:border-blue-200'
      }`}
      onClick={onToggle}
    >
      {/* Thumbnail */}
      <div className={`h-20 flex items-center justify-center ${THUMB[cat]}`}>
        <div className="opacity-70 scale-150">{catCfg.icon}</div>
      </div>
      {/* Checkbox */}
      <div className="absolute top-1.5 left-1.5" onClick={(e) => { e.stopPropagation(); onToggle() }}>
        <div className={`w-4 h-4 rounded border flex items-center justify-center transition-all ${
          checked ? 'bg-blue-500 border-blue-500' : 'bg-white/80 border-gray-300 opacity-0 group-hover:opacity-100'
        }`}>
          {checked && (
            <svg viewBox="0 0 12 12" fill="none" className="w-2.5 h-2.5">
              <path d="M2 6l3 3 5-5" stroke="white" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
            </svg>
          )}
        </div>
      </div>
      {/* Deleted badge */}
      {file.is_deleted && (
        <div className="absolute top-1.5 right-1.5 w-3.5 h-3.5 rounded-full bg-red-400/80 flex items-center justify-center">
          <svg viewBox="0 0 12 12" fill="none" className="w-2 h-2">
            <path d="M3 3h6M5 3V2h2v1M4 5v3M8 5v3M3.5 3l.5 6h4l.5-6" stroke="white" strokeWidth="1.2" strokeLinecap="round" />
          </svg>
        </div>
      )}
      {/* Name */}
      <div className="px-2 py-1.5 bg-white/80 border-t border-gray-100/60">
        <p className="text-[10px] text-gray-600 truncate font-medium" title={file.name}>{file.name || 'Unknown'}</p>
        <p className="text-[9px] text-gray-400 truncate">{formatBytes(file.size)}</p>
      </div>
    </div>
  )
}

const ScanView = ({ drive, onBack }: ScanViewProps) => {
  const [progress, setProgress] = useState<ScanProgress>({
    status: 'scanning',
    message: 'Initializing scan...',
    progress: 0,
    filesFound: 0,
  })
  const [result, setResult] = useState<ScanResult | null>(null)
  const [error, setError] = useState<string | null>(null)

  const [sidebarTab, setSidebarTab] = useState<SidebarTab>('type')
  const [selectedCategory, setSelectedCategory] = useState<FileCategory | null>(null)
  const [viewMode, setViewMode] = useState<ViewMode>('grid')
  const [searchQuery, setSearchQuery] = useState('')
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set())

  const scanStarted = useRef(false)
  const isPaused = useRef(false)

  // â”€â”€ Start scan on mount â”€â”€
  useEffect(() => {
    if (scanStarted.current) return
    scanStarted.current = true

    const cleanup = window.electron.onScanProgress((data: ScanProgress) => {
      setProgress(data)
    })

    window.electron
      .scanDrive(drive.letter, 'quick')
      .then((scanResult: ScanResult) => {
        setResult(scanResult)
        setProgress({ status: 'complete', message: 'Scan complete!', progress: 100, filesFound: scanResult.total_files })
      })
      .catch((err: Error) => {
        setError(err.message || 'Scan failed')
        setProgress({ status: 'error', message: err.message || 'Scan failed', progress: 0 })
      })

    return () => cleanup()
  }, [drive.letter])

  // â”€â”€ Category counts â”€â”€
  const categoryCounts = (() => {
    if (!result) return {} as Record<FileCategory, number>
    const all: RecoverableFile[] = [
      ...(result.mft_entries || []),
      ...(result.carved_files || []),
      ...(result.orphan_files || []),
    ]
    const counts: Record<FileCategory, number> = {
      Photo: 0, Video: 0, Audio: 0, Document: 0, Email: 0,
      Database: 0, Webfiles: 0, Archive: 0, Others: 0, Unsaved: 0, Game: 0,
    }
    for (const f of all) counts[getCategory(f)]++
    return counts
  })()

  // â”€â”€ Filtered display files â”€â”€
  const displayFiles = (() => {
    if (!result) return []
    let files: RecoverableFile[] = [
      ...(result.mft_entries || []),
      ...(result.carved_files || []),
      ...(result.orphan_files || []),
    ]
    if (selectedCategory) files = files.filter((f) => getCategory(f) === selectedCategory)
    if (searchQuery.trim()) {
      const q = searchQuery.toLowerCase()
      files = files.filter(
        (f) => f.name?.toLowerCase().includes(q) || f.path?.toLowerCase().includes(q) || f.extension?.toLowerCase().includes(q)
      )
    }
    return files
  })()

  const totalSize = result?.total_recoverable_size || 0
  const isScanning = progress.status === 'scanning'
  const pct = progress.progress ?? 0

  const handleCancelBack = () => {
    if (isScanning) window.electron.cancelScan()
    onBack()
  }

  const toggleFile = (id: string) => {
    setSelectedIds((prev) => {
      const next = new Set(prev)
      next.has(id) ? next.delete(id) : next.add(id)
      return next
    })
  }

  const toggleAll = () => {
    if (selectedIds.size === displayFiles.length) setSelectedIds(new Set())
    else setSelectedIds(new Set(displayFiles.map((f, i) => f.id || String(i))))
  }

  // â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
  return (
    <div className="flex-1 flex flex-col overflow-hidden">
      {/* â”€â”€ Top Nav Bar â”€â”€ */}
      <div className="flex items-center gap-3 px-5 py-3 mt-16 mx-5 rounded-2xl bg-white/50 backdrop-blur-md shadow-sm shrink-0">
        {/* Home */}
        <button
          onClick={handleCancelBack}
          className="flex items-center gap-1.5 text-blue-500 hover:text-blue-700 text-sm font-medium transition-colors"
        >
          <svg viewBox="0 0 20 20" fill="currentColor" className="w-4 h-4">
            <path d="M10.707 2.293a1 1 0 00-1.414 0l-7 7a1 1 0 001.414 1.414L4 10.414V17a1 1 0 001 1h3a1 1 0 001-1v-3h2v3a1 1 0 001 1h3a1 1 0 001-1v-6.586l.293.293a1 1 0 001.414-1.414l-7-7z" />
          </svg>
          Home
        </button>

        <div className="w-px h-4 bg-gray-200" />

        <button className="p-1 rounded-lg hover:bg-gray-100/60 text-gray-400 hover:text-gray-600 transition-colors">
          <ChevronLeft size={14} />
        </button>
        <button className="p-1 rounded-lg hover:bg-gray-100/60 text-gray-400 hover:text-gray-600 transition-colors">
          <ChevronRight size={14} />
        </button>

        {/* Breadcrumb */}
        <div className="flex items-center gap-1 text-sm text-gray-500 flex-1">
          <span className="hover:text-blue-500 cursor-pointer transition-colors">{drive.name}:</span>
          {selectedCategory && (
            <>
              <ChevronRight size={12} className="text-gray-300" />
              <span className="text-gray-700 font-medium">{selectedCategory}</span>
            </>
          )}
        </div>

        {/* View toggles */}
        <div className="flex items-center gap-0.5 bg-gray-100/60 rounded-lg p-0.5">
          {(
            [
              { mode: 'grid' as ViewMode, icon: <LayoutGrid size={14} /> },
              { mode: 'list' as ViewMode, icon: <List size={14} /> },
              { mode: 'detail' as ViewMode, icon: <AlignJustify size={14} /> },
            ] as { mode: ViewMode; icon: React.ReactNode }[]
          ).map(({ mode, icon }) => (
            <button
              key={mode}
              onClick={() => setViewMode(mode)}
              className={`p-1.5 rounded-md transition-all ${
                viewMode === mode ? 'bg-white shadow-sm text-blue-500' : 'text-gray-400 hover:text-gray-600'
              }`}
            >
              {icon}
            </button>
          ))}
        </div>

        <div className="w-px h-4 bg-gray-200" />

        <button className="flex items-center gap-1.5 text-xs text-gray-500 hover:text-gray-700 bg-gray-100/60 hover:bg-gray-100 rounded-lg px-2.5 py-1.5 transition-all">
          <SlidersHorizontal size={12} />
          Filter
        </button>

        <div className="relative">
          <Search size={12} className="absolute left-2.5 top-1/2 -translate-y-1/2 text-gray-400" />
          <input
            type="text"
            placeholder="Search File"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="pl-7 pr-3 py-1.5 bg-gray-100/60 rounded-lg text-xs text-gray-600 placeholder-gray-400 outline-none focus:bg-white/80 focus:ring-1 focus:ring-blue-300 transition-all w-36"
          />
        </div>
      </div>

      {/* â”€â”€ Body â”€â”€ */}
      <div className="flex flex-1 overflow-hidden gap-3 px-5 py-3">
        {/* Left Sidebar */}
        <div className="w-72 shrink-0 flex flex-col rounded-2xl bg-white/50 backdrop-blur-md shadow-sm overflow-hidden">
          {/* Sidebar Tabs */}
          <div className="flex border-b border-gray-100">
            {(['location', 'type'] as SidebarTab[]).map((tab) => (
              <button
                key={tab}
                onClick={() => setSidebarTab(tab)}
                className={`flex-1 py-2.5 text-xs font-semibold transition-all ${
                  sidebarTab === tab
                    ? 'text-blue-600 border-b-2 border-blue-500 bg-white/60'
                    : 'text-gray-400 hover:text-gray-600'
                }`}
              >
                {tab === 'location' ? 'File Location' : 'File Type'}
              </button>
            ))}
          </div>

          {/* Category list */}
          <div className="flex-1 overflow-y-auto py-1">
            {sidebarTab === 'type' ? (
              CATEGORIES.map((cat) => {
                const count = categoryCounts[cat.name] ?? 0
                const active = selectedCategory === cat.name
                return (
                  <button
                    key={cat.name}
                    onClick={() => setSelectedCategory(active ? null : cat.name)}
                    className={`w-full flex items-center justify-between px-4 py-2.5 text-sm transition-all group ${
                      active ? 'bg-blue-50/80 text-blue-700' : 'hover:bg-white/60 text-gray-600'
                    }`}
                  >
                    <div className="flex items-center gap-2.5">
                      <span className={active ? 'text-blue-500' : 'text-gray-400 group-hover:text-gray-600'}>
                        {cat.icon}
                      </span>
                      <span className={`font-medium ${active ? 'text-blue-700' : ''}`}>{cat.name}</span>
                    </div>
                    <span className={`text-xs tabular-nums ${active ? 'text-blue-500 font-semibold' : 'text-gray-400'}`}>
                      {count.toLocaleString()}
                    </span>
                  </button>
                )
              })
            ) : (
              <div className="flex flex-col items-center justify-center h-full py-12 gap-3">
                <FolderOpen size={28} className="text-gray-300" />
                <p className="text-xs text-gray-400 text-center px-4">
                  {isScanning ? 'Building folder tree...' : 'Select a category to explore'}
                </p>
              </div>
            )}
          </div>

          {/* Drive info */}
          <div className="border-t border-gray-100 px-4 py-3 flex items-center gap-2.5">
            <div className="p-1.5 rounded-lg bg-purple-100">
              <svg viewBox="0 0 20 20" fill="currentColor" className="w-4 h-4 text-purple-500">
                <path d="M2 6a2 2 0 012-2h12a2 2 0 012 2v2a2 2 0 01-2 2H4a2 2 0 01-2-2V6zm14 8a2 2 0 01-2 2H6a2 2 0 01-2-2v-1h12v1z" />
              </svg>
            </div>
            <div className="flex-1 min-w-0">
              <p className="text-xs font-semibold text-gray-700 truncate">{drive.name}: {drive.label || 'Local Disk'}</p>
              <p className="text-[10px] text-gray-400">{drive.filesystem}</p>
            </div>
          </div>
        </div>

        {/* â”€â”€ Right Content â”€â”€ */}
        <div className="flex-1 flex flex-col overflow-hidden rounded-2xl bg-white/50 backdrop-blur-md shadow-sm">
          {/* Select all row */}
          <div className="flex items-center justify-between px-4 py-2.5 border-b border-gray-100/80 shrink-0">
            {result ? (
              <label className="flex items-center gap-2 cursor-pointer" onClick={toggleAll}>
                <div className={`w-4 h-4 rounded border-2 flex items-center justify-center transition-all ${
                  selectedIds.size > 0 && selectedIds.size === displayFiles.length
                    ? 'bg-blue-500 border-blue-500'
                    : selectedIds.size > 0
                    ? 'bg-blue-200 border-blue-400'
                    : 'border-gray-300 hover:border-blue-300'
                }`}>
                  {selectedIds.size > 0 && (
                    <svg viewBox="0 0 12 12" fill="none" className="w-2.5 h-2.5">
                      <path
                        d={selectedIds.size === displayFiles.length ? 'M2 6l3 3 5-5' : 'M2 6h8'}
                        stroke="white" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round"
                      />
                    </svg>
                  )}
                </div>
                <span className="text-xs text-gray-500 font-medium">
                  {selectedIds.size > 0 ? `${selectedIds.size.toLocaleString()} selected` : 'Select All'}
                </span>
              </label>
            ) : (
              <span className="text-xs text-gray-400">
                {isScanning ? 'Scanning...' : error ? 'Scan failed' : 'No results'}
              </span>
            )}
            <span className="text-xs text-gray-400 tabular-nums">
              {displayFiles.length.toLocaleString()} files{selectedCategory ? ` (${selectedCategory})` : ''}
            </span>
          </div>

          {/* Content area */}
          <div className="flex-1 overflow-y-auto p-3">
            {/* Error */}
            {error && !result && (
              <div className="flex flex-col items-center justify-center h-full gap-4">
                <div className="w-14 h-14 rounded-full bg-red-100 flex items-center justify-center">
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" className="w-7 h-7 text-red-500" strokeWidth="1.5">
                    <path strokeLinecap="round" strokeLinejoin="round" d="M12 9v3.75m-9.303 3.376c-.866 1.5.217 3.374 1.948 3.374h14.71c1.73 0 2.813-1.874 1.948-3.374L13.949 3.378c-.866-1.5-3.032-1.5-3.898 0L2.697 16.126zM12 15.75h.007v.008H12v-.008z" />
                  </svg>
                </div>
                <div className="text-center">
                  <p className="text-sm font-semibold text-gray-700">Scan Failed</p>
                  <p className="text-xs text-gray-400 mt-1 max-w-xs">{error}</p>
                  <p className="text-xs text-gray-300 mt-0.5">Try running as administrator for full disk access.</p>
                </div>
              </div>
            )}

            {/* Scanning (no result yet) */}
            {!error && !result && isScanning && (
              <div className="flex flex-col items-center justify-center h-full gap-5">
                <div className="relative">
                  <div className="w-20 h-20 rounded-full border-4 border-blue-100" />
                  <div className="absolute inset-0 rounded-full border-4 border-transparent border-t-blue-400 animate-spin" />
                  <div
                    className="absolute inset-2 rounded-full border-4 border-transparent border-b-purple-400 animate-spin"
                    style={{ animationDirection: 'reverse', animationDuration: '1.4s' }}
                  />
                  <div className="absolute inset-0 flex items-center justify-center">
                    <Search size={22} className="text-purple-400 animate-pulse" />
                  </div>
                </div>
                <div className="text-center space-y-1">
                  <p className="text-sm font-semibold text-gray-600">Scanning {drive.name}:</p>
                  <p className="text-xs text-gray-400">{progress.message}</p>
                  {(progress.filesFound ?? 0) > 0 && (
                    <p className="text-xs text-blue-500 font-semibold">{(progress.filesFound ?? 0).toLocaleString()} files found</p>
                  )}
                </div>
              </div>
            )}

            {/* Grid view */}
            {result && viewMode === 'grid' && (
              <div className="grid grid-cols-6 gap-2">
                {displayFiles.length === 0 ? (
                  <div className="col-span-5 flex flex-col items-center justify-center py-16 gap-3">
                    <FolderOpen size={32} className="text-gray-200" />
                    <p className="text-xs text-gray-400">No files in this category</p>
                  </div>
                ) : (
                  displayFiles.slice(0, 300).map((file, idx) => {
                    const id = file.id || String(idx)
                    return (
                      <FileCard key={id} file={file} checked={selectedIds.has(id)} onToggle={() => toggleFile(id)} />
                    )
                  })
                )}
                {displayFiles.length > 300 && (
                  <div className="col-span-5 text-center text-xs text-gray-400 pt-2">
                    Showing 300 of {displayFiles.length.toLocaleString()} files
                  </div>
                )}
              </div>
            )}

            {/* List view */}
            {result && viewMode === 'list' && (
              <table className="w-full text-xs">
                <thead className="sticky top-0 bg-white/80 backdrop-blur-sm z-10">
                  <tr className="text-left text-gray-400 font-semibold uppercase tracking-wider border-b border-gray-100">
                    <th className="px-3 py-2 w-6">
                      <div onClick={toggleAll} className={`w-3.5 h-3.5 rounded border cursor-pointer flex items-center justify-center ${
                        selectedIds.size > 0 ? 'bg-blue-500 border-blue-500' : 'border-gray-300'
                      }`}>
                        {selectedIds.size > 0 && (
                          <svg viewBox="0 0 12 12" fill="none" className="w-2 h-2">
                            <path d="M2 6l3 3 5-5" stroke="white" strokeWidth="1.5" strokeLinecap="round" />
                          </svg>
                        )}
                      </div>
                    </th>
                    <th className="px-3 py-2">Name</th>
                    <th className="px-3 py-2">Path</th>
                    <th className="px-3 py-2">Size</th>
                    <th className="px-3 py-2">Type</th>
                    <th className="px-3 py-2">Recovery</th>
                  </tr>
                </thead>
                <tbody>
                  {displayFiles.slice(0, 500).map((file, idx) => {
                    const id = file.id || String(idx)
                    const catCfg = CATEGORIES.find((c) => c.name === getCategory(file))!
                    return (
                      <tr key={id} onClick={() => toggleFile(id)}
                        className={`border-b border-gray-50 hover:bg-white/60 cursor-pointer transition-colors ${
                          selectedIds.has(id) ? 'bg-blue-50/60' : ''
                        }`}
                      >
                        <td className="px-3 py-2">
                          <div className={`w-3.5 h-3.5 rounded border flex items-center justify-center ${
                            selectedIds.has(id) ? 'bg-blue-500 border-blue-500' : 'border-gray-300'
                          }`}>
                            {selectedIds.has(id) && (
                              <svg viewBox="0 0 12 12" fill="none" className="w-2 h-2">
                                <path d="M2 6l3 3 5-5" stroke="white" strokeWidth="1.5" strokeLinecap="round" />
                              </svg>
                            )}
                          </div>
                        </td>
                        <td className="px-3 py-2">
                          <div className="flex items-center gap-1.5">
                            <span className="text-gray-400">{catCfg.icon}</span>
                            <span className="text-gray-700 truncate max-w-[180px]">{file.name}</span>
                            {file.is_deleted && <span className="text-[9px] bg-red-100 text-red-400 px-1 py-0.5 rounded">del</span>}
                          </div>
                        </td>
                        <td className="px-3 py-2 text-gray-400 truncate max-w-[180px]">{file.path}</td>
                        <td className="px-3 py-2 text-gray-500 whitespace-nowrap">{formatBytes(file.size)}</td>
                        <td className="px-3 py-2">
                          <span className="px-1.5 py-0.5 bg-gray-100 text-gray-500 rounded text-[10px]">{file.extension || 'â€”'}</span>
                        </td>
                        <td className="px-3 py-2">
                          <span className={`font-semibold ${
                            file.recovery_chance >= 80 ? 'text-green-500' : file.recovery_chance >= 50 ? 'text-yellow-500' : 'text-red-400'
                          }`}>{file.recovery_chance}%</span>
                        </td>
                      </tr>
                    )
                  })}
                </tbody>
              </table>
            )}

            {/* Detail view */}
            {result && viewMode === 'detail' && (
              <div className="flex flex-col gap-0.5">
                {displayFiles.slice(0, 500).map((file, idx) => {
                  const id = file.id || String(idx)
                  const cat = getCategory(file)
                  const catCfg = CATEGORIES.find((c) => c.name === cat)!
                  return (
                    <div key={id} onClick={() => toggleFile(id)}
                      className={`flex items-center gap-3 px-3 py-2 rounded-xl cursor-pointer transition-all ${
                        selectedIds.has(id) ? 'bg-blue-50/80' : 'hover:bg-white/60'
                      }`}
                    >
                      <div className={`w-7 h-7 rounded-lg flex items-center justify-center shrink-0 ${THUMB[cat]}`}>
                        <span className="scale-75">{catCfg.icon}</span>
                      </div>
                      <div className="flex-1 min-w-0">
                        <p className="text-xs font-medium text-gray-700 truncate">{file.name}</p>
                        <p className="text-[10px] text-gray-400 truncate">{file.path}</p>
                      </div>
                      <span className="text-[10px] text-gray-400 whitespace-nowrap">{formatBytes(file.size)}</span>
                      <span className={`text-[10px] font-semibold whitespace-nowrap ${
                        file.recovery_chance >= 80 ? 'text-green-500' : 'text-yellow-500'
                      }`}>{file.recovery_chance}%</span>
                    </div>
                  )
                })}
              </div>
            )}
          </div>
        </div>
      </div>

      {/* â”€â”€ Bottom Bar â”€â”€ */}
      <div className="flex items-center gap-4 px-5 py-2.5 mx-5 mb-3 rounded-2xl bg-white/60 backdrop-blur-md shadow-sm shrink-0">
        <CircularProgress pct={pct} />

        <div className="flex-1 min-w-0">
          <p className="text-xs font-semibold text-gray-700">
            {isScanning ? 'Quick Scanning' : progress.status === 'complete' ? 'Scan Complete' : progress.status === 'error' ? 'Scan Error' : 'Scan Cancelled'}
            {', Files Found: '}
            <span className="text-blue-600">
              {((isScanning ? progress.filesFound : result?.total_files) ?? 0).toLocaleString()}
            </span>
            {result && (
              <span className="text-gray-400 font-normal ml-1">
                ({formatBytes(totalSize)})
              </span>
            )}
          </p>
          <p className="text-[10px] text-gray-400 truncate">
            {isScanning ? progress.message : result ? `${result.scan_mode || 'quick'} scan â€¢ ${(result.mft_records_scanned || 0).toLocaleString()} MFT records` : error || 'Ready'}
          </p>
        </div>

        {/* Thin progress bar */}
        <div className="w-32 h-1.5 rounded-full bg-gray-200/80 overflow-hidden">
          <div
            className="h-full rounded-full bg-gradient-to-r from-blue-400 to-purple-500 transition-all duration-500"
            style={{ width: `${pct}%` }}
          />
        </div>

        {/* Controls */}
        {isScanning && (
          <div className="flex items-center gap-1.5">
            <button
              onClick={() => { isPaused.current = !isPaused.current }}
              title="Pause"
              className="p-1.5 rounded-lg bg-gray-100/80 hover:bg-gray-200/80 text-gray-500 hover:text-gray-700 transition-all"
            >
              <Pause size={14} />
            </button>
            <button
              onClick={handleCancelBack}
              title="Stop"
              className="p-1.5 rounded-lg bg-gray-100/80 hover:bg-red-100 text-gray-500 hover:text-red-500 transition-all"
            >
              <Square size={14} />
            </button>
          </div>
        )}
        {!isScanning && (
          <button
            onClick={handleCancelBack}
            className="px-3 py-1.5 rounded-lg bg-gray-100/80 hover:bg-gray-200/80 text-gray-500 text-xs font-medium transition-all"
          >
            Back
          </button>
        )}

        {/* Recover button */}
        <button
          className={`px-5 py-2 rounded-xl text-sm font-semibold transition-all shadow-sm ${
            selectedIds.size > 0
              ? 'bg-gradient-to-r from-blue-500 to-purple-500 text-white hover:shadow-md hover:scale-[1.02]'
              : result && !isScanning
              ? 'bg-gradient-to-r from-blue-400 to-purple-400 text-white hover:from-blue-500 hover:to-purple-500'
              : 'bg-gray-200 text-gray-400 cursor-not-allowed'
          }`}
          disabled={!result && isScanning}
        >
          {selectedIds.size > 0 ? `Recover (${selectedIds.size})` : 'Recover'}
        </button>
      </div>
    </div>
  )
}

export default ScanView
