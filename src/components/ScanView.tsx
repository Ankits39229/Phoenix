import { useEffect, useState, useRef, useMemo, useCallback, memo } from 'react'
import {
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
  Square,
  CheckCircle,
  XCircle,
  FolderInput,
  X,
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
  scanMode?: 'quick' | 'deep'
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

// Large icons for card thumbnails (component refs so size can be set per context)
const CAT_ICON_LG: Record<FileCategory, React.ReactNode> = {
  Photo:    <Image size={32} />,
  Video:    <Video size={32} />,
  Audio:    <Music size={32} />,
  Document: <FileText size={32} />,
  Email:    <Mail size={32} />,
  Database: <Database size={32} />,
  Webfiles: <Globe size={32} />,
  Archive:  <Archive size={32} />,
  Others:   <File size={32} />,
  Unsaved:  <FileCode size={32} />,
  Game:     <Gamepad2 size={32} />,
}

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
// Image extensions that can be previewed directly from disk
const IMG_PREVIEW_EXTS = new Set(['jpg','jpeg','png','gif','bmp','webp','tiff','tif','avif','ico'])

const toFileUrl = (p: string): string => {
  const normalized = p.replace(/\\/g, '/')
  // Use localfile:// custom protocol registered in main.ts — avoids cross-origin
  // blocks when the app is loaded from localhost:3000 in dev mode.
  return normalized.match(/^[A-Za-z]:/) ? `localfile:///${normalized}` : `localfile://${normalized}`
}

const FileCard = memo(function FileCard({
  file, checked, onToggle,
}: { file: RecoverableFile; checked: boolean; onToggle: () => void }) {
  const cat = useMemo(() => getCategory(file), [file.extension])

  const ext = (file.extension || '').toLowerCase().replace(/^\./, '')
  const canPreview = !file.is_deleted && IMG_PREVIEW_EXTS.has(ext) && !!file.path
  const [imgFailed, setImgFailed] = useState(false)

  return (
    <div
      className={`group relative rounded-xl overflow-hidden bg-white/60 backdrop-blur-sm shadow-sm hover:shadow-md transition-all flex flex-col cursor-pointer border-2 ${
        checked ? 'border-blue-400' : 'border-transparent hover:border-blue-200'
      }`}
      onClick={onToggle}
    >
      {/* Thumbnail */}
      {canPreview && !imgFailed ? (
        <div className="h-20 bg-gray-100 overflow-hidden">
          <img
            src={toFileUrl(file.path)}
            alt={file.name}
            className="w-full h-full object-cover"
            onError={() => setImgFailed(true)}
            loading="lazy"
            draggable={false}
          />
        </div>
      ) : (
        <div className={`h-20 flex items-center justify-center ${THUMB[cat]}`}>
          {CAT_ICON_LG[cat]}
        </div>
      )}

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
})

const ScanView = ({ drive, scanMode = 'quick', onBack }: ScanViewProps) => {
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
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set())
  const [relunchingAdmin, setRelunchingAdmin] = useState(false)
  const [page, setPage] = useState(1)
  const PAGE_SIZE = 60

  // Filter panel
  const [showFilterPanel, setShowFilterPanel] = useState(false)
  const [filterDeletedOnly, setFilterDeletedOnly] = useState(false)
  const [filterMinRecovery, setFilterMinRecovery] = useState(0)

  // File Location sidebar
  const [folderTree, setFolderTree] = useState<{ path: string; name: string; count: number }[]>([])
  const [sidebarFolderPath, setSidebarFolderPath] = useState<string | null>(null)

  // Debounced search — only re-filter after user stops typing for 200ms
  const [searchQuery, setSearchQuery] = useState('')
  const [debouncedSearch, setDebouncedSearch] = useState('')
  const searchTimer = useRef<ReturnType<typeof setTimeout> | null>(null)
  const handleSearchChange = useCallback((val: string) => {
    setSearchQuery(val)
    if (searchTimer.current) clearTimeout(searchTimer.current)
    searchTimer.current = setTimeout(() => setDebouncedSearch(val), 200)
  }, [])

  // ── Recovery state ──────────────────────────────────────────────────────────
  interface RecoverState {
    phase: 'recovering' | 'done'
    current: number
    total: number
    fileName: string
    percent: number
    destFolder: string
    recovered?: number
    failed?: number
    results?: Array<{ name: string; success: boolean; message?: string; bytes_recovered?: number }>
  }
  const [recoverState, setRecoverState] = useState<RecoverState | null>(null)

  const handleRecover = async () => {
    if (selectedIds.size > 0) {
      // Recover only selected files — safe, renderer already holds these objects
      const filesToRecover = Array.from(selectedFiles.values())
      if (filesToRecover.length === 0) return
      const destFolder = await window.electron.selectFolder()
      if (!destFolder) return
      setRecoverState({ phase: 'recovering', current: 0, total: filesToRecover.length, fileName: '', percent: 0, destFolder })
      const cleanup = window.electron.onRecoverProgress((data) => {
        setRecoverState((prev) => prev ? { ...prev, current: data.current, fileName: data.fileName, percent: data.percent } : null)
      })
      try {
        const res = await window.electron.recoverFiles(drive.letter, filesToRecover, destFolder)
        cleanup()
        setRecoverState((prev) => prev ? { ...prev, phase: 'done', recovered: res.recovered, failed: res.failed, results: res.results, percent: 100 } : null)
      } catch { cleanup(); setRecoverState(null) }
    } else {
      // Recover All — pass filter params to main process; no file array loaded into renderer
      if (pageResult.total === 0) return
      const destFolder = await window.electron.selectFolder()
      if (!destFolder) return
      setRecoverState({ phase: 'recovering', current: 0, total: pageResult.total, fileName: '', percent: 0, destFolder })
      const cleanup = window.electron.onRecoverProgress((data) => {
        setRecoverState((prev) => prev ? { ...prev, current: data.current, fileName: data.fileName, percent: data.percent } : null)
      })
      try {
        const res = await window.electron.recoverFilesFiltered({
          driveLetter: drive.letter, category: selectedCategory, search: debouncedSearch,
          deletedOnly: filterDeletedOnly, minRecovery: filterMinRecovery,
          folderPath: sidebarFolderPath, destFolder,
        })
        cleanup()
        setRecoverState((prev) => prev ? { ...prev, phase: 'done', recovered: res.recovered, failed: res.failed, results: res.results, percent: 100 } : null)
      } catch { cleanup(); setRecoverState(null) }
    }
  }

  const scanStarted = useRef(false)

  // â”€â”€ Start scan on mount â”€â”€
  useEffect(() => {
    if (scanStarted.current) return
    scanStarted.current = true

    const cleanup = window.electron.onScanProgress((data: ScanProgress) => {
      setProgress(data)
    })

    window.electron
      .scanDrive(drive.letter, scanMode)
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

  // ── Page result from main process (only current page lives in renderer) ─────
  const [pageResult, setPageResult] = useState<FilesPageResult>({ files: [], total: 0, counts: {}, startIndex: 0 })

  // ── Accumulated list — appends on Load More, resets on any filter change ─────
  const [displayedFiles, setDisplayedFiles] = useState<RecoverableFile[]>([])

  // ── Track actual file objects for recovery across page changes ───────────────
  const [selectedFiles, setSelectedFiles] = useState<Map<string, RecoverableFile>>(new Map())

  // Reset page when any filter changes
  useEffect(() => { setPage(1) }, [selectedCategory, debouncedSearch, filterDeletedOnly, filterMinRecovery, sidebarFolderPath])

  // Fetch folder tree once scan finishes
  useEffect(() => {
    if (!result?.success) return
    window.electron.getFolderTree(drive.letter).then(setFolderTree)
  }, [result?.success, drive.letter])

  // Fetch a page whenever filters or page number changes
  useEffect(() => {
    if (!result?.success) return
    window.electron.getFilesPage({
      driveLetter: drive.letter,
      category: selectedCategory,
      search: debouncedSearch,
      page,
      pageSize: PAGE_SIZE,
      deletedOnly: filterDeletedOnly,
      minRecovery: filterMinRecovery,
      folderPath: sidebarFolderPath,
    }).then((newResult) => {
      setPageResult(newResult)
      // page===1 means a fresh load (filter changed or initial) → replace the list.
      // page>1 means the user clicked "Load more" → append the new batch.
      setDisplayedFiles((prev) => page === 1 ? newResult.files : [...prev, ...newResult.files])
    })
  }, [result?.success, drive.letter, selectedCategory, debouncedSearch, page, PAGE_SIZE, filterDeletedOnly, filterMinRecovery, sidebarFolderPath])

  const pagedFiles = displayedFiles
  const categoryCounts = pageResult.counts as Record<FileCategory, number>
  const totalSize = result?.total_recoverable_size || 0

  const isScanning = progress.status === 'scanning'
  const pct = progress.progress ?? 0

  const handleCancelBack = () => {
    if (isScanning) window.electron.cancelScan()
    onBack()
  }

  const toggleFile = useCallback((id: string, file: RecoverableFile) => {
    setSelectedIds((prev) => {
      const next = new Set(prev)
      next.has(id) ? next.delete(id) : next.add(id)
      return next
    })
    setSelectedFiles((prev) => {
      const next = new Map(prev)
      next.has(id) ? next.delete(id) : next.set(id, file)
      return next
    })
  }, [])

  const toggleAll = useCallback(() => {
    const allPageIds = pagedFiles.map((f, i) => f.id || String(pageResult.startIndex + i))
    const allOnPage = allPageIds.every((id) => selectedIds.has(id))
    setSelectedIds((prev) => {
      const next = new Set(prev)
      if (allOnPage) allPageIds.forEach((id) => next.delete(id))
      else allPageIds.forEach((id) => next.add(id))
      return next
    })
    setSelectedFiles((prev) => {
      const next = new Map(prev)
      if (allOnPage) {
        allPageIds.forEach((id) => next.delete(id))
      } else {
        pagedFiles.forEach((f, i) => {
          const id = f.id || String(pageResult.startIndex + i)
          next.set(id, f)
        })
      }
      return next
    })
  }, [pagedFiles, pageResult.startIndex, selectedIds])

  // ────────────────────────────────────────────────────────────────────────────
  return (
    <div className="relative flex-1 flex flex-col overflow-hidden">
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

        <div className="relative">
          <button
            onClick={() => setShowFilterPanel((p) => !p)}
            className={`flex items-center gap-1.5 text-xs rounded-lg px-2.5 py-1.5 transition-all ${
              filterDeletedOnly || filterMinRecovery > 0
                ? 'bg-blue-100 text-blue-600 hover:bg-blue-200'
                : 'text-gray-500 hover:text-gray-700 bg-gray-100/60 hover:bg-gray-100'
            }`}
          >
            <SlidersHorizontal size={12} />
            Filter
            {(filterDeletedOnly || filterMinRecovery > 0) && (
              <span className="w-1.5 h-1.5 rounded-full bg-blue-500" />
            )}
          </button>
          {showFilterPanel && (
            <div className="absolute right-0 top-full mt-1 z-30 bg-white rounded-xl shadow-lg border border-gray-100 p-3 w-56 flex flex-col gap-3">
              <div className="flex items-center justify-between">
                <p className="text-[11px] font-semibold text-gray-500 uppercase tracking-wide">Filters</p>
                <button onClick={() => setShowFilterPanel(false)} className="text-gray-400 hover:text-gray-600"><X size={12} /></button>
              </div>
              <label className="flex items-center gap-2 cursor-pointer select-none">
                <input type="checkbox" checked={filterDeletedOnly}
                  onChange={(e) => { setFilterDeletedOnly(e.target.checked); setPage(1) }}
                  className="w-3 h-3 accent-blue-500"
                />
                <span className="text-xs text-gray-600">Deleted files only</span>
              </label>
              <div className="flex flex-col gap-1.5">
                <p className="text-[11px] text-gray-500">Min. recovery chance</p>
                <div className="flex gap-1">
                  {[0, 50, 80].map((v) => (
                    <button key={v} onClick={() => { setFilterMinRecovery(v); setPage(1) }}
                      className={`flex-1 py-1 rounded-lg text-[11px] font-medium transition-all ${
                        filterMinRecovery === v ? 'bg-blue-500 text-white' : 'bg-gray-100 text-gray-500 hover:bg-gray-200'
                      }`}
                    >{v === 0 ? 'Any' : `≥${v}%`}</button>
                  ))}
                </div>
              </div>
              {(filterDeletedOnly || filterMinRecovery > 0) && (
                <button onClick={() => { setFilterDeletedOnly(false); setFilterMinRecovery(0); setPage(1) }}
                  className="text-[11px] text-red-400 hover:text-red-600 text-left">Clear filters</button>
              )}
            </div>
          )}
        </div>

        <div className="relative">
          <Search size={12} className="absolute left-2.5 top-1/2 -translate-y-1/2 text-gray-400" />
          <input
            type="text"
            placeholder="Search File"
            value={searchQuery}
            onChange={(e) => handleSearchChange(e.target.value)}
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
                onClick={() => {
                  setSidebarTab(tab)
                  // Clear the opposing filter so the two tabs act as independent modes
                  if (tab === 'type') setSidebarFolderPath(null)
                  if (tab === 'location') setSelectedCategory(null)
                }}
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
              folderTree.length === 0 ? (
                <div className="flex flex-col items-center justify-center h-full py-12 gap-3">
                  <FolderOpen size={28} className="text-gray-300" />
                  <p className="text-xs text-gray-400 text-center px-4">
                    {isScanning ? 'Building folder tree...' : result?.success ? 'No folders found' : 'Run a scan to see folders'}
                  </p>
                </div>
              ) : (
                <>
                  {sidebarFolderPath && (
                    <button
                      onClick={() => { setSidebarFolderPath(null); setPage(1) }}
                      className="w-full flex items-center gap-1.5 px-4 py-2 text-[11px] text-blue-500 hover:text-blue-700 bg-blue-50/60 border-b border-blue-100"
                    >
                      <X size={10} /> Clear folder filter
                    </button>
                  )}
                  {folderTree.map((folder) => {
                    const active = sidebarFolderPath === folder.path
                    return (
                      <button
                        key={folder.path}
                        onClick={() => { setSidebarFolderPath(active ? null : folder.path); setPage(1) }}
                        title={folder.path}
                        className={`w-full flex items-center justify-between px-4 py-2.5 text-sm transition-all group ${
                          active ? 'bg-blue-50/80 text-blue-700' : 'hover:bg-white/60 text-gray-600'
                        }`}
                      >
                        <div className="flex items-center gap-2.5 min-w-0">
                          <FolderOpen size={14} className={`shrink-0 ${active ? 'text-blue-500' : 'text-gray-400 group-hover:text-gray-600'}`} />
                          <span className={`font-medium truncate ${active ? 'text-blue-700' : ''}`}>{folder.name}</span>
                        </div>
                        <span className={`text-xs tabular-nums shrink-0 ${active ? 'text-blue-500 font-semibold' : 'text-gray-400'}`}>
                          {folder.count.toLocaleString()}
                        </span>
                      </button>
                    )
                  })}
                </>
              )
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
            {result && result.success ? (
              <label className="flex items-center gap-2 cursor-pointer" onClick={toggleAll}>
                <div className={`w-4 h-4 rounded border-2 flex items-center justify-center transition-all ${
                  selectedIds.size > 0 && selectedIds.size === pageResult.total
                    ? 'bg-blue-500 border-blue-500'
                    : selectedIds.size > 0
                    ? 'bg-blue-200 border-blue-400'
                    : 'border-gray-300 hover:border-blue-300'
                }`}>
                  {selectedIds.size > 0 && (
                    <svg viewBox="0 0 12 12" fill="none" className="w-2.5 h-2.5">
                      <path
                        d={selectedIds.size === pageResult.total ? 'M2 6l3 3 5-5' : 'M2 6h8'}
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
                {isScanning ? 'Scanning...' : result && !result.success ? (result.requires_admin ? '⚠ Administrator required' : 'Scan failed') : error ? 'Scan failed' : 'No results'}
              </span>
            )}
            <span className="text-xs text-gray-400 tabular-nums">
              {pageResult.total.toLocaleString()} files{selectedCategory ? ` (${selectedCategory})` : ''}
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

            {/* Admin / failure state */}
            {result && !result.success && (
              <div className="flex flex-col items-center justify-center h-full gap-5">
                <div className="w-16 h-16 rounded-full bg-amber-100 flex items-center justify-center">
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" className="w-8 h-8 text-amber-500" strokeWidth="1.5">
                    <path strokeLinecap="round" strokeLinejoin="round" d="M12 9v3.75m-9.303 3.376c-.866 1.5.217 3.374 1.948 3.374h14.71c1.73 0 2.813-1.874 1.948-3.374L13.949 3.378c-.866-1.5-3.032-1.5-3.898 0L2.697 16.126zM12 15.75h.007v.008H12v-.008z" />
                  </svg>
                </div>
                <div className="text-center max-w-sm">
                  {result.requires_admin ? (
                    <>
                      <p className="text-sm font-bold text-gray-800">Administrator Privileges Required</p>
                      <p className="text-xs text-gray-500 mt-1.5 leading-relaxed">
                        Scanning for deleted files requires raw disk access, which needs Administrator rights.
                        Please restart the app as Administrator to proceed.
                      </p>
                      <button
                        disabled={relunchingAdmin}
                        onClick={async () => {
                          setRelunchingAdmin(true)
                          await window.electron.relunchAsAdmin()
                        }}
                        className="mt-4 px-5 py-2 rounded-xl bg-gradient-to-r from-blue-500 to-purple-500 text-white text-sm font-semibold shadow hover:shadow-md active:scale-95 transition-all disabled:opacity-60"
                      >
                        {relunchingAdmin ? 'Relaunching…' : '🛡 Restart as Administrator'}
                      </button>
                      <p className="text-[10px] text-gray-400 mt-2">
                        Or right-click the app shortcut → Run as administrator
                      </p>
                    </>
                  ) : (
                    <>
                      <p className="text-sm font-bold text-gray-700">Scan Failed</p>
                      <p className="text-xs text-gray-400 mt-1">{result.message}</p>
                    </>
                  )}
                </div>
              </div>
            )}

            {/* Grid view */}
            {result && result.success && viewMode === 'grid' && (
              <div className="grid grid-cols-6 gap-2">
                {pageResult.total === 0 ? (
                  <div className="col-span-5 flex flex-col items-center justify-center py-16 gap-3">
                    <FolderOpen size={32} className="text-gray-200" />
                    <p className="text-xs text-gray-400">No files in this category</p>
                  </div>
                ) : (
                  pagedFiles.map((file, idx) => {
                    const id = file.id || String(pageResult.startIndex + idx)
                    return (
                      <FileCard key={id} file={file} checked={selectedIds.has(id)} onToggle={() => toggleFile(id, file)} />
                    )
                  })
                )}
                {displayedFiles.length < pageResult.total && (
                  <div className="col-span-5 text-center pt-2">
                    <button
                      onClick={() => setPage((p) => p + 1)}
                      className="text-xs text-blue-500 hover:text-blue-700 font-medium"
                    >
                      Load more ({(pageResult.total - displayedFiles.length).toLocaleString()} remaining)
                    </button>
                  </div>
                )}
              </div>
            )}

            {/* List view */}
            {result && result.success && viewMode === 'list' && (
              <>
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
                  {pagedFiles.map((file, idx) => {
                    const id = file.id || String(pageResult.startIndex + idx)
                    const catCfg = CATEGORIES.find((c) => c.name === getCategory(file))!
                    return (
                      <tr key={id} onClick={() => toggleFile(id, file)}
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
              {displayedFiles.length < pageResult.total && (
                <div className="text-center py-3">
                  <button
                    onClick={() => setPage((p) => p + 1)}
                    className="text-xs text-blue-500 hover:text-blue-700 font-medium"
                  >
                    Load more ({(pageResult.total - displayedFiles.length).toLocaleString()} remaining)
                  </button>
                </div>
              )}
              </>
            )}

            {/* Detail view */}
            {result && result.success && viewMode === 'detail' && (
              <div className="flex flex-col gap-0.5">
                {pagedFiles.map((file, idx) => {
                  const id = file.id || String(pageResult.startIndex + idx)
                  const cat = getCategory(file)
                  const catCfg = CATEGORIES.find((c) => c.name === cat)!
                  return (
                    <div key={id} onClick={() => toggleFile(id, file)}
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
                {displayedFiles.length < pageResult.total && (
                  <div className="text-center py-3">
                    <button
                      onClick={() => setPage((p) => p + 1)}
                      className="text-xs text-blue-500 hover:text-blue-700 font-medium"
                    >
                      Load more ({(pageResult.total - displayedFiles.length).toLocaleString()} remaining)
                    </button>
                  </div>
                )}
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
            {isScanning
              ? (scanMode === 'deep' ? 'Deep Scanning' : 'Quick Scanning')
              : progress.status === 'complete' ? 'Scan Complete'
              : progress.status === 'error' ? 'Scan Error'
              : 'Scan Cancelled'}
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
            {isScanning
              ? progress.message
              : result
              ? `${result.scan_mode || scanMode} scan • ${(result.mft_records_scanned || 0).toLocaleString()} MFT records${result.scan_mode?.toLowerCase() === 'deep' ? ` • ${(result.sectors_scanned || 0).toLocaleString()} sectors carved` : ''}`
              : error || 'Ready'}
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
          <button
            onClick={handleCancelBack}
            title="Stop scan"
            className="p-1.5 rounded-lg bg-gray-100/80 hover:bg-red-100 text-gray-500 hover:text-red-500 transition-all"
          >
            <Square size={14} />
          </button>
        )}
        {!isScanning && (
          <button
            onClick={handleCancelBack}
            className="px-3 py-1.5 rounded-lg bg-gray-100/80 hover:bg-gray-200/80 text-gray-500 text-xs font-medium transition-all"
          >
            Back
          </button>
        )}

        {/* Recover button hint */}
        {result && result.success && !isScanning && selectedIds.size === 0 && pageResult.total > 0 && (
          <span className="flex items-center gap-1 text-xs font-medium text-blue-500 bg-blue-50 border border-blue-100 px-2.5 py-1 rounded-lg">
            No files selected — clicking Recover will restore all {pageResult.total.toLocaleString()}
            <ChevronRight size={12} className="shrink-0" />
          </span>
        )}
        <button
          onClick={handleRecover}
          disabled={isScanning || !result?.success || pageResult.total === 0}
          className={`px-5 py-2 rounded-xl text-sm font-semibold transition-all shadow-sm whitespace-nowrap ${
            selectedIds.size > 0
              ? 'bg-gradient-to-r from-blue-500 to-purple-500 text-white hover:shadow-md hover:scale-[1.02]'
              : result && result.success && !isScanning && pageResult.total > 0
              ? 'bg-gradient-to-r from-blue-400 to-purple-400 text-white hover:from-blue-500 hover:to-purple-500'
              : 'bg-gray-200 text-gray-400 cursor-not-allowed'
          }`}
        >
          {selectedIds.size > 0
            ? `Recover (${selectedIds.size})`
            : pageResult.total > 0
            ? `Recover All (${pageResult.total})`
            : 'Recover'}
        </button>
      </div>

      {/* ── Recovery Modal Overlay ── */}
      {recoverState && (
        <div className="absolute inset-0 z-50 flex items-center justify-center bg-white/60 backdrop-blur-md rounded-2xl">
          <div className="w-[420px] rounded-2xl bg-white shadow-2xl border border-gray-100 overflow-hidden">
            {/* Header */}
            <div className="px-6 py-4 bg-gradient-to-r from-blue-500 to-purple-500 text-white">
              <p className="font-bold text-base">
                {recoverState.phase === 'recovering' ? 'Recovering Files…' : 'Recovery Complete'}
              </p>
              <p className="text-xs text-white/70 mt-0.5 truncate">→ {recoverState.destFolder}</p>
            </div>

            <div className="px-6 py-5 flex flex-col gap-4">
              {recoverState.phase === 'recovering' ? (
                <>
                  {/* Progress bar */}
                  <div className="space-y-1.5">
                    <div className="flex justify-between text-xs text-gray-500">
                      <span>{recoverState.current} / {recoverState.total} files</span>
                      <span>{recoverState.percent}%</span>
                    </div>
                    <div className="h-2 rounded-full bg-gray-100 overflow-hidden">
                      <div
                        className="h-full rounded-full bg-gradient-to-r from-blue-400 to-purple-500 transition-all duration-300"
                        style={{ width: `${recoverState.percent}%` }}
                      />
                    </div>
                  </div>
                  {/* Current file */}
                  {recoverState.fileName && (
                    <div className="flex items-center gap-2 bg-blue-50 rounded-lg px-3 py-2">
                      <div className="w-1.5 h-1.5 rounded-full bg-blue-400 animate-pulse shrink-0" />
                      <p className="text-xs text-blue-700 truncate font-medium">{recoverState.fileName}</p>
                    </div>
                  )}
                  <p className="text-[11px] text-gray-400 text-center">Please wait, do not close the app…</p>
                </>
              ) : (
                <>
                  {/* Summary */}
                  <div className="flex gap-3">
                    <div className="flex-1 rounded-xl bg-green-50 border border-green-100 px-4 py-3 text-center">
                      <CheckCircle size={20} className="text-green-500 mx-auto mb-1" />
                      <p className="text-2xl font-bold text-green-600">{recoverState.recovered}</p>
                      <p className="text-[11px] text-green-500">Recovered</p>
                    </div>
                    <div className="flex-1 rounded-xl bg-red-50 border border-red-100 px-4 py-3 text-center">
                      <XCircle size={20} className="text-red-400 mx-auto mb-1" />
                      <p className="text-2xl font-bold text-red-400">{recoverState.failed}</p>
                      <p className="text-[11px] text-red-400">Failed</p>
                    </div>
                  </div>
                  {/* Failed files list */}
                  {(recoverState.results || []).filter((r) => !r.success).length > 0 && (
                    <div className="max-h-28 overflow-y-auto space-y-1">
                      {(recoverState.results || []).filter((r) => !r.success).map((r, i) => (
                        <div key={i} className="flex items-start gap-1.5 text-[11px] text-red-500 bg-red-50 rounded px-2 py-1">
                          <XCircle size={11} className="mt-0.5 shrink-0" />
                          <span className="truncate font-medium">{r.name}</span>
                          <span className="text-red-300 shrink-0">— {r.message}</span>
                        </div>
                      ))}
                    </div>
                  )}
                  {/* Open folder + close */}
                  <div className="flex gap-2 pt-1">
                    <button
                      onClick={() => setRecoverState(null)}
                      className="flex-1 py-2 rounded-xl border border-gray-200 text-gray-500 text-sm font-medium hover:bg-gray-50 transition-all"
                    >
                      Close
                    </button>
                    <button
                      onClick={() => {
                        window.electron.openFolder(recoverState.destFolder)
                        setRecoverState(null)
                      }}
                      className="flex-1 py-2 rounded-xl bg-gradient-to-r from-blue-500 to-purple-500 text-white text-sm font-semibold hover:shadow-md transition-all flex items-center justify-center gap-1.5"
                    >
                      <FolderInput size={14} />
                      Open Folder
                    </button>
                  </div>
                </>
              )}
            </div>
          </div>
        </div>
      )}
    </div>
  )
}

export default ScanView
