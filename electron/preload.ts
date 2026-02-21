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
});
