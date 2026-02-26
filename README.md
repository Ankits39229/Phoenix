# RecoverPro

> Professional data recovery for Windows — built with Electron, React, and a native Rust recovery engine.

RecoverPro scans NTFS drives for deleted, formatted, or otherwise inaccessible files and recovers them to a destination of your choice. The recovery engine operates at two layers: an MFT parser that reconstructs deleted directory entries and a sector-by-sector file carver that finds files with no remaining MFT trace. BitLocker-encrypted (and auto-unlocked) drives are handled transparently through the Windows decryption layer.

---

## Features

| Capability | Detail |
|---|---|
| **Quick Scan** | Reads up to 100,000 MFT records in seconds to minutes; recovers recently deleted files with intact names and paths |
| **Deep Scan** | Reads up to 500,000 MFT records plus sector-by-sector file signature carving; surfaces formatted or long-deleted files |
| **BitLocker support** | Detects encrypted volumes and automatically routes to the filesystem recovery engine (Protection On / Off, Auto-Unlock) |
| **VSS integration** | Enumerates and recovers from Volume Shadow Copy snapshots |
| **File carving** | 50+ file signatures: JPEG, PNG, MP4, MKV, PDF, DOCX, ZIP, and many more |
| **Orphan detection** | Finds files whose MFT records have been recycled but whose clusters are still intact |
| **USN Journal scan** | Queries the NTFS Change Journal for recently deleted entries not visible in the MFT |
| **Paginated results** | Large result sets (100k+ files) are kept in the main process and served in pages — the renderer stays lightweight |
| **Batch recovery** | Recover selected files or all filtered results in one click; sanitised filenames, collision-safe deduplication |
| **File preview** | Live image thumbnails for non-deleted photos (JPEG, PNG, WebP, etc.) via a sandboxed `localfile://` protocol |
| **Filter & search** | Filter by category, "deleted only", minimum recovery confidence; full-text search across name, path, and extension |
| **Folder tree** | Location sidebar groups results into the top 150 parent directories by file count |
| **Dark / light theme** | System-aware, toggleable in Settings |
| **Frameless UI** | Custom title bar with native minimize / maximize / close |

---

## Requirements

| Dependency | Minimum version |
|---|---|
| Windows | 10 / 11 (64-bit) |
| Node.js | 18 LTS |
| npm | 9 |
| Rust toolchain | 1.75 (stable, `x86_64-pc-windows-msvc`) |
| Administrator rights | Required for raw disk / MFT access during scanning |

> The Rust backend calls `manage-bde` for BitLocker detection. This executable ships with all Windows 10/11 editions.

---

## Getting Started

### 1 — Install JavaScript dependencies

```bash
npm install
```

### 2 — Build the Rust backend

```bash
cd rust-backend
cargo build --release
cd ..
```

This produces `rust-backend/target/release/data_recovery_backend.exe`, which the Electron main process spawns for all disk operations.

### 3 — Start in development mode

```bash
npm run dev
```

This concurrently starts the Vite dev server (port 3000) and the Electron process with hot-reload.  
**Run the terminal as Administrator** — raw MFT access is blocked otherwise.

---

## Available Scripts

| Script | Description |
|---|---|
| `npm run dev` | Start Vite + Electron in watch mode |
| `npm run build` | Production build of both React app and Electron files |
| `npm start` | Launch the already-built production binary |

---

## Project Structure

