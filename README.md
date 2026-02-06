# Kanban Vault (macOS)

A macOS-first Kanban desktop app built with **Tauri 2 + React + TypeScript**.

The core idea: a local **Markdown vault** on disk.

- **Boards** are Markdown files in `boards/`
- **Tasks** are Markdown files in `tasks/` (**one task per file**)
- Frontmatter is **YAML** (`--- ... ---`)

This repo currently ships a minimal read-only UI that:

- Ensures a vault exists in your app data directory
- Seeds a default board and a couple tasks (first run)
- Scans boards + tasks from disk
- Displays columns and cards

## Requirements

- macOS (this repo is intended to be macOS-only; it may work elsewhere but is not a goal)
- Node.js (tested with modern Node)
- Rust toolchain (via `rustup`)
- Tauri prerequisites: https://tauri.app/start/prerequisites/

## Setup

```bash
npm install
```

## Run (dev)

```bash
npm run tauri dev
```

## Vault location

On first run the app creates a vault under **Tauri app data dir**:

- `~/Library/Application Support/<bundle-id>/vault/`
  - `boards/`
  - `tasks/`

The current UI shows the exact resolved path at the top.

## File format

### Board file

Path: `vault/boards/<boardId>.md`

```md
---
id: default
title: Default Board
columns:
  - Todo
  - Doing
  - Done
---

Optional notes...
```

### Task file

Path: `vault/tasks/<taskId>.md`

```md
---
id: task-123
title: Fix syncing
board: default
column: Todo
tags: [backend, urgent]
due: 2026-03-01
created: 2026-02-06
updated: 2026-02-06
---

Optional markdown body/description.
```

Notes:
- `board` must match a board `id`
- `column` should match one of the board's `columns`

## Implemented Tauri commands

- `vault_info()` → returns vault path and seeds layout
- `list_boards()` → parses `boards/*.md`
- `list_tasks({ boardId? })` → parses `tasks/*.md`
- `get_board_with_tasks({ boardId })` → board + columns + tasks grouped by column

## Next steps (not implemented yet)

- Editing boards and tasks
- Drag & drop across columns
- Task detail view + markdown editor
- File watching for live updates
