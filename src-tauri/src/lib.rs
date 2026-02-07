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
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("openai error: {0}")]
    OpenAi(#[from] reqwest::Error),
    #[error("invalid data: {0}")]
    InvalidData(String),
    #[error("board not found: {0}")]
    BoardNotFound(String),
    #[error("task not found: {0}")]
    TaskNotFound(String),
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
    pub project_id: Option<String>,
    #[serde(default)]
    pub epic_id: Option<String>,
    #[serde(default)]
    pub owner: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub as_a: Option<String>,
    #[serde(default)]
    pub i_want: Option<String>,
    #[serde(default)]
    pub so_that: Option<String>,
    #[serde(default)]
    pub acceptance_criteria: Option<Vec<String>>,
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
struct Db {
    version: u32,
    boards: Vec<Board>,
    tasks: Vec<Task>,
    projects: Vec<Project>,
    epics: Vec<Epic>,
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

fn db_path(app: &AppHandle) -> Result<PathBuf> {
    let base = app
        .path()
        .app_data_dir()
        .map_err(|e| VaultError::InvalidData(format!("failed to get app_data_dir: {e}")))?;
    Ok(base.join("pm-db.json"))
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
    let fallback =
        std::env::var("OPENAI_MODEL_FALLBACK").unwrap_or_else(|_| "gpt-4o-mini".to_string());
    (primary, fallback)
}

fn now_epoch() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

fn default_db() -> Db {
    Db {
        version: 1,
        boards: vec![Board {
            id: "default".to_string(),
            title: "Default Board".to_string(),
            columns: vec![
                "Inbox".to_string(),
                "Backlog".to_string(),
                "Ready".to_string(),
                "In Progress".to_string(),
                "Review".to_string(),
                "Done".to_string(),
            ],
        }],
        tasks: vec![],
        projects: vec![],
        epics: vec![],
    }
}

fn ensure_db(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    if !path.exists() {
        let db = default_db();
        let raw = serde_json::to_string_pretty(&db)?;
        fs::write(path, raw)?;
    }
    Ok(())
}

fn load_db(path: &Path) -> Result<Db> {
    ensure_db(path)?;
    let raw = fs::read_to_string(path)?;
    let db: Db = serde_json::from_str(&raw)?;
    Ok(db)
}

fn save_db(path: &Path, db: &Db) -> Result<()> {
    let raw = serde_json::to_string_pretty(db)?;
    fs::write(path, raw)?;
    Ok(())
}

fn list_boards_inner(db: &Db) -> Vec<Board> {
    let mut boards = db.boards.clone();
    boards.sort_by(|a, b| a.title.cmp(&b.title));
    boards
}

fn list_tasks_inner(db: &Db, board_id: Option<&str>) -> Vec<Task> {
    let mut tasks = db
        .tasks
        .iter()
        .filter(|t| board_id.map(|b| b == t.board).unwrap_or(true))
        .cloned()
        .collect::<Vec<_>>();
    tasks.sort_by(|a, b| a.title.cmp(&b.title));
    tasks
}

fn list_projects_inner(db: &Db) -> Vec<Project> {
    let mut projects = db.projects.clone();
    projects.sort_by(|a, b| a.title.cmp(&b.title));
    projects
}

fn list_epics_inner(db: &Db, project_id: Option<&str>) -> Vec<Epic> {
    let mut epics = db
        .epics
        .iter()
        .filter(|e| project_id.map(|p| e.project_id.as_deref() == Some(p)).unwrap_or(true))
        .cloned()
        .collect::<Vec<_>>();
    epics.sort_by(|a, b| a.title.cmp(&b.title));
    epics
}

fn board_with_tasks_inner(db: &Db, board_id: &str) -> Result<BoardWithTasks> {
    let board = db
        .boards
        .iter()
        .find(|b| b.id == board_id)
        .cloned()
        .ok_or_else(|| VaultError::BoardNotFound(board_id.to_string()))?;

    let mut by_col: HashMap<String, Vec<Task>> = HashMap::new();
    for t in db.tasks.iter().filter(|t| t.board == board_id) {
        by_col.entry(t.column.clone()).or_default().push(t.clone());
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
        let path = db_path(&app)?;
        ensure_db(&path)?;
        Ok(VaultInfo {
            path: path.to_string_lossy().to_string(),
        })
    })()
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn list_boards(app: AppHandle) -> std::result::Result<Vec<Board>, String> {
    (|| -> Result<Vec<Board>> {
        let path = db_path(&app)?;
        let db = load_db(&path)?;
        Ok(list_boards_inner(&db))
    })()
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn list_tasks(app: AppHandle, board_id: Option<String>) -> std::result::Result<Vec<Task>, String> {
    (|| -> Result<Vec<Task>> {
        let path = db_path(&app)?;
        let db = load_db(&path)?;
        Ok(list_tasks_inner(&db, board_id.as_deref()))
    })()
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_board_with_tasks(
    app: AppHandle,
    board_id: String,
) -> std::result::Result<BoardWithTasks, String> {
    (|| -> Result<BoardWithTasks> {
        let path = db_path(&app)?;
        let db = load_db(&path)?;
        board_with_tasks_inner(&db, &board_id)
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
        let path = db_path(&app)?;
        let mut db = load_db(&path)?;

        let task = db
            .tasks
            .iter_mut()
            .find(|t| t.id == payload.task_id)
            .ok_or_else(|| VaultError::TaskNotFound(payload.task_id.clone()))?;

        task.column = payload.column.clone();
        task.updated = Some(now_epoch());

        save_db(&path, &db)?;
        Ok(task.clone())
    })()
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn list_projects(app: AppHandle) -> std::result::Result<Vec<Project>, String> {
    (|| -> Result<Vec<Project>> {
        let path = db_path(&app)?;
        let db = load_db(&path)?;
        Ok(list_projects_inner(&db))
    })()
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn list_epics(app: AppHandle, project_id: Option<String>) -> std::result::Result<Vec<Epic>, String> {
    (|| -> Result<Vec<Epic>> {
        let path = db_path(&app)?;
        let db = load_db(&path)?;
        Ok(list_epics_inner(&db, project_id.as_deref()))
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
        let path = db_path(&app)?;
        let mut db = load_db(&path)?;

        let id = format!("project-{}", now_epoch());
        let fm = Project {
            id: id.clone(),
            title: payload.title,
            owner: payload.owner,
            created: Some(now_epoch()),
            updated: None,
            description: payload.description.clone(),
        };

        db.projects.push(fm.clone());
        save_db(&path, &db)?;
        Ok(fm)
    })()
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn create_epic(app: AppHandle, payload: CreateEpicPayload) -> std::result::Result<Epic, String> {
    (|| -> Result<Epic> {
        let path = db_path(&app)?;
        let mut db = load_db(&path)?;

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

        db.epics.push(fm.clone());
        save_db(&path, &db)?;
        Ok(fm)
    })()
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn create_story(app: AppHandle, payload: CreateStoryPayload) -> std::result::Result<Task, String> {
    (|| -> Result<Task> {
        let path = db_path(&app)?;
        let mut db = load_db(&path)?;

        let id = format!("story-{}", now_epoch());
        let description = payload.description.clone().unwrap_or_default();
        let fm = Task {
            id: id.clone(),
            title: payload.title,
            board: "default".to_string(),
            column: payload
                .column
                .unwrap_or_else(|| "Backlog".to_string()),
            tags: vec!["story".to_string()],
            due: None,
            created: Some(now_epoch()),
            updated: None,
            project_id: payload.project_id,
            epic_id: payload.epic_id,
            owner: payload.owner,
            description: if description.is_empty() {
                None
            } else {
                Some(description.clone())
            },
            as_a: payload.as_a,
            i_want: payload.i_want,
            so_that: payload.so_that,
            acceptance_criteria: payload.acceptance_criteria,
            body: description,
        };

        db.tasks.push(fm.clone());
        save_db(&path, &db)?;
        Ok(fm)
    })()
    .map_err(|e| e.to_string())
}

#[tauri::command]
async fn openai_autofill_story(
    _app: AppHandle,
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
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            let should_fallback = model != fallback_model
                && (status.as_u16() == 404 || text.to_lowercase().contains("model"));

            if should_fallback {
                response = request(&fallback_model).send().await?;
            } else {
                return Err(VaultError::InvalidData(format!(
                    "OpenAI error: {text}"
                )));
            }
        }

        if !response.status().is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(VaultError::InvalidData(format!(
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

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path() -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!("pm-db-test-{}.json", now_epoch()));
        path
    }

    #[test]
    fn creates_default_db() {
        let path = temp_path();
        ensure_db(&path).unwrap();
        let db = load_db(&path).unwrap();
        assert_eq!(db.boards.len(), 1);
        assert!(db.tasks.is_empty());
        let _ = fs::remove_file(path);
    }
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
