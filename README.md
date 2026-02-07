# Kanban Vault (macOS)

A macOS-first Kanban desktop app built with **Tauri 2 + React + TypeScript**.

The core idea: a local **Markdown vault** on disk.

- **Boards** are Markdown files in `boards/`
- **Stories** are Markdown files in `tasks/` (**one story per file**)
- **Projects** are Markdown files in `projects/`
- **Epics** are Markdown files in `epics/`
- Frontmatter is **YAML** (`--- ... ---`)

This repo currently ships a minimal read-only UI that:

- Ensures a vault exists in your app data directory
- Seeds a default board, project, epic, and a couple stories (first run)
- Scans boards + stories from disk
- Displays columns and cards
- Offers a wizard to create projects, epics, and stories

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
  - `projects/`
  - `epics/`

The current UI shows the exact resolved path at the top.

## OpenAI auto-fill

The story wizard can auto-fill fields via OpenAI.

Configure the API key by setting `OPENAI_API_KEY` in the environment before launching the app.
Optionally set `OPENAI_MODEL` (default: `gpt-4o-mini`) and `OPENAI_MODEL_FALLBACK` (default: `gpt-4o-mini`).

## File format

### Board file

Path: `vault/boards/<boardId>.md`

```md
---
id: default
title: Default Board
columns:
  - Inbox
  - Backlog
  - Ready
  - In Progress
  - Review
  - Done
---

Optional notes...
```

### Story file

Path: `vault/tasks/<storyId>.md`

```md
---
id: story-123
title: Fix syncing
board: default
column: Backlog
tags: [backend, urgent, story]
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
- `openai_autofill_story({ payload })` → returns suggested story fields from OpenAI

## Next steps (not implemented yet)

- Editing boards and tasks
- Drag & drop across columns
- Task detail view + markdown editor
- File watching for live updates