```
.
├── electron/
│   ├── main.ts          # Electron main process — IPC handlers, process management,
│   │                    #   paginated file cache, scan/recovery orchestration
│   └── preload.ts       # Context bridge — exposes typed window.electron API
│
├── src/
│   ├── App.tsx          # Root component, view routing, PreScanModal
│   ├── types.ts         # Shared TypeScript interfaces (DriveInfo, RecoverableFile, …)
│   ├── index.css        # Global CSS variables and theme tokens
│   ├── components/
│   │   ├── Dashboard.tsx    # Drive picker, Quick Access, hover card grid
│   │   ├── ScanView.tsx     # Scan progress, file browser (grid/list/detail),
│   │   │                    #   sidebar, filters, recovery modal
│   │   ├── TitleBar.tsx     # Custom frameless window chrome + nav
│   │   ├── Settings.tsx     # Theme and preference controls
│   │   ├── About.tsx        # Version and attribution
│   │   └── ui/
│   │       └── card-hover-effect.tsx   # Animated hover card primitive
│   ├── context/
│   │   └── ThemeContext.tsx  # Dark/light theme provider
│   └── lib/
│       └── utils.ts         # Tailwind class merge helper
│
└── rust-backend/
    ├── Cargo.toml
    └── src/
        ├── main.rs                      # CLI entry point — command router
        ├── recovery_engine.rs           # Raw-disk recovery engine (unencrypted drives)
        │                                #   MFT extended scan, file carver, orphan detection
        ├── filesystem_recovery_engine.rs# FileSystem engine (BitLocker drives)
        │                                #   MFT via FSCTL, USN journal, path reconstruction
        ├── ntfs_parser.rs               # NTFS boot sector + MFT record parser, fixup arrays
        ├── file_carver.rs               # 50+ magic-byte signatures, sector chunking
        ├── disk_reader.rs               # Raw volume handle, sector I/O, IOCTL geometry
        ├── filesystem_disk_reader.rs    # Volume handle via FS APIs, MFT record read, USN
        ├── bitlocker.rs                 # manage-bde detection, unlock/lock helpers
        ├── vss.rs                       # Volume Shadow Copy enumeration and file recovery
        └── main_filesystem.rs           # Standalone filesystem-mode binary entrypoint
```

---

## Rust Backend CLI Reference

The Electron main process communicates with the backend by spawning it as a child process and reading JSON from stdout. The same binary can be invoked manually:

```
data_recovery_backend.exe <command> [args]
```

| Command | Arguments | Description |
|---|---|---|
| `drives` | — | List all detected drives with space, label, filesystem, BitLocker status |
| `check-admin` | — | Return whether the process has Administrator privileges |
| `deep-scan` | `<drive> [quick\|deep]` | Scan a drive for deleted files; auto-routes to raw or filesystem engine |
| `recover-deleted` | `<drive> <file_json> <dest_path>` | Recover a single file described by its scan JSON record |
| `bitlocker-status` | `<drive>` | Check BitLocker encryption and lock status |
| `bitlocker-unlock-password` | `<drive> <password>` | Unlock a BitLocker volume with a password |
| `bitlocker-unlock-key` | `<drive> <recovery_key>` | Unlock a BitLocker volume with a 48-digit recovery key |
| `bitlocker-lock` | `<drive>` | Lock a BitLocker volume |
| `file-signatures` | — | List all built-in file carver signatures and their stats |
| `vss-check` | — | Check whether VSS is available on this system |
| `vss-enumerate` | `<drive>` | List all shadow copy snapshots for a drive |
| `vss-list-files` | `<snapshot_json> [path]` | List files inside a snapshot |
| `vss-recover` | `<snapshot_json> <source> <dest>` | Recover a file from a shadow copy |

All commands output a single JSON object to stdout. Diagnostic/progress messages go to stderr and are parsed by the Electron main process to drive the scan progress UI.

---

## Recovery Engine Details

### Engine selection

When you start a scan RecoverPro automatically picks the appropriate engine:

```
BitLocker encrypted + unlocked  →  FileSystem engine  (MFT + USN journal)
Unencrypted                     →  Raw Disk engine    (MFT + USN + file carving)
```

### Scan modes

| Mode | MFT records | File carving | Orphan detection | Typical time |
|---|---|---|---|---|
| Quick | 50,000 | ✗ | ✗ | Seconds – minutes |
| Deep | 500,000 | ✓ | ✓ | Minutes – hours |

### Recovery confidence score

Each result carries a `recovery_chance` value (0–100 %). The score is derived from the state of the file's data runs: intact cluster chains score higher; fragmented or zeroed chains score lower. Files found only in the USN journal (MFT record reused) receive a low baseline score to reflect that sectors may have been overwritten.

---

## Architecture Notes

