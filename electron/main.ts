import { app, BrowserWindow, ipcMain, dialog, shell, protocol, net } from 'electron';
import * as path from 'path';
import * as os from 'os';
import { execSync, spawn, ChildProcess } from 'child_process';

// Register before app is ready so it can be used as a privileged scheme
protocol.registerSchemesAsPrivileged([
  { scheme: 'localfile', privileges: { bypassCSP: true, corsEnabled: true, supportFetchAPI: true, stream: true } },
]);

let mainWindow: BrowserWindow | null = null;
let scanProcess: ChildProcess | null = null;

// ── Scan result cache – lives in main process so the renderer never holds the full array ──
let lastScanResult: any = null;

// Extension → category mapping (must match ScanView.tsx's getCategory)
const EXT_CAT: Record<string, string> = {
  jpg:'Photo',jpeg:'Photo',png:'Photo',gif:'Photo',bmp:'Photo',webp:'Photo',
  tiff:'Photo',tif:'Photo',svg:'Photo',heic:'Photo',raw:'Photo',cr2:'Photo',
  nef:'Photo',arw:'Photo',dng:'Photo',heif:'Photo',psd:'Photo',avif:'Photo',
  mp4:'Video',avi:'Video',mov:'Video',mkv:'Video',wmv:'Video',flv:'Video',
  webm:'Video',m4v:'Video','3gp':'Video',mpg:'Video',mpeg:'Video',ts:'Video',
  mp3:'Audio',wav:'Audio',flac:'Audio',aac:'Audio',ogg:'Audio',wma:'Audio',
  m4a:'Audio',opus:'Audio',aiff:'Audio',
  pdf:'Document',doc:'Document',docx:'Document',txt:'Document',xls:'Document',
  xlsx:'Document',ppt:'Document',pptx:'Document',odt:'Document',csv:'Document',
  rtf:'Document',md:'Document',pages:'Document',numbers:'Document',key:'Document',
  eml:'Email',msg:'Email',pst:'Email',ost:'Email',mbox:'Email',
  db:'Database',sqlite:'Database',sql:'Database',mdf:'Database',accdb:'Database',mdb:'Database',
  html:'Webfiles',htm:'Webfiles',css:'Webfiles',js:'Webfiles',php:'Webfiles',
  xml:'Webfiles',json:'Webfiles',jsx:'Webfiles',tsx:'Webfiles',
  zip:'Archive',rar:'Archive','7z':'Archive',tar:'Archive',gz:'Archive',
  bz2:'Archive',xz:'Archive',cab:'Archive',iso:'Archive',
};
function fileCat(f: any): string {
  const ext = (f.extension || '').toLowerCase().replace(/^\./, '');
  return EXT_CAT[ext] || 'Others';
}

// Return scan result without the heavy file arrays
function stripMeta(r: any) {
  return {
    success: r.success,
    message: r.message,
    scan_mode: r.scan_mode,
    drive: r.drive,
    total_files: r.total_files,
    total_recoverable_size: r.total_recoverable_size,
    scan_duration_ms: r.scan_duration_ms,
    mft_records_scanned: r.mft_records_scanned,
    sectors_scanned: r.sectors_scanned,
    requires_admin: r.requires_admin,
  };
}

