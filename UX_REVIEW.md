# Codebase Review — Product & UX

**Application:** Desktop file recovery tool (Electron + React + Rust backend)  
**Scope:** All UI components in `src/components/`

---

## Overview

The app has two functional screens: a drive selection dashboard and a scan/recovery view. The core recovery flow is well-structured.

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

No remaining issues.

---

### 3. ScanView (`ScanView.tsx`)

**Purpose:** The main working screen — scans a selected drive, shows recoverable files by category, and handles file recovery to a chosen destination.

**What works well:** The category filter sidebar, three view modes (grid/list/detail), paginated file loading, folder tree (File Location tab), filter popover (deleted-only + recovery chance), recovery progress modal, and the admin-elevation flow are all solid.

No remaining issues.

---

## Summary Table

| Component | In Use | Core Function Works | Notable Issues |
|---|---|---|---|
| TitleBar | Yes | Yes | No branding |
| Dashboard | Yes | Yes | — |
| ScanView | Yes | Yes | — |

---

## Priority Fixes

1. **TitleBar branding** — no app name or icon visible anywhere.
