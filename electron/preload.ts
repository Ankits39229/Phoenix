import { contextBridge, ipcRenderer } from 'electron';

contextBridge.exposeInMainWorld('electron', {
  platform: process.platform,
  minimizeWindow: () => ipcRenderer.send('window-minimize'),
  maximizeWindow: () => ipcRenderer.send('window-maximize'),
  closeWindow: () => ipcRenderer.send('window-close'),
  getDrives: () => ipcRenderer.invoke('get-drives'),
  scanDrive: (driveLetter: string, mode?: string) => ipcRenderer.invoke('scan-drive', driveLetter, mode || 'quick'),
  cancelScan: () => ipcRenderer.send('cancel-scan'),
  checkAdmin: () => ipcRenderer.invoke('check-admin'),
  onScanProgress: (callback: (data: any) => void) => {
    ipcRenderer.on('scan-progress', (_event, data) => callback(data));
    return () => ipcRenderer.removeAllListeners('scan-progress');
  },
  selectFolder: () => ipcRenderer.invoke('select-folder'),
  getSpecialFolders: () => ipcRenderer.invoke('get-special-folders'),
  relunchAsAdmin: () => ipcRenderer.invoke('relaunch-as-admin'),
  openFolder: (folderPath: string) => ipcRenderer.invoke('open-folder', folderPath),
  recoverFiles: (driveLetter: string, files: any[], destFolder: string) =>
    ipcRenderer.invoke('recover-files', driveLetter, files, destFolder),
  getFilesPage: (opts: {
    driveLetter: string;
    category: string | null;
    search: string;
    page: number;
    pageSize: number;
  }) => ipcRenderer.invoke('get-files-page', opts),
  onRecoverProgress: (callback: (data: any) => void) => {
    ipcRenderer.on('recover-progress', (_event, data) => callback(data));
    return () => ipcRenderer.removeAllListeners('recover-progress');
  },
});
