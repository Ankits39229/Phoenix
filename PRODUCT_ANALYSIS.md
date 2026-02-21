# Product & UX Analysis Report
### Data Recovery Desktop Application
**Prepared for:** Product Development & Strategy  
**Date:** February 22, 2026  
**Stack:** Electron · React 18 · TypeScript · Tailwind CSS · Rust Backend

---

## Executive Summary

This application is a **Windows desktop file recovery tool** built on a modern Electron + React + Rust stack. The core user experience is coherent and visually polished — the glassmorphism aesthetic, responsive layout, and real-time scan feedback represent a strong foundation. However, a set of functional bugs, missing UX affordances, and structural omissions significantly undercut the experience at precisely the moments users need the most confidence — during scanning, file selection, and recovery. This report documents those gaps alongside prioritized, actionable recommendations.

---

## Application Architecture Overview

| Layer | Technology | Role |
|---|---|---|
| Shell | Electron (main.ts + preload.ts) | Process management, IPC, native OS APIs |
| UI | React 18 + Tailwind CSS | Rendering, state, interaction |
| Backend | Rust binary (data_recovery_backend) | Raw disk access, MFT parsing, file carving |
| Protocols | Custom `localfile://` | Secure in-app image preview |

The architecture is sound — the Rust backend is invoked as a child process and its results are cached in the main process, with the renderer accessing paginated slices via IPC. This keeps the UI from ever holding tens of thousands of file objects in memory, which is a meaningful design win for large drives.

---

## Component Analysis

---

### 1. TitleBar

**Purpose:** Custom frameless window chrome providing minimize, maximize, and close controls for the native OS window.

**Integration:** Rendered absolutely positioned above all views (z-index 10) so it persists across both the Dashboard and ScanView without remounting.

**Addressing User Needs:** Replaces the default OS title bar to deliver a clean, branded aesthetic consistent with the glassmorphism design system.

#### Issues Identified

| # | Severity | Description |
|---|---|---|
| T-1 | Medium | **No app identity.** The logo placeholder is an empty gray circle with no icon, wordmark, or application name. First-time users have no visual anchor for what product they are using. |
| T-2 | Low | **No application menu.** There is no access to Settings, Help, About, or version information. Users have no way to determine the app version or reach documentation. |
| T-3 | Low | **Maximize icon is semantically incorrect.** A `Square` icon is used for maximize, but on a maximized window the expected affordance is a "restore down" icon. The button does not change state visually when the window is maximized. |

#### Recommendations
- Replace the placeholder logo with the actual product icon and a short wordmark.
- Add a minimal "About" tooltip or menu accessible from the title bar.
- Listen to the Electron `maximize`/`unmaximize` window events and toggle the icon between `Square` and `Copy` (restore) variants.

---

### 2. Dashboard (Drive Selection Screen)

**Purpose:** The application's entry point. Presents detected hard drives, quick-access shortcuts (Desktop, Downloads), and a folder browser so users can target a scan location.

**Integration:** Driven by two `window.electron` async calls (`getDrives`, `getSpecialFolders`) resolved in parallel on mount. Passes a `DriveInfo` object upstream to `App.tsx`, which transitions the view to `ScanView`.

**Addressing User Needs:** Reduces friction by surfacing the most common recovery targets — physical drives and well-known folders — without requiring the user to know or type a path.

#### Issues Identified

| # | Severity | Description |
|---|---|---|
| D-1 | High | **No onboarding context.** The heading reads "Select a location to recover files" but new users are given no explanation of what a scan is, how long it takes, or what will be recovered. The product's core value proposition is never stated. |
| D-2 | Medium | **Quick Access buttons have no loading state feedback.** The Desktop and Downloads buttons are `disabled` until `specialFolders` resolves, but show no spinner or skeleton — they simply appear greyed out, which reads as "unavailable" rather than "loading." |
| D-3 | Medium | **`formatBytes` caps at GB.** The function does not handle terabyte-scale drives. A 4 TB drive would display as `4096.00 GB`, which is confusing and unprofessional for users with large storage arrays. |
| D-4 | Medium | **No scan depth selection before entering ScanView.** Users are not asked whether they want a Quick Scan or Deep Scan on the dashboard, yet the scan begins immediately on ScanView mount with `'quick'` hardcoded. This decision is invisible to users. |
| D-5 | Low | **No recent scans.** There is no history, so a user who accidentally navigated back while a scan was complete must restart the entire scan from scratch. |
| D-6 | Low | **No network/removable drive section.** USB sticks and network drives are silently absent unless the OS enumerates them as standard drives. There is no placeholder or message indicating this limitation. |