// ── Shared filter helper ───────────────────────────────────────────────────────
interface FilterOpts {
  driveLetter: string;
  category: string | null;
  search: string;
  deletedOnly?: boolean;
  minRecovery?: number;
  folderPath?: string | null;
  importantFoldersOnly?: boolean;
}
function getFilteredFiles(opts: FilterOpts): any[] {
  if (!lastScanResult) return [];
  const { driveLetter, category, search, deletedOnly, minRecovery, folderPath, importantFoldersOnly } = opts;

  const activeFolderFilter = folderPath
    ? folderPath.replace(/\\/g, '/').toLowerCase()
    : driveLetter.length > 2
    ? driveLetter.replace(/\\/g, '/').toLowerCase()
    : null;

  const raw: any[] = [
    ...(lastScanResult.mft_entries || []),
    ...(lastScanResult.carved_files || []),
    ...(lastScanResult.orphan_files || []),
  ];

  let filtered = activeFolderFilter
    ? raw.filter((f) => {
        if (f.is_deleted) return true;
        const fp = (f.path || '').replace(/\\/g, '/').toLowerCase();
        if (fp.includes('$recycle.bin')) return true;
        return fp.startsWith(activeFolderFilter);
      })
    : raw;

  // Deduplicate by id — USN journal entries can share the same MFT record number
  // as MFT scan entries or appear multiple times for the same file.
  const seenIds = new Set<string>();
  filtered = filtered.filter((f) => {
    const id = f.id || '';
    if (!id || seenIds.has(id)) return false;
    seenIds.add(id);
    return true;
  });

  if (category) filtered = filtered.filter((f) => fileCat(f) === category);
  if (search.trim()) {
    const q = search.toLowerCase();
    filtered = filtered.filter((f) =>
      f.name?.toLowerCase().includes(q) ||
      f.path?.toLowerCase().includes(q) ||
      f.extension?.toLowerCase().includes(q)
    );
  }
  if (deletedOnly) filtered = filtered.filter((f) => f.is_deleted);
  if (minRecovery && minRecovery > 0) filtered = filtered.filter((f) => (f.recovery_chance || 0) >= minRecovery);

  // Important folders filter — C: drive only: Desktop, Downloads, Documents, Pictures, Videos, Music
  if (importantFoldersOnly) {
    const home = os.homedir().replace(/\\/g, '/').toLowerCase();
    const importantDirs = ['desktop', 'downloads', 'documents', 'pictures', 'videos', 'music', 'onedrive'];
    const importantPaths = importantDirs.map((d) => `${home}/${d}`);
    filtered = filtered.filter((f) => {
      const fp = (f.path || '').replace(/\\/g, '/').toLowerCase();
      if (fp.includes('$recycle.bin')) return true;
      return importantPaths.some((p) => fp.startsWith(p));
    });
  }

  return filtered;
}

// Paginated file fetch — renderer calls this instead of holding all file objects
ipcMain.handle('get-files-page', (_event, opts: {
  driveLetter: string;
  category: string | null;
  search: string;
  page: number;
  pageSize: number;
  deletedOnly?: boolean;
  minRecovery?: number;
  folderPath?: string | null;
  importantFoldersOnly?: boolean;
}) => {
  if (!lastScanResult) return { files: [], total: 0, counts: {}, startIndex: 0 };
  const { page, pageSize } = opts;

  // Category counts use folder filter + deletedOnly filter but no category/search filter
  const folderCounted = getFilteredFiles({ ...opts, category: null, search: '', minRecovery: 0 });
  const counts: Record<string, number> = {};
  for (const f of folderCounted) {
    const cat = fileCat(f);
    counts[cat] = (counts[cat] || 0) + 1;
  }

  const filtered = getFilteredFiles(opts);
  const total = filtered.length;
  const start = pageSize > 0 ? (page - 1) * pageSize : 0;
  const files = pageSize > 0 ? filtered.slice(start, start + pageSize) : filtered;

  return { files, total, counts, startIndex: start };
});

// Folder tree — returns unique parent folders sorted by file count
ipcMain.handle('get-folder-tree', (_event, _driveLetter: string) => {
  if (!lastScanResult) return [];
  const raw: any[] = [
    ...(lastScanResult.mft_entries || []),
    ...(lastScanResult.carved_files || []),
    ...(lastScanResult.orphan_files || []),
  ];
  const counts: Record<string, number> = {};
  for (const f of raw) {
    const p = (f.path || '').replace(/\\/g, '/');
    const idx = p.lastIndexOf('/');
    if (idx > 0) {
      const folder = p.substring(0, idx);
      counts[folder] = (counts[folder] || 0) + 1;
    }
  }
  return Object.entries(counts)
    .sort((a, b) => b[1] - a[1])
    .slice(0, 150)
    .map(([folderPath, count]) => {
      const parts = folderPath.split('/');
      return { path: folderPath, name: parts[parts.length - 1] || folderPath, count };
    });
});

