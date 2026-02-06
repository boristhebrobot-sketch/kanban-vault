// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};
use tauri::{AppHandle, Manager};
use thiserror::Error;

#[derive(Debug, Error)]
enum VaultError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("yaml error: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("invalid frontmatter: {0}")]
    InvalidFrontmatter(String),
    #[error("board not found: {0}")]
    BoardNotFound(String),
}

type Result<T> = std::result::Result<T, VaultError>;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Board {
    pub id: String,
    pub title: String,
    pub columns: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub board: String,
    pub column: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub due: Option<String>,
    #[serde(default)]
    pub created: Option<String>,
    #[serde(default)]
    pub updated: Option<String>,
    #[serde(default)]
    pub body: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BoardColumn {
    pub name: String,
    pub tasks: Vec<Task>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BoardWithTasks {
    pub board: Board,
    pub columns: Vec<BoardColumn>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VaultInfo {
    pub path: String,
}

fn vault_dir(app: &AppHandle) -> Result<PathBuf> {
    // Use OS-specific app data dir so the vault persists across app restarts.
    // e.g. ~/Library/Application Support/<bundle-id>/vault
    let base = app
        .path()
        .app_data_dir()
        .map_err(|e| VaultError::InvalidFrontmatter(format!("failed to get app_data_dir: {e}")))?;
    Ok(base.join("vault"))
}

fn ensure_vault_layout(vault: &Path) -> Result<()> {
    fs::create_dir_all(vault.join("boards"))?;
    fs::create_dir_all(vault.join("tasks"))?;

    // Seed a default board + a couple sample tasks if empty.
    let default_board = vault.join("boards").join("default.md");
    if !default_board.exists() {
        fs::write(
            &default_board,
            r#"---
id: default
title: Default Board
columns:
  - Todo
  - Doing
  - Done
---

This is a starter board. Edit this file to change columns.
"#,
        )?;
    }

    let tasks_dir = vault.join("tasks");
    let has_any_task = fs::read_dir(&tasks_dir)
        .ok()
        .and_then(|mut rd| rd.next())
        .is_some();
    if !has_any_task {
        fs::write(
            tasks_dir.join("task-1.md"),
            r#"---
id: task-1
title: Welcome to Kanban Vault
board: default
column: Todo
tags: [welcome]
created: 2026-02-06
---

This is a task stored as a single Markdown file.
"#,
        )?;
        fs::write(
            tasks_dir.join("task-2.md"),
            r#"---
id: task-2
title: Drag/drop and editing (coming soon)
board: default
column: Doing
tags: [ui]
created: 2026-02-06
---

Next steps: add editing, drag/drop, and a detail pane.
"#,
        )?;
    }

    Ok(())
}

fn parse_frontmatter<T: for<'de> Deserialize<'de>>(content: &str) -> Result<(T, String)> {
    let content = content.replace("\r\n", "\n");
    if !content.starts_with("---\n") {
        return Err(VaultError::InvalidFrontmatter(
            "file must start with YAML frontmatter (---)".to_string(),
        ));
    }
    let rest = &content[4..];
    let end = rest
        .find("\n---\n")
        .ok_or_else(|| VaultError::InvalidFrontmatter("missing closing ---".to_string()))?;
    let yaml = &rest[..end];
    let body = &rest[end + 5..];
    let parsed: T = serde_yaml::from_str(yaml)?;
    Ok((parsed, body.trim().to_string()))
}

fn read_board(path: &Path) -> Result<Board> {
    #[derive(Debug, Deserialize)]
    struct BoardFm {
        id: String,
        title: String,
        columns: Vec<String>,
    }

    let raw = fs::read_to_string(path)?;
    let (fm, _body): (BoardFm, String) = parse_frontmatter(&raw)?;
    Ok(Board {
        id: fm.id,
        title: fm.title,
        columns: fm.columns,
    })
}

fn read_task(path: &Path) -> Result<Task> {
    #[derive(Debug, Deserialize)]
    struct TaskFm {
        id: String,
        title: String,
        board: String,
        column: String,
        #[serde(default)]
        tags: Vec<String>,
        #[serde(default)]
        due: Option<String>,
        #[serde(default)]
        created: Option<String>,
        #[serde(default)]
        updated: Option<String>,
    }

    let raw = fs::read_to_string(path)?;
    let (fm, body): (TaskFm, String) = parse_frontmatter(&raw)?;
    Ok(Task {
        id: fm.id,
        title: fm.title,
        board: fm.board,
        column: fm.column,
        tags: fm.tags,
        due: fm.due,
        created: fm.created,
        updated: fm.updated,
        body,
    })
}

fn list_boards_inner(vault: &Path) -> Result<Vec<Board>> {
    let mut boards = Vec::new();
    for entry in fs::read_dir(vault.join("boards"))? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("md") {
            continue;
        }
        if let Ok(board) = read_board(&path) {
            boards.push(board);
        }
    }
    boards.sort_by(|a, b| a.title.cmp(&b.title));
    Ok(boards)
}

fn list_tasks_inner(vault: &Path, board_id: Option<&str>) -> Result<Vec<Task>> {
    let mut tasks = Vec::new();
    for entry in fs::read_dir(vault.join("tasks"))? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("md") {
            continue;
        }
        if let Ok(task) = read_task(&path) {
            if board_id.map(|b| b == task.board).unwrap_or(true) {
                tasks.push(task);
            }
        }
    }
    tasks.sort_by(|a, b| a.title.cmp(&b.title));
    Ok(tasks)
}

fn board_with_tasks_inner(vault: &Path, board_id: &str) -> Result<BoardWithTasks> {
    let boards = list_boards_inner(vault)?;
    let board = boards
        .into_iter()
        .find(|b| b.id == board_id)
        .ok_or_else(|| VaultError::BoardNotFound(board_id.to_string()))?;

    let tasks = list_tasks_inner(vault, Some(board_id))?;
    let mut by_col: HashMap<String, Vec<Task>> = HashMap::new();
    for t in tasks {
        by_col.entry(t.column.clone()).or_default().push(t);
    }

    let columns = board
        .columns
        .iter()
        .map(|name| {
            let mut tasks = by_col.remove(name).unwrap_or_default();
            tasks.sort_by(|a, b| a.title.cmp(&b.title));
            BoardColumn {
                name: name.clone(),
                tasks,
            }
        })
        .collect::<Vec<_>>();

    Ok(BoardWithTasks { board, columns })
}

#[tauri::command]
fn vault_info(app: AppHandle) -> std::result::Result<VaultInfo, String> {
    (|| -> Result<VaultInfo> {
        let vault = vault_dir(&app)?;
        ensure_vault_layout(&vault)?;
        Ok(VaultInfo {
            path: vault.to_string_lossy().to_string(),
        })
    })()
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn list_boards(app: AppHandle) -> std::result::Result<Vec<Board>, String> {
    (|| -> Result<Vec<Board>> {
        let vault = vault_dir(&app)?;
        ensure_vault_layout(&vault)?;
        list_boards_inner(&vault)
    })()
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn list_tasks(app: AppHandle, board_id: Option<String>) -> std::result::Result<Vec<Task>, String> {
    (|| -> Result<Vec<Task>> {
        let vault = vault_dir(&app)?;
        ensure_vault_layout(&vault)?;
        list_tasks_inner(&vault, board_id.as_deref())
    })()
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_board_with_tasks(
    app: AppHandle,
    board_id: String,
) -> std::result::Result<BoardWithTasks, String> {
    (|| -> Result<BoardWithTasks> {
        let vault = vault_dir(&app)?;
        ensure_vault_layout(&vault)?;
        board_with_tasks_inner(&vault, &board_id)
    })()
    .map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            vault_info,
            list_boards,
            list_tasks,
            get_board_with_tasks
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