#### Recommendations
- Add a compact onboarding card or hero description (2–3 lines) explaining the tool's purpose and scan modes.
- Add a skeleton loader or spinner inside the Quick Access buttons while `specialFolders` is loading.
- Extend `formatBytes` to include TB: `if (tb >= 1) return \`${tb.toFixed(2)} TB\``.
- Introduce a scan mode selector (Quick / Deep) on the dashboard or as a pre-scan modal before ScanView mounts.
- Persist the last completed scan result (drive letter + timestamp) locally and offer a "Resume" shortcut.

---

### 3. ScanView

**Purpose:** The application's primary workhorse. Manages the full lifecycle of a scan — initializing the Rust backend, displaying real-time progress, presenting recovered files in a filterable/searchable grid or list, and executing file recovery to a user-selected destination.

**Integration:** Receives a `DriveInfo` prop and communicates exclusively with the Electron main process via IPC (`scanDrive`, `getFilesPage`, `recoverFiles`, `onScanProgress`, etc.). The Rust-side scan result is cached in the main process; the renderer only ever holds a 60-file page slice.

**Addressing User Needs:** Directly serves the primary use case — locating and recovering lost or deleted files — with category filtering, folder-tree navigation, search, and three view modes.

#### 3a. Top Navigation Bar

| # | Severity | Description |
|---|---|---|
| S-1 | Low | **Breadcrumb only shows drive + active category.** When a folder filter is active from the Location tab, the breadcrumb does not update to reflect it. Users cannot tell at a glance which folder is being filtered. |
| S-2 | Low | **Filter panel has no visible "applied" summary.** The blue dot indicator signals filters are active, but does not state what they are. Hovering or opening the panel is required to know the current state. |

#### 3b. Left Sidebar — File Type & File Location Tabs

| # | Severity | Description |
|---|---|---|
| S-3 | Medium | **"Unsaved" category is unexplained.** The category name is cryptic to non-technical users. There is no tooltip or description clarifying that it means fragments recovered via file carving (not tied to a directory entry). |
| S-4 | Medium | **Folder tree is flat.** The Location tab lists folders as a flat sorted list by file count, with no hierarchy. Long absolute paths are truncated and only the last path segment is used as the display name, leading to duplicate-looking entries (e.g., multiple folders all showing as "temp"). |
| S-5 | Low | **Switching sidebar tabs clears the opposing filter silently.** Switching from "File Type" to "File Location" clears `selectedCategory` without any notification, discarding the user's active filter. |

#### 3c. File Grid / List / Detail Views

| # | Severity | Description |
|---|---|---|
| S-6 | **Critical** | **"Load more" replaces the current page instead of appending.** Clicking "Load more" increments the page number, which triggers `getFilesPage` to fetch the *next* 60 files and calls `setPageResult`, fully replacing `pagedFiles`. The previous page's files disappear from the view. Users expecting progressive loading see their current selection context disrupted. |
| S-7 | High | **No sort controls.** Files cannot be sorted by name, size, date modified, recovery chance, or type. In lists with thousands of entries, finding the most recoverable or most recent files requires scrolling through an unsorted output. |
| S-8 | High | **No file detail / preview panel.** Clicking a file card only toggles its selection checkbox. There is no way to inspect a file's full path, last modified date, original directory, or preview its content before committing to recovery. |
| S-9 | Medium | **Grid view card is too small for meaningful information.** File names are truncated at ~24 characters at `text-[10px]` size, and file size is shown at `text-[9px]`. In a 6-column grid, cards are difficult to read and interact with, especially for users with accessibility needs. |
| S-10 | Medium | **"Deleted" badge in grid view is a tiny 14px dot** with a trash icon that is essentially invisible at normal screen resolution. Users may not notice which files are actively deleted vs. existing. |
| S-11 | Medium | **Detail view is missing the `is_deleted` badge and has no explicit checkbox.** Unlike grid and list views, the detail row only shows a selection highlight on click — there is no checkbox or deleted indicator, making selection state and file status less clear. |
| S-12 | Low | **Encoding artifact in list view.** The extension column fallback renders `—` (em dash) as the raw Unicode escape sequence `â€"` due to the source file's encoding not being interpreted correctly at runtime in that specific string literal. |
| S-13 | Low | **`relunchingAdmin` / `relunchAsAdmin` typo.** The variable name, state key, and IPC handler all use "relunch" instead of "relaunch." This surfaces in the button label `'Relaunching…'` (spelled correctly) but the code identifier will cause confusion during maintenance. |

