# Kanban Vault (macOS)

A macOS-first Kanban desktop app built with **Tauri 2 + React + TypeScript**.

The core idea: a local **JSON database** on disk.

- Stored in a single file: `pm-db.json`
- Contains `boards`, `tasks`, `projects`, and `epics`
- Designed for simple, fast local usage

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

## Database location

On first run the app creates a database file under **Tauri app data dir**:

- `~/Library/Application Support/<bundle-id>/pm-db.json`

The current UI shows the exact resolved path at the top.

## OpenAI auto-fill

The story wizard can auto-fill fields via OpenAI.

Configure the API key by setting `OPENAI_API_KEY` in the environment before launching the app.
Optionally set `OPENAI_MODEL` (default: `gpt-4o-mini`) and `OPENAI_MODEL_FALLBACK` (default: `gpt-4o-mini`).

## File format

`pm-db.json` (example):

```json
{
  "version": 1,
  "boards": [
    {
      "id": "default",
      "title": "Default Board",
      "columns": ["Inbox", "Backlog", "Ready", "In Progress", "Review", "Done"]
    }
  ],
  "tasks": [
    {
      "id": "story-123",
      "title": "Fix syncing",
      "board": "default",
      "column": "Backlog",
      "tags": ["backend", "urgent", "story"],
      "due": "2026-03-01",
      "created": "1770485517",
      "updated": null,
      "body": "Optional description"
    }
  ],
  "projects": [],
  "epics": []
}
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
- Task detail view
- JSON schema migrations for versioned upgrades
- Live update watching (optional)