- **IPC boundary** — The file arrays from a scan are never serialised across the IPC bridge. They live in `lastScanResult` in the main process; the renderer requests paginated slices and filter counts via `get-files-page`.
- **Custom protocol** — Live file thumbnails are served through a `localfile://` scheme registered in the main process, bypassing Chromium's cross-origin restrictions for local filesystem paths.
- **Process isolation** — Each recovery operation spawns an independent backend process. This keeps the UI responsive and limits blast radius if a single recovery call hangs or crashes.
- **Cancellation** — Scanning can be cancelled at any time via the stop button. The backend process is killed and the scan-cancelled event is forwarded to the renderer.

---

## Building for Production

```bash
# 1. Build the Rust backend in release mode (optimised, LTO enabled)
cd rust-backend && cargo build --release && cd ..

# 2. Build the Electron + React app
npm run build
```

The release backend binary (`data_recovery_backend.exe`) must be placed in `resources/` before packaging so Electron can find it at `process.resourcesPath`.

---

## License

MIT © RecoverPro


## Features

- 🎨 Beautiful light theme with gradient background (blue to purple/pink)
- 🎯 Dashboard with action card grid for quick tasks
- 💬 Status notifications in the top bar
- 🔔 Shopping cart and notification badges
- 🎤 Voice input support (Press and hold S to speak)
- 📜 Queries history sidebar
- 🪟 Frameless custom window
- ⚡ Fast and responsive
- 📱 Resizable layout

## UI Design

The interface features:
- **Top Bar**: Order status, shopping cart with badge, notifications, and user profile
- **Main Dashboard**: Centered action cards for various tasks:
  - Order food delivery
  - Buy something
  - Order groceries
  - Book a hotel
  - Book a movie
  - Book a flight
  - Get a ride
  - My orders (with active badge)
- **Left Sidebar**: Decorative orbs and queries history dropdown
- **Voice Input**: Keyboard shortcut hint for voice commands

## Tech Stack

- **Electron** - Desktop application framework
- **React 18** - UI library
- **TypeScript** - Type safety
- **Tailwind CSS** - Utility-first CSS framework
- **Vite** - Fast build tool
- **Lucide React** - Beautiful icon set

## Installation

First, install the dependencies:

```bash
npm install
```

## Development

Run the application in development mode:

```bash
npm run dev
```

This will:
1. Start the Vite dev server for React
2. Compile the Electron TypeScript files
3. Launch the Electron application

The app will hot-reload when you make changes to the React code.

## Building

Build the application for production:

```bash
npm run build
```

This will compile both the React app and Electron files into the `dist` directory.

## Running Production Build

After building, you can run the production version:

```bash
npm start
```

## Project Structure

```
natural-ai-desktop/
├── electron/           # Electron main process files
│   ├── main.ts        # Main process entry point
│   └── preload.ts     # Preload script
├── src/               # React application
│   ├── components/    # React components
│   │   ├── TitleBar.tsx
│   │   ├── Sidebar.tsx
│   │   ├── ChatArea.tsx
│   │   └── ChatMessage.tsx
│   ├── App.tsx        # Main App component
│   ├── main.tsx       # React entry point
│   ├── index.css      # Global styles
│   └── types.ts       # TypeScript types
├── index.html         # HTML template
├── package.json       # Dependencies and scripts
├── tsconfig.json      # TypeScript config for React
├── tsconfig.electron.json  # TypeScript config for Electron
├── tailwind.config.js # Tailwind CSS configuration
├── postcss.config.js  # PostCSS configuration
└── vite.config.ts     # Vite configuration
```

## Customization

### Colors

You can customize the color scheme in `tailwind.config.js`:

```javascript
theme: {
  extend: {
    colors: {
      // Add your custom colors here
    },
  },
}
```

### Window Settings

Modify the Electron window settings in `electron/main.ts`:

```typescript
mainWindow = new BrowserWindow({
  width: 1200,
  height: 800,
  // Customize other window options
});
```

## Features to Implement

This is a UI demo. To make it functional, you can:

1. **Connect to an AI API**: Integrate with OpenAI, Anthropic, or other AI services
2. **Persistent Storage**: Add database support for conversation history
3. **Settings Panel**: Add user preferences and configuration options
4. **File Uploads**: Allow users to upload and analyze files
5. **Export Conversations**: Enable exporting chats to markdown or PDF
6. **Keyboard Shortcuts**: Add hotkeys for common actions
7. **Multi-language Support**: Add i18n for multiple languages

## License

MIT

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