#### 3d. Bottom Bar & Scan Progress

| # | Severity | Description |
|---|---|---|
| S-14 | High | **Scan mode is hardcoded to `'quick'`.** The call `window.electron.scanDrive(drive.letter, 'quick')` is not surfaced as a user choice. Deep scan capability exists in the Rust backend but is unreachable from the UI. Users who need to recover heavily fragmented or overwritten files have no path to a deeper scan. |
| S-15 | Medium | **"Recover All" path is ambiguous.** When no files are selected, the Recover button label reads `"Recover All (N)"` and a tooltip-style inline hint explains this. However, visually the button style is subdued (lighter gradient) while also being the most destructive action available. The UX for "I want to recover everything" vs. "I accidentally clicked" is insufficiently differentiated. |
| S-16 | Low | **Bottom bar status text is concatenated with string punctuation.** `'Quick Scanning, Files Found: '` embeds punctuation in a code string. This cannot be localized and reads awkwardly as a sentence fragment. |

#### 3e. Recovery Modal

| # | Severity | Description |
|---|---|---|
| S-17 | High | **No way to cancel an in-progress recovery.** The modal shows "Please wait, do not close the app…" with no cancel button. For bulk recoveries of thousands of files, users are locked out of the UI for extended periods with no escape. |
| S-18 | Medium | **Recovery modal does not show per-file byte progress.** The `bytes_recovered` field is returned in results but is only visible post-completion. During recovery, users see a file count but not the amount of data transferred, making it hard to estimate completion time. |
| S-19 | Medium | **Destination folder is not remembered.** Every recovery session opens a fresh folder picker. Users who repeatedly recover to the same folder must navigate there each time. |
| S-20 | Low | **Failed file list in recovery results does not distinguish error types.** All failures show a message string, but users cannot tell whether a failure was due to disk read error, file system permission, or a file that no longer exists on disk. |

---

## Cross-Cutting Issues

| # | Severity | Description |
|---|---|---|
| X-1 | High | **`src/types.ts` is empty.** All shared types (`DriveInfo`, `RecoverableFile`, `ScanProgress`, etc.) are either declared inline within component files or implicitly assumed from IPC responses. This makes the type surface impossible to discover, increases the risk of type drift between main and renderer, and complicates onboarding for new contributors. |
| X-2 | High | **README.md describes a completely different application** ("AI assistant with voice input, shopping cart, hotel booking"). The documentation is entirely mismatched with the product. Stakeholders using the README as a reference are reading incorrect information. |
| X-3 | Medium | **No loading skeleton or progressive disclosure when navigating from Dashboard to ScanView.** The transition is abrupt — blank content area renders for a frame before scan initialization begins. |
| X-4 | Medium | **Scan result is lost on navigation.** If a user clicks "Back" (or accidentally triggers `handleCancelBack`), the completed scan result is cleared from both renderer state and the main process cache. There is no warning dialog before discarding a completed scan. |
| X-5 | Low | **No keyboard navigation support.** File cards and sidebar items are not reachable via Tab, and there are no keyboard shortcuts for common actions (select all: Ctrl+A, recover: Ctrl+R, search focus: Ctrl+F). This limits accessibility. |

---

## Recommended New Panels & Features

### A. Pre-Scan Configuration Modal *(High Value)*
Presented before a scan starts, offering:
- **Scan depth selector:** Quick Scan (MFT-only, seconds to minutes) vs. Deep Scan (full sector carving, minutes to hours).
- **File type pre-filter:** Allow users to limit the scan to specific categories (e.g., "Only Photos and Videos") to reduce noise and scan time.
- **Estimated duration:** A rough estimate based on drive size and scan mode.