// Recover all files matching filter — avoids sending large file arrays to renderer
ipcMain.handle('recover-files-filtered', async (_event, opts: {
  driveLetter: string;
  category: string | null;
  search: string;
  deletedOnly?: boolean;
  minRecovery?: number;
  folderPath?: string | null;
  importantFoldersOnly?: boolean;
  destFolder: string;
}) => {
  const { destFolder } = opts;
  const files = getFilteredFiles(opts);
  const backendPath = getRustBackendPath();
  const driveArg = opts.driveLetter.length > 2
    ? opts.driveLetter.charAt(0).toUpperCase()
    : opts.driveLetter.replace(':', '').toUpperCase();
  const { existsSync } = require('fs');
  const results: any[] = [];

  for (let i = 0; i < files.length; i++) {
    const file = files[i];
    if (mainWindow) {
      mainWindow.webContents.send('recover-progress', {
        current: i + 1,
        total: files.length,
        fileName: file.name || '',
        percent: Math.round((i / files.length) * 100),
      });
    }
    await new Promise<void>((resolve) => {
      const fileJson = JSON.stringify(file);
      const rawName: string = file.name || `recovered_file_${i + 1}`;
      const safeName = rawName.replace(/[<>:"/\\|?*\x00-\x1f]/g, '_');
      let destFilePath = path.join(destFolder, safeName);
      if (existsSync(destFilePath)) {
        const ext = path.extname(safeName);
        const base = path.basename(safeName, ext);
        destFilePath = path.join(destFolder, `${base}_recovered_${i + 1}${ext}`);
      }
      const proc = spawn(backendPath, ['recover-deleted', driveArg, fileJson, destFilePath]);
      let stdout = '';
      let stderr = '';
      proc.stdout?.on('data', (d: Buffer) => { stdout += d.toString(); });
      proc.stderr?.on('data', (d: Buffer) => { stderr += d.toString(); });
      proc.on('close', () => {
        try { results.push({ name: file.name, ...JSON.parse(stdout.trim()) }); }
        catch { results.push({ name: file.name, success: false, message: stderr.trim() || 'Unknown error', bytes_recovered: 0 }); }
        resolve();
      });
      proc.on('error', (err: Error) => {
        results.push({ name: file.name, success: false, message: err.message, bytes_recovered: 0 });
        resolve();
      });
    });
  }
  if (mainWindow) {
    mainWindow.webContents.send('recover-progress', { current: files.length, total: files.length, fileName: '', percent: 100 });
  }
  return {
    recovered: results.filter((r) => r.success).length,
    failed: results.filter((r) => !r.success).length,
    total: files.length,
    results,
  };
});

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
    // If it's a full path like "C:\Users\Desktop", extract just the drive letter.
    // If it's "C:" or "C", keep just the letter.
    const driveArg = driveLetter.length > 2
      ? driveLetter.charAt(0).toUpperCase()
      : driveLetter.replace(':', '').toUpperCase();
    
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
            lastScanResult = result;
            if (mainWindow) {
              mainWindow.webContents.send('scan-progress', {
                status: 'complete',
                message: 'Scan complete!',
                progress: 100,
              });
            }
            // Resolve only metadata — file arrays stay in main process
            resolve(stripMeta(result));
          } catch (parseError) {
            reject(new Error('Failed to parse scan results'));
          }
        } else {
          // Even if exit code is non-zero, try to parse stdout (Rust backend sometimes exits 1 with valid results)
          if (stdout.trim()) {
            try {
              const result = JSON.parse(stdout.trim());
              lastScanResult = result;
              resolve(stripMeta(result));
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

// Open a folder in Windows Explorer
ipcMain.handle('open-folder', async (_event, folderPath: string) => {
  await shell.openPath(folderPath);
});

// Recover selected files using Rust backend
ipcMain.handle('recover-files', async (_event, driveLetter: string, files: any[], destinationFolder: string) => {
  const backendPath = getRustBackendPath();
  const driveArg = driveLetter.length > 2
    ? driveLetter.charAt(0).toUpperCase()
    : driveLetter.replace(':', '').toUpperCase();

  const results: any[] = [];

  for (let i = 0; i < files.length; i++) {
    const file = files[i];

    if (mainWindow) {
      mainWindow.webContents.send('recover-progress', {
        current: i + 1,
        total: files.length,
        fileName: file.name || '',
        percent: Math.round(((i) / files.length) * 100),
      });
    }

    await new Promise<void>((resolve) => {
      const fileJson = JSON.stringify(file);

      // Rust's save_carved_file expects a full FILE path, not a folder.
      // Build: <destinationFolder>/<sanitized_filename>
      // Sanitize name: strip chars that are illegal in Windows filenames.
      const rawName: string = file.name || `recovered_file_${i + 1}`;
      const safeName = rawName.replace(/[<>:"/\\|?*\x00-\x1f]/g, '_');

      // Avoid collisions: if a file with this name already exists, suffix with index.
      let destFilePath = path.join(destinationFolder, safeName);
      const { existsSync } = require('fs');
      if (existsSync(destFilePath)) {
        const ext = path.extname(safeName);
        const base = path.basename(safeName, ext);
        destFilePath = path.join(destinationFolder, `${base}_recovered_${i + 1}${ext}`);
      }

      const proc = spawn(backendPath, ['recover-deleted', driveArg, fileJson, destFilePath]);
      let stdout = '';
      let stderr = '';
      proc.stdout?.on('data', (d: Buffer) => { stdout += d.toString(); });
      proc.stderr?.on('data', (d: Buffer) => { stderr += d.toString(); });
      proc.on('close', () => {
        try {
          results.push({ name: file.name, ...JSON.parse(stdout.trim()) });
        } catch {
          results.push({ name: file.name, success: false, message: stderr.trim() || 'Unknown error', bytes_recovered: 0 });
        }
        resolve();
      });
      proc.on('error', (err: Error) => {
        results.push({ name: file.name, success: false, message: err.message, bytes_recovered: 0 });
        resolve();
      });
    });
  }

  if (mainWindow) {
    mainWindow.webContents.send('recover-progress', {
      current: files.length,
      total: files.length,
      fileName: '',
      percent: 100,
    });
  }

  return {
    recovered: results.filter((r) => r.success).length,
    failed: results.filter((r) => !r.success).length,
    total: files.length,
    results,
  };
});

// Relaunch the app with Administrator privileges (UAC prompt on Windows)
ipcMain.handle('relaunch-as-admin', async () => {
  try {
    const execPath = process.execPath;
    // In development the exec path is the electron binary itself; pass the app path as arg
    const appArgs = process.argv.slice(1).map((a) => `"${a}"`).join(' ');
    const cmd = `Start-Process "${execPath}" ${appArgs ? `-ArgumentList ${appArgs}` : ''} -Verb RunAs`;
    spawn('powershell', ['-Command', cmd], { detached: true, stdio: 'ignore' }).unref();
    setTimeout(() => app.quit(), 500);
    return { success: true };
  } catch (err: any) {
    return { success: false, message: err.message };
  }
});

app.whenReady().then(() => {
  // Serve local filesystem images via localfile:// to avoid cross-origin blocks
  protocol.handle('localfile', (req) => {
    const withoutScheme = req.url.slice('localfile://'.length) // e.g. '/C:/Users/...'
    const decoded = decodeURIComponent(withoutScheme)
    return net.fetch(`file://${decoded}`)
  })
  createWindow()
});

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
