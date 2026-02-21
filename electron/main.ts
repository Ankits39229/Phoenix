import { app, BrowserWindow, ipcMain } from 'electron';
import * as path from 'path';
import * as os from 'os';
import { execSync } from 'child_process';

let mainWindow: BrowserWindow | null = null;

// Check if we're in development mode
const isDev = process.env.NODE_ENV === 'development' || !app.isPackaged;

function createWindow() {
  mainWindow = new BrowserWindow({
    width: 1200,
    height: 800,
    minWidth: 900,
    minHeight: 600,
    frame: false,
    backgroundColor: '#e0e7ff',
    webPreferences: {
      nodeIntegration: false,
      contextIsolation: true,
      preload: path.join(__dirname, 'preload.js'),
    },
  });

  // Load the app
  if (isDev) {
    mainWindow.loadURL('http://localhost:3000');
    mainWindow.webContents.openDevTools();
  } else {
    mainWindow.loadFile(path.join(__dirname, '../react/index.html'));
  }

  mainWindow.on('closed', () => {
    mainWindow = null;
  });
}

// Window control handlers
ipcMain.on('window-minimize', () => {
  if (mainWindow) {
    mainWindow.minimize();
  }
});

ipcMain.on('window-maximize', () => {
  if (mainWindow) {
    if (mainWindow.isMaximized()) {
      mainWindow.unmaximize();
    } else {
      mainWindow.maximize();
    }
  }
});

ipcMain.on('window-close', () => {
  if (mainWindow) {
    mainWindow.close();
  }
});

// Get system drives
ipcMain.handle('get-drives', async () => {
  try {
    if (os.platform() === 'win32') {
      // For Windows, use PowerShell to get drive info
      const result = execSync(
        'powershell.exe -Command "Get-PSDrive -PSProvider FileSystem | Select-Object Name, Used, Free, @{Name=\'Total\';Expression={$_.Used+$_.Free}} | ConvertTo-Json"',
        { encoding: 'utf-8' }
      );
      
      const drives = JSON.parse(result);
      const driveArray = Array.isArray(drives) ? drives : [drives];
      
      return driveArray.map((drive: any) => ({
        name: drive.Name,
        label: `${drive.Name}: Drive`,
        used: drive.Used || 0,
        free: drive.Free || 0,
        total: drive.Total || 0,
        usedPercentage: drive.Total ? Math.round((drive.Used / drive.Total) * 100) : 0,
      }));
    } else {
      // For other platforms, return a simple list
      return [
        {
          name: 'Home',
          label: 'Home Directory',
          used: 0,
          free: 0,
          total: 0,
          usedPercentage: 0,
        },
      ];
    }
  } catch (error) {
    console.error('Error getting drives:', error);
    return [];
  }
});

app.whenReady().then(createWindow);

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') {
    app.quit();
  }
});

app.on('activate', () => {
  if (BrowserWindow.getAllWindows().length === 0) {
    createWindow();
  }
});