*Rationale:* The choice of scan depth is the single most impactful decision in recovery workflows. Hiding it degrades outcomes without informing users of the trade-off.

---

### B. File Detail / Preview Side Panel *(High Value)*
A collapsible right-side panel that activates on single-click of any file, showing:
- Full original path
- File size, extension, last modified date
- Recovery chance with a plain-language explanation ("High — file metadata intact, data likely overwritten less than 5%")
- In-app image preview (leveraging the existing `localfile://` protocol) or hex preview for documents
- A dedicated "Recover This File" button

*Rationale:* Users currently have no way to verify a file's identity before selecting it. This is especially critical for partial files or carved fragments where the name may be auto-generated.

---

### C. Scan History Panel *(Medium Value)*
A persisted log of previous scans per drive, accessible from the Dashboard:
- Drive letter, scan date, scan mode, files found, files recovered
- Option to "Resume" a cached scan result without re-scanning
- Option to "Re-scan" to refresh

*Rationale:* Recovery sessions are interrupted frequently (power loss, accidental back-navigation, reboots). A history log enables incremental workflows without starting over.

---

### D. Recovery Summary & Report Panel *(Medium Value)*
Rendered post-recovery, replacing the current modal close:
- Summary of bytes recovered, success rate, and destination folder
- A tree listing recovered files grouped by original location
- Option to export the report as a `.txt` or `.csv` for insurance or legal documentation

*Rationale:* Professional recovery workflows often require a record of what was recovered, especially in enterprise or legal contexts.

---

### E. Drive Health Panel *(Low-to-Medium Value)*
A secondary card on the Dashboard per detected drive, expandable to show:
- S.M.A.R.T. data summary (read errors, reallocated sectors, pending sectors)
- A plain-language health status: Healthy / Warning / Critical
- Guidance: "We recommend recovering files before this drive fails entirely."

*Rationale:* Users typically run recovery software because a drive is failing. Surfacing S.M.A.R.T. health reinforces urgency and differentiates the product from simpler undelete utilities.

---

## Priority Matrix

| Priority | Item | Effort | Impact |
|---|---|---|---|
| P0 | Fix "Load more" page-replace bug (S-6) | Low | High |
| P0 | Add cancel button to recovery modal (S-17) | Low | High |
| P0 | Populate `src/types.ts` with all shared interfaces (X-1) | Low | Medium |
| P0 | Correct README.md (X-2) | Very Low | Medium |
| P1 | Pre-Scan Configuration Modal — scan depth choice (S-14, Rec. A) | Medium | Very High |
| P1 | Add sort controls to all three view modes (S-7) | Medium | High |
| P1 | Warn user before navigating back from a completed scan (X-4) | Low | High |
| P1 | Fix `formatBytes` to handle TB in Dashboard (D-3) | Very Low | Medium |
| P2 | File Detail / Preview Side Panel (Rec. B) | High | High |
| P2 | Remember last recovery destination (S-19) | Low | Medium |
| P2 | Fix "Unsaved" category tooltip (S-3) | Very Low | Medium |
| P2 | Fix folder tree flat list → hierarchical display (S-4) | Medium | Medium |
| P3 | Scan History Panel (Rec. C) | High | High |
| P3 | Recovery Summary Report (Rec. D) | Medium | Medium |
| P3 | Keyboard navigation and accessibility pass (X-5) | Medium | Medium |
| P3 | Drive Health / S.M.A.R.T. Panel (Rec. E) | High | Medium |

---

## Closing Notes

The application's architectural decisions are genuinely strong — the Rust backend for raw disk access, the paginated IPC model, the custom `localfile://` protocol for image preview, and the deferred scan start via `scanStarted` ref all reflect thoughtful engineering. The visual design is polished and consistent.

The gap is predominantly in **workflow completeness** rather than visual quality. The critical path of (select drive → scan → review files → recover) works end-to-end, but breaks down in edge cases (scan interruption, large recoveries, ambiguous "Recover All" behavior) that represent the exact high-stakes scenarios real users encounter. Addressing the P0 and P1 items above would substantially close this gap and make the product suitable for professional and prosumer users.
