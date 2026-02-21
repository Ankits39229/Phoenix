import { app, BrowserWindow, ipcMain, dialog } from 'electron';
import * as path from 'path';
import * as os from 'os';
import { execSync, spawn, ChildProcess } from 'child_process';

let mainWindow: BrowserWindow | null = null;
let scanProcess: ChildProcess | null = null;

// Check if we're in development mode
const isDev = process.env.NODE_ENV === 'development' || !app.isPackaged;

// Path to Rust backend binary
function getRustBackendPath(): string {
  if (isDev) {
    // __dirname = dist/electron, so go up two levels to project root
    const projectRoot = path.join(__dirname, '..', '..');
    const binaryPath = path.join(projectRoot, 'rust-backend', 'target', 'release', 'data_recovery_backend.exe');
    console.log('[RustBackend] path:', binaryPath);
    return binaryPath;
  }
  return path.join(process.resourcesPath, 'data_recovery_backend.exe');
}

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

// Get system drives using Rust backend
ipcMain.handle('get-drives', async () => {
  try {
    const backendPath = getRustBackendPath();
    const result = execSync(`"${backendPath}" drives`, { encoding: 'utf-8' });
    const drives = JSON.parse(result);
    return drives.map((drive: any) => ({
      name: drive.letter.replace(':', ''),
      letter: drive.letter,
      label: drive.label || 'Local Disk',
      totalSpace: drive.total_space || 0,
      freeSpace: drive.free_space || 0,
      usedSpace: (drive.total_space || 0) - (drive.free_space || 0),
      usedPercentage: drive.total_space ? Math.round(((drive.total_space - drive.free_space) / drive.total_space) * 100) : 0,
      isBitlocker: drive.is_bitlocker || false,
      isLocked: drive.is_locked || false,
      filesystem: drive.filesystem || 'Unknown',
    }));
  } catch (error: any) {
    console.error('Rust backend failed, falling back to PowerShell:', error.message);
    try {
      const result = execSync(
        'powershell.exe -Command "Get-PSDrive -PSProvider FileSystem | Select-Object Name, Used, Free, @{Name=\'Total\';Expression={$_.Used+$_.Free}} | ConvertTo-Json"',
        { encoding: 'utf-8' }
      );
      const drives = JSON.parse(result);
      const driveArray = Array.isArray(drives) ? drives : [drives];
      return driveArray.map((drive: any) => ({
        name: drive.Name,
        letter: `${drive.Name}:`,
        label: 'Local Disk',
        totalSpace: drive.Total || 0,
        freeSpace: drive.Free || 0,
        usedSpace: drive.Used || 0,
        usedPercentage: drive.Total ? Math.round((drive.Used / drive.Total) * 100) : 0,
        isBitlocker: false,
        isLocked: false,
        filesystem: 'Unknown',
      }));
    } catch (fallbackError) {
      console.error('Fallback also failed:', fallbackError);
      return [];
    }
  }
});

// Scan drive using Rust backend
ipcMain.handle('scan-drive', async (_event, driveLetter: string, mode: string = 'quick') => {
  try {
    const backendPath = getRustBackendPath();
    const driveArg = driveLetter.replace(':', '');
    
    return new Promise((resolve, reject) => {
      // Send initial scanning status
      if (mainWindow) {
        mainWindow.webContents.send('scan-progress', {
          status: 'scanning',
          message: `Starting ${mode} scan on ${driveLetter}...`,
          progress: 0,
        });
      }

      scanProcess = spawn(backendPath, ['deep-scan', driveArg, mode]);
      
      let stdout = '';
      let stderr = '';

      scanProcess.stdout?.on('data', (data: Buffer) => {
        stdout += data.toString();
      });

      scanProcess.stderr?.on('data', (data: Buffer) => {
        const msg = data.toString();
        stderr += msg;
        
        // Parse progress from stderr debug messages
        if (mainWindow) {
          const progressMatch = msg.match(/(\d+)%/);
          const recordMatch = msg.match(/(\d+)\s*records?\s*scanned/i);
          const filesMatch = msg.match(/Found\s*(\d+)\s*files?/i);
          
          mainWindow.webContents.send('scan-progress', {
            status: 'scanning',
            message: msg.trim().split('\n').pop() || 'Scanning...',
            progress: progressMatch ? parseInt(progressMatch[1]) : undefined,
            recordsScanned: recordMatch ? parseInt(recordMatch[1]) : undefined,
            filesFound: filesMatch ? parseInt(filesMatch[1]) : undefined,
          });
        }
      });

      scanProcess.on('close', (code: number | null) => {
        scanProcess = null;
        if (code === 0 && stdout.trim()) {
          try {
            const result = JSON.parse(stdout.trim());
            if (mainWindow) {
              mainWindow.webContents.send('scan-progress', {
                status: 'complete',
                message: 'Scan complete!',
                progress: 100,
              });
            }
            resolve(result);
          } catch (parseError) {
            reject(new Error('Failed to parse scan results'));
          }
        } else {
          // Even if exit code is non-zero, try to parse stdout (Rust backend sometimes exits 1 with valid results)
          if (stdout.trim()) {
            try {
              const result = JSON.parse(stdout.trim());
              resolve(result);
            } catch (_e) {
              reject(new Error(stderr || 'Scan failed'));
            }
          } else {
            reject(new Error(stderr || 'Scan failed'));
          }
        }
      });

      scanProcess.on('error', (err: Error) => {
        scanProcess = null;
        reject(new Error(`Failed to start scanner: ${err.message}`));
      });
    });
  } catch (error: any) {
    return { success: false, message: error.message };
  }
});

// Cancel ongoing scan
ipcMain.on('cancel-scan', () => {
  if (scanProcess) {
    scanProcess.kill();
    scanProcess = null;
    if (mainWindow) {
      mainWindow.webContents.send('scan-progress', {
        status: 'cancelled',
        message: 'Scan cancelled',
        progress: 0,
      });
    }
  }
});

// Check admin status
ipcMain.handle('select-folder', async () => {
  if (!mainWindow) return null;
  const result = await dialog.showOpenDialog(mainWindow, {
    properties: ['openDirectory'],
    title: 'Select a folder to scan',
  });
  if (result.canceled || result.filePaths.length === 0) return null;
  return result.filePaths[0];
});

ipcMain.handle('get-special-folders', async () => {
  const home = os.homedir();
  return {
    desktop: path.join(home, 'Desktop'),
    downloads: path.join(home, 'Downloads'),
  };
});

ipcMain.handle('check-admin', async () => {
  try {
    const backendPath = getRustBackendPath();
    const result = execSync(`"${backendPath}" check-admin`, { encoding: 'utf-8' });
    return JSON.parse(result);
  } catch (error) {
    return { is_admin: false, message: 'Failed to check admin status' };
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
