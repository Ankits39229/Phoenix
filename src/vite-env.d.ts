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
    requires_admin: boolean
    // file arrays omitted — they stay in the main process
    mft_entries?: RecoverableFile[]
    carved_files?: RecoverableFile[]
    orphan_files?: RecoverableFile[]
  }

  interface FilesPageResult {
    files: RecoverableFile[]
    total: number
    counts: Record<string, number>
    startIndex: number
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
      relunchAsAdmin: () => Promise<{ success: boolean; message?: string }>
      openFolder: (folderPath: string) => Promise<void>
      getFilesPage: (opts: {
        driveLetter: string
        category: string | null
        search: string
        page: number
        pageSize: number
        deletedOnly?: boolean
        minRecovery?: number
        folderPath?: string | null
      }) => Promise<FilesPageResult>
      getFolderTree: (driveLetter: string) => Promise<{ path: string; name: string; count: number }[]>
      recoverFilesFiltered: (opts: {
        driveLetter: string
        category: string | null
        search: string
        deletedOnly?: boolean
        minRecovery?: number
        folderPath?: string | null
        destFolder: string
      }) => Promise<{
        recovered: number
        failed: number
        total: number
        results: Array<{ name: string; success: boolean; message?: string; bytes_recovered?: number }>
      }>
      recoverFiles: (
        driveLetter: string,
        files: RecoverableFile[],
        destFolder: string
      ) => Promise<{
        recovered: number
        failed: number
        total: number
        results: Array<{ name: string; success: boolean; message?: string; bytes_recovered?: number }>
      }>
      onRecoverProgress: (callback: (data: {
        current: number
        total: number
        fileName: string
        percent: number
      }) => void) => () => void
    }
  }
}

export {}
