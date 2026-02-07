// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Manager};
use thiserror::Error;

#[derive(Debug, Error)]
enum VaultError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("yaml error: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("openai error: {0}")]
    OpenAi(#[from] reqwest::Error),
    #[error("invalid frontmatter: {0}")]
    InvalidFrontmatter(String),
    #[error("board not found: {0}")]
    BoardNotFound(String),
    #[error("OpenAI API key not configured. Set OPENAI_API_KEY in the environment.")]
    OpenAiKeyMissing,
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Project {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub owner: Option<String>,
    #[serde(default)]
    pub created: Option<String>,
    #[serde(default)]
    pub updated: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Epic {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub owner: Option<String>,
    #[serde(default)]
    pub created: Option<String>,
    #[serde(default)]
    pub updated: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct OpenAiAutoFillPayload {
    title: Option<String>,
    description: String,
    as_a: Option<String>,
    i_want: Option<String>,
    so_that: Option<String>,
    acceptance_criteria: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct OpenAiAutoFillResponse {
    title: Option<String>,
    as_a: Option<String>,
    i_want: Option<String>,
    so_that: Option<String>,
    acceptance_criteria: Option<Vec<String>>,
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

fn resolve_openai_key() -> Result<String> {
    if let Ok(value) = std::env::var("OPENAI_API_KEY") {
        if !value.trim().is_empty() {
            return Ok(value);
        }
    }

    Err(VaultError::OpenAiKeyMissing)
}

fn resolve_openai_model() -> (String, String) {
    let primary = std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4o-mini".to_string());
    let fallback = std::env::var("OPENAI_MODEL_FALLBACK").unwrap_or_else(|_| "gpt-4o-mini".to_string());
    (primary, fallback)
}

fn ensure_vault_layout(vault: &Path) -> Result<()> {
    fs::create_dir_all(vault.join("boards"))?;
    fs::create_dir_all(vault.join("tasks"))?;
    fs::create_dir_all(vault.join("projects"))?;
    fs::create_dir_all(vault.join("epics"))?;

    // Seed a default board + a couple sample tasks if empty.
    let default_board = vault.join("boards").join("default.md");
    if !default_board.exists() {
        fs::write(
            &default_board,
            r#"---
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

Project management board for the app.
"#,
        )?;
    }

    let projects_dir = vault.join("projects");
    let has_any_project = fs::read_dir(&projects_dir)
        .ok()
        .and_then(|mut rd| rd.next())
        .is_some();
    if !has_any_project {
        fs::write(
            projects_dir.join("project-1.md"),
            r#"---
id: project-1
title: Kanban Vault MVP
owner: Product
created: 2026-02-06
description: Core workflows and vault structure.
---

Core workflows and vault structure.
"#,
        )?;
    }

    let epics_dir = vault.join("epics");
    let has_any_epic = fs::read_dir(&epics_dir)
        .ok()
        .and_then(|mut rd| rd.next())
        .is_some();
    if !has_any_epic {
        fs::write(
            epics_dir.join("epic-1.md"),
            r#"---
id: epic-1
title: Wizard-driven story intake
project_id: project-1
owner: Product
created: 2026-02-06
description: Guided story creation with AI support.
---

Guided story creation with AI support.
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
            tasks_dir.join("story-1.md"),
            r#"---
id: story-1
title: Welcome to Kanban Vault
board: default
column: Inbox
tags: [welcome, story]
created: 2026-02-06
---

This is a story stored as a Markdown file inside the vault.
"#,
        )?;
        fs::write(
            tasks_dir.join("story-2.md"),
            r#"---
id: story-2
title: Try drag + drop between statuses
board: default
column: In Progress
tags: [ui, story]
created: 2026-02-06
---

Move this card across columns to update its status.
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

fn now_epoch() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

fn write_frontmatter<T: Serialize>(path: &Path, fm: &T, body: &str) -> Result<()> {
    let yaml = serde_yaml::to_string(fm)?;
    let output = format!("---\n{}---\n\n{}\n", yaml, body.trim());
    fs::write(path, output)?;
    Ok(())
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

fn read_project(path: &Path) -> Result<Project> {
    #[derive(Debug, Deserialize)]
    struct ProjectFm {
        id: String,
        title: String,
        #[serde(default)]
        owner: Option<String>,
        #[serde(default)]
        created: Option<String>,
        #[serde(default)]
        updated: Option<String>,
        #[serde(default)]
        description: Option<String>,
    }

    let raw = fs::read_to_string(path)?;
    let (fm, _body): (ProjectFm, String) = parse_frontmatter(&raw)?;
    Ok(Project {
        id: fm.id,
        title: fm.title,
        owner: fm.owner,
        created: fm.created,
        updated: fm.updated,
        description: fm.description,
    })
}

fn read_epic(path: &Path) -> Result<Epic> {
    #[derive(Debug, Deserialize)]
    struct EpicFm {
        id: String,
        title: String,
        #[serde(default)]
        project_id: Option<String>,
        #[serde(default)]
        owner: Option<String>,
        #[serde(default)]
        created: Option<String>,
        #[serde(default)]
        updated: Option<String>,
        #[serde(default)]
        description: Option<String>,
    }

    let raw = fs::read_to_string(path)?;
    let (fm, _body): (EpicFm, String) = parse_frontmatter(&raw)?;
    Ok(Epic {
        id: fm.id,
        title: fm.title,
        project_id: fm.project_id,
        owner: fm.owner,
        created: fm.created,
        updated: fm.updated,
        description: fm.description,
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

fn list_projects_inner(vault: &Path) -> Result<Vec<Project>> {
    let mut projects = Vec::new();
    for entry in fs::read_dir(vault.join("projects"))? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("md") {
            continue;
        }
        if let Ok(project) = read_project(&path) {
            projects.push(project);
        }
    }
    projects.sort_by(|a, b| a.title.cmp(&b.title));
    Ok(projects)
}

fn list_epics_inner(vault: &Path, project_id: Option<&str>) -> Result<Vec<Epic>> {
    let mut epics = Vec::new();
    for entry in fs::read_dir(vault.join("epics"))? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("md") {
            continue;
        }
        if let Ok(epic) = read_epic(&path) {
            if project_id.map(|p| epic.project_id.as_deref() == Some(p)).unwrap_or(true) {
                epics.push(epic);
            }
        }
    }
    epics.sort_by(|a, b| a.title.cmp(&b.title));
    Ok(epics)
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

fn task_path_by_id(vault: &Path, task_id: &str) -> Result<PathBuf> {
    for entry in fs::read_dir(vault.join("tasks"))? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("md") {
            continue;
        }
        if let Ok(task) = read_task(&path) {
            if task.id == task_id {
                return Ok(path);
            }
        }
    }
    Err(VaultError::InvalidFrontmatter(format!(
        "task not found: {task_id}"
    )))
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

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTaskColumnPayload {
    pub task_id: String,
    pub column: String,
}

#[tauri::command]
fn update_task_column(
    app: AppHandle,
    payload: UpdateTaskColumnPayload,
) -> std::result::Result<Task, String> {
    (|| -> Result<Task> {
        let vault = vault_dir(&app)?;
        ensure_vault_layout(&vault)?;
        let path = task_path_by_id(&vault, &payload.task_id)?;
        let raw = fs::read_to_string(&path)?;
        let (mut fm, body): (serde_yaml::Value, String) = parse_frontmatter(&raw)?;
        if let Some(map) = fm.as_mapping_mut() {
            map.insert(
                serde_yaml::Value::String("column".to_string()),
                serde_yaml::Value::String(payload.column.clone()),
            );
            map.insert(
                serde_yaml::Value::String("updated".to_string()),
                serde_yaml::Value::String(now_epoch()),
            );
        }
        write_frontmatter(&path, &fm, &body)?;
        read_task(&path)
    })()
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn list_projects(app: AppHandle) -> std::result::Result<Vec<Project>, String> {
    (|| -> Result<Vec<Project>> {
        let vault = vault_dir(&app)?;
        ensure_vault_layout(&vault)?;
        list_projects_inner(&vault)
    })()
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn list_epics(app: AppHandle, project_id: Option<String>) -> std::result::Result<Vec<Epic>, String> {
    (|| -> Result<Vec<Epic>> {
        let vault = vault_dir(&app)?;
        ensure_vault_layout(&vault)?;
        list_epics_inner(&vault, project_id.as_deref())
    })()
    .map_err(|e| e.to_string())
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CreateProjectPayload {
    pub title: String,
    pub owner: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CreateEpicPayload {
    pub title: String,
    pub project_id: Option<String>,
    pub owner: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CreateStoryPayload {
    pub title: String,
    pub project_id: Option<String>,
    pub epic_id: Option<String>,
    pub owner: Option<String>,
    pub description: Option<String>,
    pub as_a: Option<String>,
    pub i_want: Option<String>,
    pub so_that: Option<String>,
    pub acceptance_criteria: Option<Vec<String>>,
    pub column: Option<String>,
}

#[tauri::command]
fn create_project(
    app: AppHandle,
    payload: CreateProjectPayload,
) -> std::result::Result<Project, String> {
    (|| -> Result<Project> {
        let vault = vault_dir(&app)?;
        ensure_vault_layout(&vault)?;
        let id = format!("project-{}", now_epoch());
        let fm = Project {
            id: id.clone(),
            title: payload.title,
            owner: payload.owner,
            created: Some(now_epoch()),
            updated: None,
            description: payload.description.clone(),
        };
        let body = payload.description.unwrap_or_default();
        let path = vault.join("projects").join(format!("{}.md", id));
        write_frontmatter(&path, &fm, &body)?;
        Ok(fm)
    })()
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn create_epic(app: AppHandle, payload: CreateEpicPayload) -> std::result::Result<Epic, String> {
    (|| -> Result<Epic> {
        let vault = vault_dir(&app)?;
        ensure_vault_layout(&vault)?;
        let id = format!("epic-{}", now_epoch());
        let fm = Epic {
            id: id.clone(),
            title: payload.title,
            project_id: payload.project_id,
            owner: payload.owner,
            created: Some(now_epoch()),
            updated: None,
            description: payload.description.clone(),
        };
        let body = payload.description.unwrap_or_default();
        let path = vault.join("epics").join(format!("{}.md", id));
        write_frontmatter(&path, &fm, &body)?;
        Ok(fm)
    })()
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn create_story(app: AppHandle, payload: CreateStoryPayload) -> std::result::Result<Task, String> {
    (|| -> Result<Task> {
        let vault = vault_dir(&app)?;
        ensure_vault_layout(&vault)?;
        let id = format!("story-{}", now_epoch());
        #[derive(Debug, Serialize)]
        struct StoryFm {
            id: String,
            title: String,
            board: String,
            column: String,
            #[serde(skip_serializing_if = "Option::is_none")]
            project_id: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            epic_id: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            owner: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            description: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            as_a: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            i_want: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            so_that: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            acceptance_criteria: Option<Vec<String>>,
            #[serde(skip_serializing_if = "Option::is_none")]
            created: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            updated: Option<String>,
            #[serde(default)]
            tags: Vec<String>,
        }

        let fm = StoryFm {
            id: id.clone(),
            title: payload.title,
            board: "default".to_string(),
            column: payload
                .column
                .unwrap_or_else(|| "Backlog".to_string()),
            project_id: payload.project_id,
            epic_id: payload.epic_id,
            owner: payload.owner,
            description: payload.description.clone(),
            as_a: payload.as_a,
            i_want: payload.i_want,
            so_that: payload.so_that,
            acceptance_criteria: payload.acceptance_criteria,
            created: Some(now_epoch()),
            updated: None,
            tags: vec!["story".to_string()],
        };

        let body = payload.description.unwrap_or_default();
        let path = vault.join("tasks").join(format!("{}.md", id));
        write_frontmatter(&path, &fm, &body)?;
        read_task(&path)
    })()
    .map_err(|e| e.to_string())
}
\n\n
#[tauri::command]
async fn openai_autofill_story(
    app: AppHandle,
    payload: OpenAiAutoFillPayload,
) -> std::result::Result<OpenAiAutoFillResponse, String> {
    (async move {
        let api_key = resolve_openai_key()?;
        let (model, fallback_model) = resolve_openai_model();
        let prompt = format!(
            "Generate missing story fields. Return JSON only with keys: title, asA, iWant, soThat, acceptanceCriteria (array of strings).\n\nDescription: {}\nExisting title: {}\nExisting asA: {}\nExisting iWant: {}\nExisting soThat: {}\nExisting acceptanceCriteria: {}",
            payload.description,
            payload.title.clone().unwrap_or_default(),
            payload.as_a.clone().unwrap_or_default(),
            payload.i_want.clone().unwrap_or_default(),
            payload.so_that.clone().unwrap_or_default(),
            payload
                .acceptance_criteria
                .clone()
                .unwrap_or_default()
                .join("; ")
        );

        let client = reqwest::Client::new();

        let request = |model_name: &str| {
            let body = json!({
                "model": model_name,
                "messages": [
                    {
                        "role": "system",
                        "content": "You are a product manager writing user stories. Only return JSON, no markdown. Keep answers concise. Use null for fields you cannot infer."
                    },
                    { "role": "user", "content": prompt }
                ],
                "response_format": { "type": "json_object" }
            });

            client
                .post("https://api.openai.com/v1/chat/completions")
                .bearer_auth(&api_key)
                .json(&body)
        };

        let mut response = request(&model).send().await?;

        if !response.status().is_success() {
            let text = response.text().await.unwrap_or_default();
            let status = response.status();
            let should_fallback = model != fallback_model
                && (status.as_u16() == 404
                    || text.to_lowercase().contains("model"));

            if should_fallback {
                response = request(&fallback_model).send().await?;
            } else {
                return Err(VaultError::InvalidFrontmatter(format!(
                    "OpenAI error: {text}"
                )));
            }
        }

        if !response.status().is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(VaultError::InvalidFrontmatter(format!(
                "OpenAI error: {text}"
            )));
        }

        let value: serde_json::Value = response.json().await?;
        let content = value
            .get("choices")
            .and_then(|choices| choices.get(0))
            .and_then(|choice| choice.get("message"))
            .and_then(|message| message.get("content"))
            .and_then(|content| content.as_str())
            .unwrap_or("{}");

        let parsed: OpenAiAutoFillResponse = serde_json::from_str(content)?;
        Ok(parsed)
    })()
    .await
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
            get_board_with_tasks,
            update_task_column,
            list_projects,
            list_epics,
            create_project,
            create_epic,
            create_story,
            openai_autofill_story
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
