use dirs::home_dir;
use reqwest::blocking::{Client, Response};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

const STORAGE_DIR_NAME: &str = ".flux-flow";
const TODOS_FILE_NAME: &str = "todos.json";
const IDEAS_FILE_NAME: &str = "ideas.json";
const TEAM_BACKEND_DEFAULT_URL: &str = "http://157.173.124.239:25578";
const TEAM_BACKEND_URL_ENV: &str = "FLUX_FLOW_TEAM_BACKEND_URL";
const TEAM_HTTP_TIMEOUT_SECS: u64 = 15;
const MAX_TITLE_LEN: usize = 500;
const MAX_DESCRIPTION_LEN: usize = 10_000;
const MAX_CONTENT_LEN: usize = 50_000;
const MAX_TAG_LEN: usize = 100;
const MAX_TAGS_COUNT: usize = 50;
const MAX_ITEMS_COUNT: usize = 10_000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TeamRole {
    Owner,
    Admin,
    Member,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TodoItem {
    pub id: String,
    pub title: String,
    pub description: String,
    pub created_at: String,
    pub due_at: Option<String>,
    pub remind_at: Option<String>,
    pub completed: bool,
    pub reminder_fired_at: Option<String>,
    pub due_fired_at: Option<String>,
    #[serde(default = "default_priority")]
    pub priority: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub created_by_member_id: Option<String>,
    #[serde(default)]
    pub created_by_member_name: Option<String>,
    #[serde(default)]
    pub assignee_member_id: Option<String>,
    #[serde(default)]
    pub assignee_member_name: Option<String>,
}

fn default_priority() -> String {
    "medium".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdeaItem {
    pub id: String,
    pub title: String,
    pub content: String,
    pub created_at: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamSession {
    pub team_code: String,
    pub member_id: String,
    pub member_name: String,
    pub auth_token: String,
    pub role: TeamRole,
    pub owner: bool,
    pub member_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamMemberInfo {
    pub id: String,
    pub name: String,
    pub role: TeamRole,
    pub joined_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamActivityItem {
    pub id: String,
    pub created_at: String,
    pub actor_member_id: String,
    pub actor_member_name: String,
    pub action: String,
    pub details: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamContext {
    pub session: TeamSession,
    pub members: Vec<TeamMemberInfo>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateTeamRequest {
    display_name: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct JoinTeamRequest {
    code: String,
    display_name: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SaveTodosRequest {
    todos: Vec<TodoItem>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SaveIdeasRequest {
    ideas: Vec<IdeaItem>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdateRoleRequest {
    role: TeamRole,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TodosPayload {
    todos: Vec<TodoItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdeasPayload {
    ideas: Vec<IdeaItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TeamActivityPayload {
    activities: Vec<TeamActivityItem>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ErrorPayload {
    error: String,
}

fn storage_dir() -> Result<PathBuf, String> {
    let home = home_dir().ok_or_else(|| "Unable to resolve home directory".to_string())?;
    let dir = home.join(STORAGE_DIR_NAME);

    fs::create_dir_all(&dir)
        .map_err(|error| format!("Failed to create storage directory: {error}"))?;
    Ok(dir)
}

fn read_json_or_default<T>(path: &Path) -> Result<T, String>
where
    T: Default + DeserializeOwned,
{
    if !path.exists() {
        return Ok(T::default());
    }

    let raw = fs::read_to_string(path)
        .map_err(|error| format!("Failed to read {}: {error}", path.display()))?;
    if raw.trim().is_empty() {
        return Ok(T::default());
    }

    serde_json::from_str::<T>(&raw)
        .map_err(|error| format!("Failed to parse {}: {error}", path.display()))
}

fn write_json<T>(path: &Path, value: &T) -> Result<(), String>
where
    T: Serialize,
{
    let payload = serde_json::to_string_pretty(value)
        .map_err(|error| format!("Failed to serialize {}: {error}", path.display()))?;

    let mut file = File::create(path)
        .map_err(|error| format!("Failed to create {}: {error}", path.display()))?;
    file.write_all(payload.as_bytes())
        .map_err(|error| format!("Failed to write {}: {error}", path.display()))?;

    Ok(())
}

fn team_backend_base_url() -> String {
    match env::var(TEAM_BACKEND_URL_ENV) {
        Ok(value) if !value.trim().is_empty() => value.trim().trim_end_matches('/').to_string(),
        _ => TEAM_BACKEND_DEFAULT_URL.to_string(),
    }
}

fn team_endpoint(path: &str) -> String {
    format!("{}{}", team_backend_base_url(), path)
}

fn team_http_timeout() -> Duration {
    let secs = env::var("FLUX_FLOW_TEAM_HTTP_TIMEOUT")
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(TEAM_HTTP_TIMEOUT_SECS);
    Duration::from_secs(secs)
}

fn team_http_client() -> Result<Client, String> {
    Client::builder()
        .timeout(team_http_timeout())
        .build()
        .map_err(|error| format!("Failed to initialize HTTP client: {error}"))
}

fn normalize_member_name(value: Option<String>) -> Option<String> {
    match value {
        Some(raw) => {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.chars().take(40).collect())
            }
        }
        None => None,
    }
}

fn sanitize_path_segment(value: &str, label: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!("{label} is required."));
    }
    if trimmed.contains('/') || trimmed.contains('\\') || trimmed.contains("..") || trimmed.contains('\0') {
        return Err(format!("{label} contains invalid characters."));
    }
    Ok(trimmed.to_string())
}

fn validate_todo(todo: &TodoItem) -> Result<(), String> {
    if todo.title.len() > MAX_TITLE_LEN {
        return Err(format!("Todo title exceeds maximum length of {MAX_TITLE_LEN} characters."));
    }
    if todo.description.len() > MAX_DESCRIPTION_LEN {
        return Err(format!("Todo description exceeds maximum length of {MAX_DESCRIPTION_LEN} characters."));
    }
    if todo.tags.len() > MAX_TAGS_COUNT {
        return Err(format!("Too many tags (max {MAX_TAGS_COUNT})."));
    }
    for tag in &todo.tags {
        if tag.len() > MAX_TAG_LEN {
            return Err(format!("Tag exceeds maximum length of {MAX_TAG_LEN} characters."));
        }
    }
    Ok(())
}

fn validate_idea(idea: &IdeaItem) -> Result<(), String> {
    if idea.title.len() > MAX_TITLE_LEN {
        return Err(format!("Idea title exceeds maximum length of {MAX_TITLE_LEN} characters."));
    }
    if idea.content.len() > MAX_CONTENT_LEN {
        return Err(format!("Idea content exceeds maximum length of {MAX_CONTENT_LEN} characters."));
    }
    if idea.tags.len() > MAX_TAGS_COUNT {
        return Err(format!("Too many tags (max {MAX_TAGS_COUNT})."));
    }
    for tag in &idea.tags {
        if tag.len() > MAX_TAG_LEN {
            return Err(format!("Tag exceeds maximum length of {MAX_TAG_LEN} characters."));
        }
    }
    Ok(())
}

fn validate_todos(todos: &[TodoItem]) -> Result<(), String> {
    if todos.len() > MAX_ITEMS_COUNT {
        return Err(format!("Too many todos (max {MAX_ITEMS_COUNT})."));
    }
    for todo in todos {
        validate_todo(todo)?;
    }
    Ok(())
}

fn validate_ideas(ideas: &[IdeaItem]) -> Result<(), String> {
    if ideas.len() > MAX_ITEMS_COUNT {
        return Err(format!("Too many ideas (max {MAX_ITEMS_COUNT})."));
    }
    for idea in ideas {
        validate_idea(idea)?;
    }
    Ok(())
}

fn normalize_required_value(value: String, label: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Err(format!("{label} is required."))
    } else {
        Ok(trimmed.to_string())
    }
}

fn normalize_auth_token(value: String) -> Result<String, String> {
    normalize_required_value(value, "Auth token")
}

fn normalize_team_code(value: &str) -> String {
    let compact: String = value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .map(|ch| ch.to_ascii_uppercase())
        .collect();

    if compact.len() == 8 {
        format!("{}-{}", &compact[..4], &compact[4..])
    } else {
        compact
    }
}

fn parse_team_code(value: &str) -> Result<String, String> {
    let normalized = normalize_team_code(value);
    let valid = normalized.len() == 9
        && normalized.chars().nth(4) == Some('-')
        && normalized
            .chars()
            .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '-');

    if valid {
        Ok(normalized)
    } else {
        Err("Team code must be in format XXXX-XXXX.".to_string())
    }
}

fn parse_team_error(response: Response) -> String {
    let status = response.status();
    let body = response.text().unwrap_or_default();

    if let Ok(err) = serde_json::from_str::<ErrorPayload>(&body) {
        return format!("Team backend error ({status}): {}", err.error);
    }

    if !body.trim().is_empty() {
        return format!("Team backend error ({status}): {}", body.trim());
    }

    format!("Team backend error ({status})")
}

fn parse_json_response<T>(response: Response, context: &str) -> Result<T, String>
where
    T: DeserializeOwned,
{
    if !response.status().is_success() {
        return Err(parse_team_error(response));
    }

    response
        .json::<T>()
        .map_err(|error| format!("Failed to parse {context} response: {error}"))
}

fn expect_empty_response(response: Response, context: &str) -> Result<(), String> {
    if response.status().is_success() {
        Ok(())
    } else {
        Err(format!("{context}: {}", parse_team_error(response)))
    }
}

#[tauri::command]
pub fn load_todos() -> Result<Vec<TodoItem>, String> {
    let dir = storage_dir()?;
    let path = dir.join(TODOS_FILE_NAME);
    read_json_or_default(&path)
}

#[tauri::command]
pub fn save_todos(todos: Vec<TodoItem>) -> Result<(), String> {
    validate_todos(&todos)?;
    let dir = storage_dir()?;
    let path = dir.join(TODOS_FILE_NAME);
    write_json(&path, &todos)
}

#[tauri::command]
pub fn load_ideas() -> Result<Vec<IdeaItem>, String> {
    let dir = storage_dir()?;
    let path = dir.join(IDEAS_FILE_NAME);
    read_json_or_default(&path)
}

#[tauri::command]
pub fn save_ideas(ideas: Vec<IdeaItem>) -> Result<(), String> {
    validate_ideas(&ideas)?;
    let dir = storage_dir()?;
    let path = dir.join(IDEAS_FILE_NAME);
    write_json(&path, &ideas)
}

#[tauri::command]
pub fn create_team(display_name: Option<String>) -> Result<TeamSession, String> {
    let client = team_http_client()?;
    let response = client
        .post(team_endpoint("/api/team/create"))
        .json(&CreateTeamRequest {
            display_name: normalize_member_name(display_name),
        })
        .send()
        .map_err(|error| format!("Failed to reach Team backend: {error}"))?;

    parse_json_response(response, "create team")
}

#[tauri::command]
pub fn join_team(code: String, display_name: Option<String>) -> Result<TeamSession, String> {
    let normalized_code = parse_team_code(&code)?;
    let client = team_http_client()?;
    let response = client
        .post(team_endpoint("/api/team/join"))
        .json(&JoinTeamRequest {
            code: normalized_code,
            display_name: normalize_member_name(display_name),
        })
        .send()
        .map_err(|error| format!("Failed to reach Team backend: {error}"))?;

    parse_json_response(response, "join team")
}

#[tauri::command]
pub fn load_team_context(team_code: String, auth_token: String) -> Result<TeamContext, String> {
    let normalized_code = parse_team_code(&team_code)?;
    let auth_token = normalize_auth_token(auth_token)?;
    let client = team_http_client()?;
    let response = client
        .get(team_endpoint(&format!("/api/team/{normalized_code}/context")))
        .bearer_auth(auth_token)
        .send()
        .map_err(|error| format!("Failed to reach Team backend: {error}"))?;

    parse_json_response(response, "load team context")
}

#[tauri::command]
pub fn load_team_activity(team_code: String, auth_token: String) -> Result<Vec<TeamActivityItem>, String> {
    let normalized_code = parse_team_code(&team_code)?;
    let auth_token = normalize_auth_token(auth_token)?;
    let client = team_http_client()?;
    let response = client
        .get(team_endpoint(&format!("/api/team/{normalized_code}/activity")))
        .bearer_auth(auth_token)
        .send()
        .map_err(|error| format!("Failed to reach Team backend: {error}"))?;

    let payload: TeamActivityPayload = parse_json_response(response, "load team activity")?;
    Ok(payload.activities)
}

#[tauri::command]
pub fn update_team_member_role(
    team_code: String,
    auth_token: String,
    target_member_id: String,
    role: TeamRole,
) -> Result<(), String> {
    let normalized_code = parse_team_code(&team_code)?;
    let auth_token = normalize_auth_token(auth_token)?;
    let target_member_id = sanitize_path_segment(&target_member_id, "Target member id")?;
    let client = team_http_client()?;
    let response = client
        .put(team_endpoint(&format!(
            "/api/team/{normalized_code}/members/{target_member_id}/role"
        )))
        .bearer_auth(auth_token)
        .json(&UpdateRoleRequest { role })
        .send()
        .map_err(|error| format!("Failed to reach Team backend: {error}"))?;

    expect_empty_response(response, "Update member role failed")
}

#[tauri::command]
pub fn load_team_todos(team_code: String, auth_token: String) -> Result<Vec<TodoItem>, String> {
    let normalized_code = parse_team_code(&team_code)?;
    let auth_token = normalize_auth_token(auth_token)?;
    let client = team_http_client()?;
    let response = client
        .get(team_endpoint(&format!("/api/team/{normalized_code}/todos")))
        .bearer_auth(auth_token)
        .send()
        .map_err(|error| format!("Failed to reach Team backend: {error}"))?;

    let payload: TodosPayload = parse_json_response(response, "load team todos")?;
    Ok(payload.todos)
}

#[tauri::command]
pub fn save_team_todos(team_code: String, auth_token: String, todos: Vec<TodoItem>) -> Result<(), String> {
    validate_todos(&todos)?;
    let normalized_code = parse_team_code(&team_code)?;
    let auth_token = normalize_auth_token(auth_token)?;
    let client = team_http_client()?;
    let response = client
        .put(team_endpoint(&format!("/api/team/{normalized_code}/todos")))
        .bearer_auth(auth_token)
        .json(&SaveTodosRequest { todos })
        .send()
        .map_err(|error| format!("Failed to reach Team backend: {error}"))?;

    expect_empty_response(response, "Save team todos failed")
}

#[tauri::command]
pub fn load_team_ideas(team_code: String, auth_token: String) -> Result<Vec<IdeaItem>, String> {
    let normalized_code = parse_team_code(&team_code)?;
    let auth_token = normalize_auth_token(auth_token)?;
    let client = team_http_client()?;
    let response = client
        .get(team_endpoint(&format!("/api/team/{normalized_code}/ideas")))
        .bearer_auth(auth_token)
        .send()
        .map_err(|error| format!("Failed to reach Team backend: {error}"))?;

    let payload: IdeasPayload = parse_json_response(response, "load team ideas")?;
    Ok(payload.ideas)
}

#[tauri::command]
pub fn save_team_ideas(team_code: String, auth_token: String, ideas: Vec<IdeaItem>) -> Result<(), String> {
    validate_ideas(&ideas)?;
    let normalized_code = parse_team_code(&team_code)?;
    let auth_token = normalize_auth_token(auth_token)?;
    let client = team_http_client()?;
    let response = client
        .put(team_endpoint(&format!("/api/team/{normalized_code}/ideas")))
        .bearer_auth(auth_token)
        .json(&SaveIdeasRequest { ideas })
        .send()
        .map_err(|error| format!("Failed to reach Team backend: {error}"))?;

    expect_empty_response(response, "Save team ideas failed")
}
