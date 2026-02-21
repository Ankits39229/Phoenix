# Codebase Review — Product & UX

**Application:** Desktop file recovery tool (Electron + React + Rust backend)  
**Scope:** All UI components in `src/components/`

---

## Overview

The app has three functional screens: a drive selection dashboard, a scan/recovery view, and a chat interface. The core recovery flow is well-structured. Several components and features are either incomplete or disconnected from the app entirely.

---

## Components

### 1. TitleBar (`TitleBar.tsx`)

**Purpose:** Custom window chrome replacing the native OS title bar (minimize, maximize, close).

**Issues:**
- The logo area renders two plain circles with no icon, text, or branding. There is nothing to identify the application.
- The app has no visible name or version anywhere in the interface.

**Suggestions:**
- Add the app name and a real icon in the title bar left section.
- Consider showing the current drive/scan context in the title bar so users know what is loaded.

---

### 2. Dashboard (`Dashboard.tsx`)

**Purpose:** Entry point where users select a drive or folder to scan for recoverable files.

**Issues:**
- **Formatting bug:** The drive label renders as `Local Disk(C:)` with no space before the parenthesis. The label and drive letter are concatenated directly: `{drive.label || 'Local Disk'}({drive.name}:)`.
- **Hardcoded count:** The Quick Access section header always shows `(3)` regardless of what is available.
- **Silent failures:** If `getDrives()` or `getSpecialFolders()` throws, the error is logged to the console only. The user sees either an empty list or perpetually-disabled buttons with no explanation.
- **No empty state for drives:** If no drives are detected (e.g., permission issue), the section renders an empty grid with no message.

**Suggestions:**
- Fix the label/letter spacing: `` {drive.label || 'Local Disk'} ({drive.name}:) ``.
- Make the Quick Access count dynamic or remove it.
- Show an error banner or retry option when drive detection fails.

---

### 3. ScanView (`ScanView.tsx`)

**Purpose:** The main working screen — scans a selected drive, shows recoverable files by category, and handles file recovery to a chosen destination.

**What works well:** The category filter sidebar, three view modes (grid/list/detail), paginated file loading, recovery progress modal, and the admin-elevation flow are all solid.

**Issues:**

- **Non-functional Pause button:** The pause button toggles `isPaused.current` but never calls any backend pause API. Clicking it does nothing observable.
- **Non-functional Filter button:** The Filter button in the toolbar has no click handler and no associated state. It is a dead UI element.
- **Non-functional breadcrumb navigation:** The `ChevronLeft` and `ChevronRight` buttons next to the breadcrumb have no `onClick` handlers. They appear interactive but do nothing.
- **File Location tab is a placeholder:** The "File Location" tab in the left sidebar never renders a folder tree. After a completed scan it shows "Select a category to explore" — which is the message intended for the type tab. The folder tree is never built or displayed.
- **"Recover All" memory risk:** When no files are selected, clicking Recover fetches all matching files with `pageSize: -1`. On large drives this could load thousands of file objects into the renderer process at once.
- **Stale comment blocks:** Several comment headers in the component body — `// ── Category counts ──`, `// ── All files ──`, `// ── Filtered display files ──`, `// ── Paged slice ──` — reference code that was removed, leaving empty dead sections that add noise.
- **Recovery hint is easy to miss:** The italic text "Select files to recover, or click to recover all" sits between other controls and can be overlooked. Users may not realize the Recover button works without selecting files.

**Suggestions:**
- Either wire up the Pause button to a backend call or remove it until the feature is ready.
- Remove or hide the Filter button until filtering is implemented.
- Remove the ChevronLeft/Right buttons or disable them — they imply history navigation that doesn't exist.
- Implement the File Location tab or remove it and replace with something useful (e.g., scan statistics).
- Cap "Recover All" at a reasonable batch size or stream files to the backend in chunks.
- Make the "recover all" hint more prominent, or change the button label itself depending on selection state (which it already partially does — just reinforce this visually).

---

## Summary Table

| Component | In Use | Core Function Works | Notable Issues |
|---|---|---|---|
| TitleBar | Yes | Yes | No branding |
| Dashboard | Yes | Yes | Label formatting bug, silent failures |
| ScanView | Yes | Mostly | Pause/Filter/Nav buttons non-functional, File Location tab empty |

---

## Priority Fixes

1. **Dashboard label bug** — simple one-line fix, currently visible to all users.
2. **Remove or disable non-functional buttons** (Pause, Filter, breadcrumb arrows) — they imply capability the app doesn't have.
3. **Handle drive detection errors** — a user with permission issues currently sees a blank screen with no explanation.
4. **File Location tab** — either implement it or remove it; currently it misleads users.
5. **File Location tab** — either implement a folder tree or remove the tab entirely to free up sidebar space.
