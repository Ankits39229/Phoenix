interface DriveInfo {
  name: string
  label: string
  used: number
  free: number
  total: number
  usedPercentage: number
}

declare global {
  interface Window {
    electron: {
      platform: string
      minimizeWindow: () => void
      maximizeWindow: () => void
      closeWindow: () => void
      getDrives: () => Promise<DriveInfo[]>
    }
  }
}

export {}
