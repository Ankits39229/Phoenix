# RecoverPro — Product & UX Analysis
**Prepared for:** Product Development & Strategy Stakeholders  
**Date:** February 28, 2026  
**Scope:** Full codebase review — frontend (React/Electron), backend (Rust), architecture, and UX

---

## Executive Summary

RecoverPro is a Windows desktop application for recovering deleted or lost files from local drives, built on an Electron + React frontend with a high-performance native Rust backend. The product is architecturally sound, with a thoughtful separation between UI and disk-access logic. The visual design is polished, with theme support, animated interactions, and careful attention to edge cases such as BitLocker-encrypted volumes.

However, a significant gap exists between the **settings UI** and **actual behaviour** — several controls are cosmetic-only with no effect on scans or recovery. There are also actionable UX issues around bulk recovery flows, missing sort/filter capability, and incomplete feedback for users during long operations. This report details all findings by panel and closes with prioritised recommendations.

---

## Table of Contents

1. [Application Architecture Overview](#1-application-architecture-overview)
2. [TitleBar & Navigation](#2-titlebar--navigation)
3. [Dashboard Panel](#3-dashboard-panel)
4. [Pre-Scan Modal](#4-pre-scan-modal)
5. [ScanView Panel](#5-scanview-panel)
6. [Recovery Modal (Inline in ScanView)](#6-recovery-modal-inline-in-scanview)
7. [Settings Panel](#7-settings-panel)
8. [About Panel](#8-about-panel)
9. [Backend: Scanning & Recovery Engine](#9-backend-scanning--recovery-engine)
10. [Recommended New Panels & Features](#10-recommended-new-panels--features)
11. [Prioritised Action Plan](#11-prioritised-action-plan)

---

## 1. Application Architecture Overview

| Layer | Technology | Role |
|---|---|---|
| Renderer | React 18 + TypeScript + Tailwind CSS | All UI panels and state |
| Host | Electron (custom frameless window) | OS integration, IPC bridge |
| Backend | Rust binary (`data_recovery_backend.exe`) | Disk I/O, MFT parsing, file carving, recovery |
| IPC | contextBridge + ipcMain/ipcRenderer | Typed message passing |
| Theming | CSS custom properties + `data-theme` attribute | 6 themes (Default, Material Light/Dark, Mono Light/Dark, Automatic) |

**Strengths:**
- File arrays are kept in the main process (`lastScanResult`) and only paginated slices reach the renderer — excellent for memory safety with large drives.
- The Rust backend is cleanly modular: MFT parsing, file carving, filesystem reader, and BitLocker handling are separate modules.
- BitLocker limitations are surfaced to the user clearly and proactively at multiple points.
- The custom `localfile://` protocol for image thumbnails correctly avoids CSP cross-origin blocks.

---

## 2. TitleBar & Navigation

### Purpose & Integration
The `TitleBar` component provides the window chrome (minimize / maximize / close) and the primary navigation between **Recover**, **Settings**, and **About** pages. Because the window is frameless, it uses a `drag-region` CSS class so users can still move the window.

### Issues Found

| Severity | Issue | Details |
|---|---|---|
| **Medium** | Navigation silently disabled during scan | `handleNavigate` returns early when a scan is in progress, but the nav tabs remain fully visible and clickable-looking with no disabled visual state. Users clicking Settings mid-scan receive no feedback. |
| **Low** | Logo placeholder | The logo area renders a generic div circle with no branding. Shipping without a real icon undermines professional credibility. |
| **Low** | No keyboard shortcut support | Window controls and nav have no keyboard equivalents (`Alt+F4` aside), which affects power users and accessibility. |

### Improvement Suggestions
- Apply `opacity-50 cursor-not-allowed` to nav tabs during an active scan and add a tooltip: *"Navigation is disabled while a scan is in progress."*
- Replace the placeholder logo with an actual SVG/ICO asset.
- Add `aria-label` attributes to window control buttons for accessibility.

---

## 3. Dashboard Panel

### Purpose & Integration
The Dashboard is the entry point. It detects and lists all available hard drives via the Rust backend, offers Quick Access shortcuts to Desktop and Downloads folders, and allows users to browse to any custom folder. Selecting a location triggers the Pre-Scan Modal.

### Key Features
- Animated hover-effect cards (`card-hover-effect.tsx`) powered by Framer Motion
- BitLocker lock indicator per drive
- Storage usage bar with colour-coded fill (green → yellow → red)
- Graceful error state with retry capability

### Issues Found

| Severity | Issue | Details |
|---|---|---|
| **Medium** | No drive refresh after modal cancel | If the user dismisses the Pre-Scan Modal and a USB drive is inserted/removed, the drive list is stale until a full app reload. There is no manual refresh button on the Dashboard. |
| **Medium** | Quick Access items are clickable while loading | When `specialFolders` is `null` (still loading), items are marked `disabled` but the label still shows "Loading..." without a skeleton/shimmer, which can look broken. |
| **Low** | Drive size format inconsistency | `formatBytes` in `Dashboard.tsx` uses `gb.toFixed(2)` (e.g. "476.94 GB") while `formatBytes` in `ScanView.tsx` uses `toFixed(2)` with `['B','KB','MB','GB','TB']`. Two separate helper functions exist when one shared utility would suffice. |
| **Low** | Locked BitLocker drives are shown but not actioned | Drives with `isLocked: true` appear with only a lock icon. There is no "Unlock" affordance or explanation beyond the icon, leaving users without guidance. |
| **Low** | No drive type differentiation | All drives (HDD, SSD, USB, CD-ROM) display the same `HardDrive` icon. Users cannot tell at a glance what kind of physical media they are scanning. |

### Improvement Suggestions
- Add a small "Refresh" icon button in the section header to re-query drives on demand.
- Differentiate drive icons (USB = `Usb`, Optical = `Disc`, SSD = distinct icon).
- Show a contextual action for locked BitLocker drives (e.g. "Unlock Drive" button opening a password dialog).

---

## 4. Pre-Scan Modal

### Purpose & Integration
Intercepts the drive-start flow and presents a choice between **Quick Scan** (up to 100K MFT records, seconds–minutes) and **Deep Scan** (up to 500K MFT records + sector carving, minutes–hours). Displays a BitLocker limitation notice when relevant.

### Key Features
- Inline mode comparison with bullet-level feature lists
- BitLocker-aware conditional rendering (strikethrough for unavailable Deep Scan features)
- Estimated time tag per mode

### Issues Found

| Severity | Issue | Details |
|---|---|---|
| **High** | Scan limits in modal don't match Settings | The modal hardcodes "100,000 MFT records" and "500,000 MFT records" in its bullets. The Settings panel has sliders to change these limits, but those values are never read by the Rust backend invocation — the Rust binary is always called with fixed arguments regardless of what the user set in Settings. The modal's stated numbers are therefore always accurate by accident, but Settings gives a false impression of control. |
| **Medium** | Deep Scan selectable on BitLocker with no confirmation | Although a warning is shown, users can still confirm a Deep Scan on an encrypted volume. The scan proceeds but silently falls back to MFT-only. A confirmation step ("This drive is encrypted — Deep Scan will behave like Quick Scan in terms of carving. Continue?") would set expectations. |
| **Low** | Modal uses fixed 520px width | On minimum window size (900px), the modal can feel cramped. A `max-w-[520px] w-full` responsive pattern is safer. |
| **Low** | No estimated time for the specific drive size | The time estimates ("Seconds – minutes") are static strings. Even a rough estimate based on drive size would be more informative. |

---

## 5. ScanView Panel

### Purpose & Integration
The core working surface of the application. Displays real-time scan progress, a dual-tab sidebar (File Type / File Location), a three-mode file browser (Grid / List / Detail), search, filter controls, and the Recover button. This panel handles the entire scan lifecycle.

### Key Features
- Paginated file loading — only 60 files at a time rendered in the DOM
- Per-category file counts in the sidebar
- Folder tree sidebar with top-150 directories by file count
- Inline image previews via `localfile://` protocol
- Recovery chance indicator with colour-coded thresholds (green ≥80%, yellow ≥50%, red <50%)
- "Recover All" vs "Recover Selected" logic separated cleanly

### Issues Found

**Functional / Data Bugs**

| Severity | Issue | Details |
|---|---|---|
| **High** | `src/types.ts` is completely empty | `DriveInfo`, `RecoverableFile`, `ScanProgress`, `ScanResult`, and `FilesPageResult` are referenced across all components with no import statements. This compiles only because these types must exist as global ambient declarations somewhere outside this file — a fragile setup that will break if that global source changes. |
| **High** | Breadcrumb drive letter has `cursor-pointer` but no `onClick` | In the top nav bar, the drive letter (`{drive.name}:`) has `hover:text-blue-500 cursor-pointer` styling but no click handler — nothing happens when clicked. Users may expect it to act as a "clear filter" or "go to root" action. |
| **Medium** | "Select All" is scoped to loaded pages only | The checkbox selects/deselects only files currently in `displayedFiles` (the accumulated set). If 10,000 files match a filter but only 60 are loaded, "Select All" selects 60 while the counter still shows "Select All" until Load More is used. This is confusing when the Recover button then says "Recover (60)" implying the user would miss 9,940 files. |
| **Medium** | No sort controls | The file list has no sorting by Name, Size, Date Modified, or Recovery Chance. Recovery Chance sorting in particular would be highly valuable — users want to attempt high-confidence files first. |
| **Medium** | "Load More" does not scroll the viewport | After clicking Load More, new files are appended but the scroll position does not move to reveal them. There is no visual feedback that new content was added. |
| **Medium** | Grid view `grid-cols-6` is fixed | At minimum window width (900px), with a 288px sidebar, the content area is ~580px, making 6-column cards approximately 90px wide — barely usable. Responsive columns (`grid-cols-3 sm:grid-cols-4 lg:grid-cols-6`) would be appropriate. |
| **Low** | Recovery chance `< 50` shows `text-yellow-500` not `text-red-400` in Detail view | In Grid and List views, <50% uses `text-red-400`, but in Detail view there is no `< 50` branch — it falls through to `text-yellow-500`. Minor visual inconsistency. |
| **Low** | Filter panel only closes via its X button | There is no click-outside-to-close behaviour. The panel stays open even when the user starts interacting with the file list. |

**UX / Interaction Issues**

| Severity | Issue | Details |
|---|---|---|
| **High** | No cancel option during active recovery | Once recovery starts, the modal shows "Please wait, do not close the app…" with no Cancel button. Recovering hundreds of files via sequential Rust process spawning can take many minutes. Users have no escape short of force-quitting. |
| **Medium** | No confirmation dialog before "Recover All" | When no files are selected, clicking Recover immediately opens a folder picker and then begins recovering every file that matches active filters — potentially thousands. A confirmation step ("You are about to recover X files to [destination]. Continue?") is strongly recommended. |
| **Medium** | Recovery-All hint text is easily missed | The hint ("No files selected — clicking Recover will restore all N") sits in the bottom bar in small text. Many users will miss this and be surprised. |
| **Low** | Search field lacks a clear/X button | To clear the search, users must manually delete the text. A clear button inside the input (common pattern) would improve efficiency. |
| **Low** | "typo": `relunchingAdmin` | The state variable is named `relunchingAdmin` (should be `relaunchingAdmin`). Low user impact but unprofessional in the codebase. |
| **Low** | No "Scan Again" shortcut after completion | Completing a scan and wanting to run another (e.g. try Deep after Quick) requires navigating back to the Dashboard and starting the flow again. A "Scan Again" button on the bottom bar would match user mental models. |
| **Low** | Scan completion has no sound or OS notification | The Settings panel has a "Scan completion notifications" toggle, but it has no implementation — no `new Notification(...)` call or Electron `Notification` API usage. |

---

## 6. Recovery Modal (Inline in ScanView)

### Purpose & Integration
A full-screen overlay that appears during and after file recovery. Shows a progress bar, current filename, and final success/failure summary with the option to open the destination folder.

### Key Features
- Animated progress bar with percentage
- Success/failed counts in colour-coded cards
- Scrollable failed-file list with individual error messages
- "Open Folder" shortcut after completion

### Issues Found

| Severity | Issue | Details |
|---|---|---|
| **High** | No cancel during recovery (repeated for emphasis) | As noted above, active recovery cannot be stopped. In the backend, files are recovered one-by-one in a loop with `await new Promise` per spawned process — cancellation would require signalling the main process. |
| **Medium** | Failed file error messages can be cryptic | Raw Rust error strings (e.g. "Access is denied (os error 5)") are shown directly to end users without friendly translations. |
| **Medium** | No total recovered bytes shown | The summary shows file counts (Recovered / Failed) but not the total data size recovered. This is a key metric users care about. |
| **Low** | No option to retry failed files | After recovery, failed files are listed but there is no "Retry Failed" button. Users must re-select those files manually. |
| **Low** | Auto-open folder setting is not honoured | `Settings.tsx` has an "Open destination folder after recovery" toggle, but the Recovery Modal never reads this setting — "Open Folder" is always manual. |

---

## 7. Settings Panel

### Purpose & Integration
Provides user control over appearance (6 themes with live-preview miniature windows), scan limits, recovery preferences, notifications, and a cache-clearing utility.

### Key Features
- Beautifully rendered theme preview cards with animated mini-window mockups
- Interactive sliders for MFT record limits (Quick Scan: 10K–500K, Deep Scan: 100K–2000K)
- Toggle switches for system file skipping, auto-folder-open, and notifications

### Issues Found — Critical Gap: Settings Are Not Persisted Nor Applied

| Severity | Issue | Details |
|---|---|---|
| **Critical** | Scan limit sliders have no effect | `quickScanLimit` and `deepScanLimit` state values are render-local only. The Rust backend is invoked with a fixed `mode` argument (`'quick'` or `'deep'`) and the backend's own constants determine record limits — these sliders never send values to the backend. |
| **Critical** | All non-theme settings reset on relaunch | `deepScanLimit`, `quickScanLimit`, `notifications`, `autoOpenFolder`, and `skipSystemFiles` are plain `useState` initialised with hardcoded defaults. None read from `localStorage` or any persisted config. Every app launch resets these to defaults. |
| **Critical** | "Clear scan cache" button does nothing | The Clear button in the Danger Zone section has no `onClick` handler. The button renders and is visually interactive but is completely non-functional. |
| **High** | Skip system files toggle has no effect | `skipSystemFiles` state is toggled in the UI but never passed to the scan invocation or read by the backend. |
| **High** | Notifications toggle has no implementation | After enabling this toggle, no notification is ever sent on scan completion. There is no call to `new window.Notification(...)` or `electron.Notification` anywhere in the codebase. |
| **High** | Auto-open folder has no implementation | As noted in §6, the Recovery Modal does not read or honour this setting. |
| **Medium** | "Current" badge on Default theme is hardcoded | The theme card array has `badge: 'Current'` hardcoded on the `default` entry. This badge shows even when `material-dark` is the active theme. It should reflect the actual current theme, or the badge should be removed. |
| **Low** | Invalid CSS hex colour in index.css | In the Material Dark theme block: `--app-bg: linear-gradient(160deg, #10_0E14 0%, ...)` — the value `#10_0E14` contains an underscore and is not valid hex. The corrected duplicate below it overrides this, but the invalid declaration is noise and could cause confusion. |

---

## 8. About Panel

### Purpose & Integration
A static informational panel presenting the app version, tagline, and feature highlights (Quick Scan, Deep Scan, Multi-Drive, Privacy First). Intended to build user trust and communicate the product's value proposition.

### Issues Found

| Severity | Issue | Details |
|---|---|---|
| **Medium** | Version is hardcoded as `'1.0.0'` | The version string `const VERSION = '1.0.0'` is a literal constant with no connection to `package.json`. This will drift out of sync as the product is updated. It should pull from `app.getVersion()` via an IPC call or be injected at build time. |
| **Medium** | No support, documentation, or feedback links | There is no "Report a Bug," "View Documentation," or "Contact Support" link. Users experiencing issues have nowhere to go from within the app. |
| **Low** | No build date or changelog link | Professional utilities typically show build date or a "What's New" link. |
| **Low** | The licence, copyright, and privacy policy are absent | For a product with Administrator-level access, users and organisations may expect a privacy statement ("no telemetry, no uploads") to be formally present rather than limited to a single feature card bullet. |
| **Low** | Feature grid is static and could be contextual | All four features always show. For example, if the app detects it has no admin rights, the "Quick Scan" card could show a note about limitations. |

---

## 9. Backend: Scanning & Recovery Engine

The scanning and recovery engine is the core of the product — it reads the drive, finds deleted files, and writes them back to disk. The issues below are specific to how that engine behaves, not how it looks.

---

### 9.1 Scan limits set by the user are never used

**Problem:** The Settings screen has two sliders that let users control how many file records the scanner reads — one for Quick Scan and one for Deep Scan. However, these values are never actually sent to the scanning engine. The engine always uses its own fixed internal limits, regardless of what the user set.

**Impact:** Users who lower the limit expecting a faster scan, or raise it expecting a more thorough one, will see no difference in results or speed. More importantly, the Settings screen creates a false sense of control. Users troubleshooting a slow scan or missing files will adjust these sliders and reach incorrect conclusions.

**Solution:** When starting a scan, pass the user's chosen limits directly to the scanning engine as arguments. The engine already accepts a scan mode argument — the same approach should be used for record limits.

---

### 9.2 Recovering files one at a time makes batch recovery very slow

**Problem:** When recovering multiple files, the engine processes each file individually — it finishes one, then starts the next, and so on. For 50 files, that means 50 separate back-to-back operations with no overlap.

**Impact:** Recovering a large selection (hundreds of files) takes significantly longer than it needs to. On slower drives, this can stretch a recovery job from seconds to many minutes, during which the user must keep the app open and wait with no option to cancel.

**Solution:** The engine should accept a list of files in a single call rather than one file at a time. Alternatively, multiple recoveries can be run in parallel (e.g. four at once) to cut total time substantially. Either approach would also simplify the recovery logic on the app side.

---

### 9.3 Recovery cannot be stopped once started

**Problem:** There is no way to cancel a recovery job after it begins. The only options are to wait for all files to finish or force-quit the application entirely.

**Impact:** If a user accidentally starts a "Recover All" with thousands of files, or selects the wrong destination folder, they cannot stop the job without closing the app. Force-quitting mid-recovery risks leaving partial files at the destination with no clear indication of what succeeded.

**Solution:** The engine should support a stop signal. The app already has a cancel mechanism for scans — the same pattern should be applied to recovery. When cancelled, the engine should finish the file it is currently writing (to avoid corruption) and then stop cleanly.

---

### 9.4 Failure messages shown to users are raw system errors

**Problem:** When the engine cannot recover a file, it returns the exact error message from the operating system — for example, *"Access is denied (os error 5)"* or *"The system cannot find the path specified (os error 3)"*. These messages are shown word-for-word in the recovery results screen.

**Impact:** Most users do not know what OS error codes mean. Seeing technical error strings in a recovery tool that they trusted with important files damages confidence, especially in stressful data-loss situations.

**Solution:** The engine (or the app layer that receives its output) should map known error types to plain descriptions. For example:
- "Access is denied" → *"Windows blocked access to this file. Try running the app as Administrator."*
- "Path not found" → *"The original location of this file no longer exists on the drive."*
- Any other error → *"This file could not be recovered. It may be too overwritten to restore."*

---

### 9.5 Deep Scan on an encrypted drive silently behaves like a Quick Scan

**Problem:** When scanning an encrypted (BitLocker) drive, the engine cannot read raw disk sectors — so it skips the deep-level file carving entirely and only reads the standard file table. This fallback happens automatically without any notification mid-scan.

**Impact:** A user who selected Deep Scan expecting comprehensive results will receive Quick Scan-level results, but the scan status will still say "Deep Scan" and take the same amount of time. They have no way of knowing the scan was limited unless they read the small notice that appears after the scan finishes.

**Solution:** As soon as the engine detects it has fallen back to a limited scan, it should send a clear mid-scan notice to the app — not just a post-scan banner. The scan label should change to reflect what actually ran (e.g. "Deep Scan — Encryption Limit Applied"). Ideally, when the user picks Deep Scan on a BitLocker drive, they should be asked to confirm before the scan starts, since the result will not match their expectation.

---

### 9.6 The "skip system files" setting has no effect on scan results

**Problem:** The Settings screen has a toggle to exclude operating system files (like the Windows page file and internal logs) from scan results. This setting is never passed to the scanning engine — it always includes system files in results regardless of the toggle state.

**Impact:** Users who turn this on to reduce noise in their results will still see system files filling the list. On a Windows drive, this can be hundreds of entries with no user value. It also makes the toggle feel broken, which reduces trust in other settings.

**Solution:** When starting a scan, include the system-file exclusion flag in the engine's arguments. The engine already identifies system files during MFT parsing — the filter just needs to be applied when that setting is on.

---

### 9.7 Duplicate scanning module exists but is not connected

**Problem:** There are two scanning modules in the engine's source — one older and one newer. The older one still exists as a file but is not used by any part of the engine.

**Impact:** No direct user impact, but it creates risk: a developer might accidentally modify the wrong module, or future changes to the active module may never be applied to the dormant one if they are mistakenly believed to be the same. Over time, this makes the codebase harder to maintain safely.

**Solution:** Remove the unused module. If it contains logic that differs from the active one, document what was intentionally left out before deleting it.

---

### 9.8 Deep Scan completes too quickly and may not be scanning at all

**Problem:** Deep Scan is expected to be a slow, thorough operation — yet it often finishes in seconds. The investigation reveals four compounding reasons:

1. **BitLocker drives are silently rerouted.** When the target drive is BitLocker-encrypted (even if unlocked), the engine automatically switches to a "filesystem mode" that only reads the Windows MFT index — no raw sector carving is done at all. The scan completes in seconds because it is, effectively, a Quick Scan in disguise. The user is shown a subtle note in the result message, but the scan label still says "Deep Scan", which is misleading.

2. **Raw disk reads may fail silently on modern Windows.** The carving phase reads the physical drive sector-by-sector using a low-level Windows handle (`\\.\C:`). Even when the app is running as Administrator, Windows can block raw sector access on system drives that are in active use. When this happens, every read returns empty data and the loop exits instantly — with no error shown to the user. The scan simply ends with zero carved files and a success status.

3. **An internal time cap limits how much the engine actually checks.** Even when raw I/O succeeds, the carving phase is capped at 100 million sectors (roughly 50 GB worth of data). The developer intends this to keep scans "under 10 minutes", so on large modern drives (1 TB+), most of the drive is never scanned.

4. **The scan stops after finding 50,000 carved results.** If the carving phase finds 50,000 file signatures, it stops regardless of how much drive space remains unscanned.

**Impact:** Users choose Deep Scan precisely because they believe it is doing more work. When it finishes in the same time as Quick Scan (or faster), they either assume the data is unrecoverable, or worse, trust results that are actually incomplete. In the BitLocker case, they may never know that the thorough scan they ran did not include any sector-level carving. Files that would only be found by raw carving — fragments not tracked in the MFT, files deleted long ago — are silently missed.

**Solution:**
- When the engine falls back to filesystem mode (BitLocker), show a clear, visible banner in the scan results — not a note buried in the results message — stating that sector carving was skipped and recovery may be incomplete.
- Log the reason when raw sector reads return empty data, and display a plain-language error to the user ("Could not read raw sectors. Some deleted files may not appear in results.") rather than showing a normal successful result.
- Replace the hard-coded 100-million-sector internal cap with the user-configurable Deep Scan limit already visible in the Settings screen (which currently has no effect). This connects the existing UI control to the actual engine behaviour.
- Show a live progress message during the carving phase ("Scanning sector X of Y") so users can see that work is happening and estimate remaining time, rather than assuming the scan is frozen or has already finished.

---

### 9.9 Recovered files are empty or corrupt, yet the UI reports success

**Problem:** After a scan shows hundreds of results with high recovery-chance scores, attempting recovery frequently produces files that are blank (all zeros) or contain scrambled, unreadable data. Despite this, the recovery result screen shows green ticks and counts those files as "recovered." Four separate problems combine to cause this:

1. **The recovery-chance percentage does not measure what users think it does.** The score — displayed in green for anything 80% or above — is calculated only from whether the MFT entry contains a cluster address. If the cluster number is present and non-zero, the file receives 85%. It does not check whether those disk clusters have since been taken over by new data. A photo deleted two years ago will show 85% if the MFT record still has its old cluster address intact.

2. **Overwritten clusters produce files with someone else's content.** When a file is deleted, Windows marks its clusters as free and may immediately reuse them for new files. When the recovery engine reads those clusters now, it captures whatever is currently stored there — fragments of newer files, Windows update data, or random other content. The result is a file of the correct size but wrong bytes. No error is raised; the engine saves it and reports success.

3. **Blocked cluster reads produce zero-filled files.** If Windows prevents raw cluster access (which is common on active system drives), every read for that cluster returns an error. The engine's policy when a read fails is to fill that section with zeros in order to "maintain file structure." A 5 MB document where all reads fail is saved as a 5 MB file of nothing but zeros. `success: true` is returned to the UI.

4. **Carved files are saved with guessed sizes, often containing data from surrounding files.** The carving phase identifies a file boundary by its header signature (e.g. the JPEG magic bytes). When the file type has no footer signature — which is most types — the engine does not know where the file ends, so it reads up to the file type's maximum allowed size (e.g. up to 200 MB for JPEG). The actual file may be 500 KB, but the engine writes up to 200 MB, capturing data belonging to entirely different files stored after it on disk. Even when validation detects the header does not match, the engine returns `success: true` with a note saying "file may be partially corrupted" — the UI still counts it as a recovered file.

**Impact:** Users watch the recovery progress bar complete, see "Recovered: 47/47 files ✓", open the destination folder, and find files they cannot open. This is the most damaging failure in the product — it creates false confidence, wastes time, and may cause users to stop trying to recover genuinely salvageable data. It also means high recovery-chance scores are not meaningful; a 90% score may still produce an unreadable file.

**Solution:**
- Before recovery, check whether the target clusters currently hold data matching the file's expected type (by reading and checking the first few bytes against the known file signature). If the check fails, mark the file as "likely overwritten" and lower its recovery-chance score accordingly. Do not score based solely on whether a cluster address is known.
- After recovery, do not report `success: true` when the header validation check fails. Distinguish between three outcomes: "Recovered successfully", "Recovered but content may be corrupted", and "Recovery failed — data overwritten". The UI should display these differently, not all as green ticks.
- When cluster reads return errors, report the file as failed rather than saving a zero-padded placeholder as a successful recovery. A zero-filled file is not a recovered file.
- For the carving path, cap the bytes written at the size indicated by the file's footer when a footer is found, rather than always reading to the maximum allowed size. For file types without footer support, write only one or two cluster-sizes worth of data and clearly label the result as a fragment, not a complete file.

---

## 10. Recommended New Panels & Features

### 10.1 Recovery History Panel
**Pain point addressed:** Users have no record of what was recovered, when, or where. If recovered files are accidentally deleted or a recovery partially fails, there is no audit trail.  
**Proposal:** A "History" page (new TitleBar tab) that stores completed recovery sessions in `localStorage` or a local JSON file. Each entry shows: date/time, drive scanned, files recovered, destination, success/fail counts. Allow re-opening the destination folder or exporting the log.

### 10.2 File Preview Drawer
**Pain point addressed:** Identified files can only be seen as small grid cards or text-only rows. Users have no way to inspect a file before committing to recovery.  
**Proposal:** A slide-in right panel or modal triggered by right-clicking or double-clicking a file that shows: file name, full path, size, recovery chance, last modified date for non-deleted files, and a preview for images. For documents, show metadata (page count, author) if available from MFT attributes.

### 10.3 Scan Comparison / Session Management
**Pain point addressed:** Users who run a Quick Scan and then a Deep Scan cannot compare results or track what changed.  
**Proposal:** Allow saving scan sessions and reopening them without re-scanning. Display a "Previous Sessions" section on the Dashboard for recently scanned drives with result counts and time elapsed since scan.

### 10.4 Advanced Filter Panel
**Pain point addressed:** The current filter offers only "Deleted Only" and three recovery-chance thresholds. Users dealing with thousands of results need more.  
**Proposal:** Expand the filter panel to include:
- Date range filter (file modification date)
- Size range filter (e.g. only files >1 MB)
- Multiple simultaneous category selection
- Custom extension input

### 10.5 Drive Health / SMART Overview
**Pain point addressed:** Users recovering files from a failing drive need to know urgency. The app provides no signal about drive health.  
**Proposal:** On the Dashboard, show a basic drive health status icon (Good / Warning / Critical) sourced from SMART data via the Rust backend (libraries like `smartmontools`-equivalent exist in Rust). A "Warning" badge on a drive alerts users to recover files urgently.

---

## 11. Prioritised Action Plan

### P0 — Must Fix (Breaks Trust or Core Function)

| # | Area | Action |
|---|---|---|
| 1 | Backend — Deep Scan | When the engine falls back to filesystem mode on an encrypted drive, show a prominent banner in the results — not a buried note — telling the user that raw sector carving was skipped and results may be incomplete. |
| 2 | Backend — Deep Scan | Detect when raw sector reads fail silently (returning empty data) and show the user a plain error message rather than displaying a normal successful scan result with no carved files. |
| 3 | Backend — Deep Scan | Replace the hardcoded internal sector cap with the Deep Scan limit setting from the Settings screen, so the user's configured value is what the engine actually respects. |
| 4 | UI — Deep Scan | Add a live progress message during the sector-carving phase so users can see work is happening, instead of the scan appearing to complete instantly or freeze. |
| 5 | Backend — Scanning | Pass the scan limit values the user sets in Settings to the scanning engine when a scan starts, so those settings actually take effect. |
| 6 | Backend — Scanning | Pass the "skip system files" setting to the scanning engine so it filters out OS files when the user has turned that option on. |
| 7 | Backend — Recovery | Add a Cancel button to the recovery screen. When pressed, the engine should finish the file it is currently writing and then stop cleanly. |
| 8 | Backend — Recovery | Stop reporting `success: true` when recovered file header validation fails or all cluster reads return zeros. The UI should show three distinct outcomes: success, corrupted, and failed — not all as green ticks. |
| 9 | Backend — Recovery | Before recovery, read the first few bytes of the target clusters and check them against the expected file type signature. If they do not match, lower the recovery-chance score to reflect that the clusters have likely been overwritten with new data. |
| 10 | UI — Settings | Save all settings (scan limits, skip system files, notifications, auto-open folder) so they are remembered the next time the app is launched. |
| 11 | UI — Settings | Make the "Clear scan cache" button function — it currently does nothing when clicked. |
| 12 | UI — Recovery | Add a confirmation step before "Recover All" that shows the user exactly how many files will be recovered and where, before anything begins. |

### P1 — Should Fix (Meaningfully Degrades Experience)

| # | Area | Action |
|---|---|---|
| 13 | Backend — Recovery | Replace raw system error messages in the recovery results with plain explanations users can act on (e.g. "Windows blocked access — try running as Administrator"). |
| 14 | Backend — Scanning | When a Deep Scan on an encrypted drive falls back to a limited scan, notify the user during the scan — not only after it finishes. |
| 15 | Backend — Recovery | Update the recovery engine to accept multiple files in one call, so batch recovery does not need to process files one by one. |
| 16 | UI — Settings | Send a desktop notification when a scan finishes, since the notifications toggle in Settings currently has no effect. |
| 17 | UI — Settings | Honour the "open destination folder after recovery" setting in the recovery screen — currently it is always manual. |
| 18 | UI — ScanView | Add sort controls (by name, size, or recovery chance) to the file browser so users can prioritise which files to recover first. |
| 19 | UI — Navigation | Grey out the Settings and About tabs while a scan is running, and show a tooltip explaining why, instead of silently ignoring clicks. |
| 20 | UI — Dashboard | Add a Refresh button to the drive list so users can reload drives without restarting the app (e.g. after plugging in a USB drive). |
| 21 | UI — About | Pull the app version number automatically from the build rather than keeping it as a fixed text string that falls out of date. |

### P2 — Nice to Have (Polish & Completeness)

| # | Area | Action |
|---|---|---|
| 22 | UI — ScanView | Clicking outside the filter dropdown should close it, consistent with standard UI behaviour. |
| 23 | UI — ScanView | Add a clear (×) button inside the search field so users can clear their search in one click. |
| 24 | UI — Recovery | Show the total amount of data recovered (in MB / GB) alongside the file count in the recovery summary. |
| 25 | UI — Recovery | Add a "Retry Failed" button after recovery completes, so users can attempt failed files again without re-selecting them manually. |
| 26 | UI — ScanView | Add a "Scan Again" button after a scan finishes so users can immediately run a different scan mode without going back to the home screen. |
| 27 | UI — ScanView | After clicking "Load More," scroll the view slightly to reveal the newly loaded files so users know new content was added. |
| 28 | Backend | Remove the unused duplicate scanning module from the engine's source to keep the codebase clean and unambiguous. |

---

## Closing Notes

RecoverPro has a strong foundation. The scanning engine is capable, the drive detection is reliable, and the memory strategy of keeping large scan results in the background process rather than the UI is well-considered. The most damaging gap right now is the disconnect between the Settings screen and actual engine behaviour — scanning limits, system file exclusion, and notifications all appear configurable but do nothing. For a tool that users depend on in stressful data-loss situations, that gap must be closed before expanding any other features.

The second priority is recovery reliability: adding a cancel option, replacing cryptic error messages, and speeding up batch recovery will determine whether users trust the tool enough to rely on it for serious data loss.

---

*Analysis prepared based on full source review of all TypeScript, React, CSS, Rust, and Electron configuration files in the workspace.*
