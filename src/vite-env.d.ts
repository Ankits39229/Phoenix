/// <reference types="vite/client" />

declare global {
  interface DriveInfo {
    name: string
    letter: string
    label: string
    totalSpace: number
    freeSpace: number
    usedSpace: number
    usedPercentage: number
    isBitlocker: boolean
    isLocked: boolean
    filesystem: string
  }

  interface ScanProgress {
    status: 'scanning' | 'complete' | 'cancelled' | 'error'
    message: string
    progress?: number
    recordsScanned?: number
    filesFound?: number
  }

  interface RecoverableFile {
    id: string
    name: string
    path: string
    size: number
    extension: string
    category: string
    file_type: string
    modified: string
    created: string
    is_deleted: boolean
    recovery_chance: number
    source: string
  }

  interface ScanResult {
    success: boolean
    message: string
    scan_mode: string
    drive: string
    total_files: number
    total_recoverable_size: number
    scan_duration_ms: number
    mft_records_scanned: number
    mft_entries: RecoverableFile[]
    carved_files: RecoverableFile[]
    orphan_files: RecoverableFile[]
    requires_admin: boolean
  }

  interface Window {
    electron: {
      platform: string
      minimizeWindow: () => void
      maximizeWindow: () => void
      closeWindow: () => void
      getDrives: () => Promise<DriveInfo[]>
      scanDrive: (driveLetter: string, mode?: string) => Promise<ScanResult>
      cancelScan: () => void
      checkAdmin: () => Promise<{ is_admin: boolean; message: string }>
      onScanProgress: (callback: (data: ScanProgress) => void) => () => void
      selectFolder: () => Promise<string | null>
      getSpecialFolders: () => Promise<{ desktop: string; downloads: string }>
    }
  }
}

export {}
