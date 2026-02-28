use axum::extract::{Path, State};
use axum::http::{header, HeaderMap, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};
use std::net::SocketAddr;
use std::path::{Path as FsPath, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

const DEFAULT_BIND_ADDR: &str = "0.0.0.0:25578";
const DEFAULT_DB_PATH: &str = "backend.json";
const TEAM_CODE_CHARS: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
const MAX_ACTIVITY_ITEMS: usize = 500;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum TeamRole {
    Owner,
    Admin,
    Member,
}

impl Default for TeamRole {
    fn default() -> Self {
        Self::Member
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct TodoItem {
    id: String,
    title: String,
    description: String,
    created_at: String,
    due_at: Option<String>,
    remind_at: Option<String>,
    completed: bool,
    reminder_fired_at: Option<String>,
    due_fired_at: Option<String>,
    #[serde(default = "default_priority")]
    priority: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    created_by_member_id: Option<String>,
    #[serde(default)]
    created_by_member_name: Option<String>,
    #[serde(default)]
    assignee_member_id: Option<String>,
    #[serde(default)]
    assignee_member_name: Option<String>,
}

fn default_priority() -> String {
    "medium".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdeaItem {
    id: String,
    title: String,
    content: String,
    created_at: String,
    #[serde(default)]
    tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TeamSession {
    team_code: String,
    member_id: String,
    member_name: String,
    auth_token: String,
    role: TeamRole,
    owner: bool,
    member_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TeamMemberInfo {
    id: String,
    name: String,
    role: TeamRole,
    joined_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TeamActivityItem {
    id: String,
    created_at: String,
    actor_member_id: String,
    actor_member_name: String,
    action: String,
    details: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct TeamDatabase {
    teams: Vec<TeamRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TeamRecord {
    code: String,
    created_at: String,
    members: Vec<TeamMember>,
    todos: Vec<TodoItem>,
    ideas: Vec<IdeaItem>,
    #[serde(default)]
    activities: Vec<TeamActivityItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TeamMember {
    id: String,
    name: String,
    joined_at: String,
    #[serde(default)]
    auth_token: String,
    #[serde(default)]
    role: TeamRole,
    #[serde(default)]
    owner: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateTeamRequest {
    display_name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct JoinTeamRequest {
    code: String,
    display_name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SaveTodosRequest {
    todos: Vec<TodoItem>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SaveIdeasRequest {
    ideas: Vec<IdeaItem>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateRoleRequest {
    role: TeamRole,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TodosPayload {
    todos: Vec<TodoItem>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdeasPayload {
    ideas: Vec<IdeaItem>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TeamContextPayload {
    session: TeamSession,
    members: Vec<TeamMemberInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ActivityPayload {
    activities: Vec<TeamActivityItem>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ErrorResponse {
    error: String,
}

#[derive(Clone)]
struct AppState {
    db_path: PathBuf,
    db: Arc<Mutex<TeamDatabase>>,
}

#[derive(Debug)]
enum ApiError {
    BadRequest(String),
    Unauthorized(String),
    Forbidden(String),
    NotFound(String),
    Internal(String),
}

impl ApiError {
    fn status_code(&self) -> StatusCode {
        match self {
            ApiError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ApiError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            ApiError::Forbidden(_) => StatusCode::FORBIDDEN,
            ApiError::NotFound(_) => StatusCode::NOT_FOUND,
            ApiError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn message(&self) -> &str {
        match self {
            ApiError::BadRequest(message)
            | ApiError::Unauthorized(message)
            | ApiError::Forbidden(message)
            | ApiError::NotFound(message)
            | ApiError::Internal(message) => message,
        }
    }
}

impl Display for ApiError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message())
    }
}

impl std::error::Error for ApiError {}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let message = self.message().to_string();

        match status {
            StatusCode::BAD_REQUEST => warn!(error = %message, "Request validation failed"),
            StatusCode::UNAUTHORIZED => warn!(error = %message, "Authentication failed"),
            StatusCode::FORBIDDEN => warn!(error = %message, "Permission denied"),
            StatusCode::NOT_FOUND => warn!(error = %message, "Resource not found"),
            _ => error!(error = %message, "Unhandled backend error"),
        }

        (status, Json(ErrorResponse { error: message })).into_response()
    }
}

#[tokio::main]
async fn main() {
    init_tracing();

    let bind_addr = std::env::var("FLUX_FLOW_TEAM_BIND_ADDR").unwrap_or_else(|_| DEFAULT_BIND_ADDR.to_string());
    let db_path = std::env::var("FLUX_FLOW_TEAM_DB_PATH").unwrap_or_else(|_| DEFAULT_DB_PATH.to_string());
    let db_path = PathBuf::from(db_path);
    let mut database = match load_database(&db_path).await {
        Ok(database) => database,
        Err(error) => {
            warn!(error = %error, "Failed to load database, starting with empty state");
            TeamDatabase::default()
        }
    };
    normalize_database(&mut database);

    let state = AppState {
        db_path,
        db: Arc::new(Mutex::new(database)),
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_headers(Any)
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::OPTIONS]);

    let app = Router::new()
        .route("/health", get(health))
        .route("/api/team/create", post(create_team))
        .route("/api/team/join", post(join_team))
        .route("/api/team/{team_code}/context", get(load_team_context))
        .route("/api/team/{team_code}/activity", get(load_team_activity))
        .route("/api/team/{team_code}/todos", get(load_todos).put(save_todos))
        .route("/api/team/{team_code}/ideas", get(load_ideas).put(save_ideas))
        .route(
            "/api/team/{team_code}/members/{target_member_id}/role",
            put(update_member_role),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(state)
        .layer(cors);

    let addr: SocketAddr = match bind_addr.parse() {
        Ok(address) => address,
        Err(error) => {
            warn!(bind_addr = %bind_addr, error = %error, "Invalid bind address, using fallback");
            DEFAULT_BIND_ADDR.parse().expect("default bind address must be valid")
        }
    };
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind socket");
    info!(address = %addr, "Flux Flow backend started");
    axum::serve(listener, app).await.expect("server failed");
}

async fn health() -> &'static str {
    "ok"
}

fn init_tracing() {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("flux_flow_backend=info,tower_http=info"));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer().compact().with_target(false))
        .init();
}

async fn create_team(
    State(state): State<AppState>,
    Json(request): Json<CreateTeamRequest>,
) -> Result<Json<TeamSession>, ApiError> {
    let owner_name = normalize_member_name(request.display_name, "Owner");
    let member_id = next_member_id();
    let joined_at = unix_millis_string();

    let mut db = state.db.lock().await;
    let team_code = generate_team_code(&db)?;
    let owner_member = TeamMember {
        id: member_id.clone(),
        name: owner_name.clone(),
        joined_at: joined_at.clone(),
        auth_token: next_auth_token(),
        role: TeamRole::Owner,
        owner: true,
    };

    let mut team = TeamRecord {
        code: team_code.clone(),
        created_at: joined_at.clone(),
        members: vec![owner_member.clone()],
        todos: Vec::new(),
        ideas: Vec::new(),
        activities: Vec::new(),
    };
    push_activity(
        &mut team,
        &owner_member,
        "team_created",
        format!("Created team {}", team_code),
    );
    db.teams.push(team);

    let snapshot = db.clone();
    drop(db);
    persist_database(&state.db_path, &snapshot).await?;
    info!(team_code = %team_code, owner = %owner_name, "Created team");

    Ok(Json(build_session(
        &team_code,
        &owner_member,
        1,
    )))
}

async fn join_team(
    State(state): State<AppState>,
    Json(request): Json<JoinTeamRequest>,
) -> Result<Json<TeamSession>, ApiError> {
    let code = parse_team_code(&request.code)?;
    let member_name = normalize_member_name(request.display_name, "Member");
    let member_id = next_member_id();
    let joined_at = unix_millis_string();

    let mut db = state.db.lock().await;
    let team = db
        .teams
        .iter_mut()
        .find(|team| team.code == code)
        .ok_or_else(|| ApiError::NotFound("Team not found. Check your code and try again.".to_string()))?;

    let member = TeamMember {
        id: member_id.clone(),
        name: member_name.clone(),
        joined_at,
        auth_token: next_auth_token(),
        role: TeamRole::Member,
        owner: false,
    };
    team.members.push(member.clone());
    let member_count = team.members.len();
    push_activity(
        team,
        &member,
        "member_joined",
        format!("{} joined the team", member.name),
    );

    let snapshot = db.clone();
    drop(db);
    persist_database(&state.db_path, &snapshot).await?;
    info!(team_code = %code, member = %member_name, member_count, "Joined team");

    Ok(Json(build_session(&code, &member, member_count)))
}

async fn load_team_context(
    Path(team_code): Path<String>,
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<Json<TeamContextPayload>, ApiError> {
    let code = parse_team_code(&team_code)?;
    let db = state.db.lock().await;
    let team = db
        .teams
        .iter()
        .find(|team| team.code == code)
        .ok_or_else(|| ApiError::NotFound("Team not found. Check your code and try again.".to_string()))?;
    let member = authenticate_member(team, &headers)?;

    let mut members = team
        .members
        .iter()
        .map(to_member_info)
        .collect::<Vec<_>>();
    members.sort_by(|left, right| {
        role_rank(&left.role)
            .cmp(&role_rank(&right.role))
            .then(left.joined_at.cmp(&right.joined_at))
    });

    Ok(Json(TeamContextPayload {
        session: build_session(&code, member, team.members.len()),
        members,
    }))
}

async fn load_team_activity(
    Path(team_code): Path<String>,
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<Json<ActivityPayload>, ApiError> {
    let code = parse_team_code(&team_code)?;
    let db = state.db.lock().await;
    let team = db
        .teams
        .iter()
        .find(|team| team.code == code)
        .ok_or_else(|| ApiError::NotFound("Team not found. Check your code and try again.".to_string()))?;
    let member = authenticate_member(team, &headers)?;

    if member.role != TeamRole::Owner {
        return Err(ApiError::Forbidden(
            "Only team owner can view the activity feed.".to_string(),
        ));
    }

    let mut activities = team.activities.clone();
    activities.reverse();
    Ok(Json(ActivityPayload { activities }))
}

async fn update_member_role(
    Path((team_code, target_member_id)): Path<(String, String)>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<UpdateRoleRequest>,
) -> Result<StatusCode, ApiError> {
    let code = parse_team_code(&team_code)?;
    let target_id = normalize_member_id(&target_member_id)?;

    if request.role == TeamRole::Owner {
        return Err(ApiError::BadRequest(
            "Setting Owner role via this action is not allowed.".to_string(),
        ));
    }

    let mut db = state.db.lock().await;
    let team = db
        .teams
        .iter_mut()
        .find(|team| team.code == code)
        .ok_or_else(|| ApiError::NotFound("Team not found. Check your code and try again.".to_string()))?;

    let actor = authenticate_member(team, &headers)?.clone();

    if actor.role != TeamRole::Owner {
        return Err(ApiError::Forbidden(
            "Only owner can change member roles.".to_string(),
        ));
    }

    if target_id == actor.id {
        return Err(ApiError::BadRequest(
            "Owner cannot change their own role.".to_string(),
        ));
    }

    let target = team
        .members
        .iter_mut()
        .find(|member| member.id == target_id)
        .ok_or_else(|| ApiError::NotFound("Target member not found.".to_string()))?;

    if target.role == TeamRole::Owner {
        return Err(ApiError::BadRequest(
            "Owner role cannot be changed.".to_string(),
        ));
    }

    target.role = request.role.clone();
    target.owner = false;
    let details = format!("{} role changed to {}", target.name, role_label(&target.role));
    push_activity(team, &actor, "role_updated", details);

    let snapshot = db.clone();
    drop(db);
    persist_database(&state.db_path, &snapshot).await?;
    info!(
        team_code = %code,
        actor_member_id = %actor.id,
        target_member_id = %target_id,
        role = %role_label(&request.role),
        "Updated member role"
    );

    Ok(StatusCode::NO_CONTENT)
}

async fn load_todos(
    Path(team_code): Path<String>,
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<Json<TodosPayload>, ApiError> {
    let code = parse_team_code(&team_code)?;
    let db = state.db.lock().await;
    let team = db
        .teams
        .iter()
        .find(|team| team.code == code)
        .ok_or_else(|| ApiError::NotFound("Team not found. Check your code and try again.".to_string()))?;
    authenticate_member(team, &headers)?;

    Ok(Json(TodosPayload {
        todos: team.todos.clone(),
    }))
}

async fn save_todos(
    Path(team_code): Path<String>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<SaveTodosRequest>,
) -> Result<StatusCode, ApiError> {
    let code = parse_team_code(&team_code)?;
    let mut db = state.db.lock().await;
    let team = db
        .teams
        .iter_mut()
        .find(|team| team.code == code)
        .ok_or_else(|| ApiError::NotFound("Team not found. Check your code and try again.".to_string()))?;

    let actor = authenticate_member(team, &headers)?.clone();
    let member_names = member_name_map(team);
    let old_todos = team
        .todos
        .iter()
        .map(|todo| (todo.id.clone(), todo.clone()))
        .collect::<HashMap<_, _>>();

    let mut normalized_todos = Vec::with_capacity(payload.todos.len());
    let mut seen_ids = HashSet::new();
    let mut change_events = Vec::new();

    for mut todo in payload.todos {
        let todo_id = todo.id.clone();
        if !seen_ids.insert(todo_id.clone()) {
            return Err(ApiError::BadRequest("Duplicate TODO id in payload.".to_string()));
        }

        if let Some(old) = old_todos.get(&todo_id) {
            if old != &todo && !can_manage_task(&actor, old) {
                return Err(ApiError::Forbidden(format!(
                    "You cannot edit task \"{}\".",
                    old.title
                )));
            }

            todo.created_by_member_id = old.created_by_member_id.clone();
            todo.created_by_member_name = old.created_by_member_name.clone();
            normalize_assignee(&mut todo, &member_names);

            if old != &todo {
                change_events.push((
                    "todo_updated".to_string(),
                    format!("Updated \"{}\"", todo.title),
                ));
            }
        } else {
            todo.created_by_member_id = Some(actor.id.clone());
            todo.created_by_member_name = Some(actor.name.clone());
            normalize_assignee(&mut todo, &member_names);
            change_events.push((
                "todo_created".to_string(),
                format!("Created \"{}\"", todo.title),
            ));
        }

        normalized_todos.push(todo);
    }

    for old in &team.todos {
        if seen_ids.contains(&old.id) {
            continue;
        }
        if !can_manage_task(&actor, old) {
            return Err(ApiError::Forbidden(format!(
                "You cannot delete task \"{}\".",
                old.title
            )));
        }
        change_events.push((
            "todo_deleted".to_string(),
            format!("Deleted \"{}\"", old.title),
        ));
    }

    team.todos = normalized_todos;
    for (action, details) in change_events {
        push_activity(team, &actor, &action, details);
    }

    let snapshot = db.clone();
    drop(db);
    persist_database(&state.db_path, &snapshot).await?;
    info!(team_code = %code, actor_member_id = %actor.id, "Saved team todos");

    Ok(StatusCode::NO_CONTENT)
}

async fn load_ideas(
    Path(team_code): Path<String>,
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<Json<IdeasPayload>, ApiError> {
    let code = parse_team_code(&team_code)?;
    let db = state.db.lock().await;
    let team = db
        .teams
        .iter()
        .find(|team| team.code == code)
        .ok_or_else(|| ApiError::NotFound("Team not found. Check your code and try again.".to_string()))?;
    authenticate_member(team, &headers)?;

    Ok(Json(IdeasPayload {
        ideas: team.ideas.clone(),
    }))
}

async fn save_ideas(
    Path(team_code): Path<String>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<SaveIdeasRequest>,
) -> Result<StatusCode, ApiError> {
    let code = parse_team_code(&team_code)?;
    let mut db = state.db.lock().await;
    let team = db
        .teams
        .iter_mut()
        .find(|team| team.code == code)
        .ok_or_else(|| ApiError::NotFound("Team not found. Check your code and try again.".to_string()))?;
    let actor = authenticate_member(team, &headers)?.clone();

    team.ideas = payload.ideas;
    let snapshot = db.clone();
    drop(db);
    persist_database(&state.db_path, &snapshot).await?;
    info!(team_code = %code, actor_member_id = %actor.id, "Saved team ideas");

    Ok(StatusCode::NO_CONTENT)
}

fn build_session(team_code: &str, member: &TeamMember, member_count: usize) -> TeamSession {
    TeamSession {
        team_code: team_code.to_string(),
        member_id: member.id.clone(),
        member_name: member.name.clone(),
        auth_token: member.auth_token.clone(),
        role: member.role.clone(),
        owner: member.role == TeamRole::Owner,
        member_count,
    }
}

fn to_member_info(member: &TeamMember) -> TeamMemberInfo {
    TeamMemberInfo {
        id: member.id.clone(),
        name: member.name.clone(),
        role: member.role.clone(),
        joined_at: member.joined_at.clone(),
    }
}

fn role_rank(role: &TeamRole) -> usize {
    match role {
        TeamRole::Owner => 0,
        TeamRole::Admin => 1,
        TeamRole::Member => 2,
    }
}

fn role_label(role: &TeamRole) -> &'static str {
    match role {
        TeamRole::Owner => "owner",
        TeamRole::Admin => "admin",
        TeamRole::Member => "member",
    }
}

fn extract_bearer_token(headers: &HeaderMap) -> Result<String, ApiError> {
    let raw = headers
        .get(header::AUTHORIZATION)
        .ok_or_else(|| ApiError::Unauthorized("Missing Authorization header.".to_string()))?
        .to_str()
        .map_err(|_| ApiError::Unauthorized("Invalid Authorization header.".to_string()))?;

    let token = raw
        .strip_prefix("Bearer ")
        .ok_or_else(|| ApiError::Unauthorized("Authorization must be Bearer token.".to_string()))?
        .trim();

    if token.is_empty() {
        return Err(ApiError::Unauthorized(
            "Empty Bearer token is not allowed.".to_string(),
        ));
    }

    Ok(token.to_string())
}

fn authenticate_member<'a>(team: &'a TeamRecord, headers: &HeaderMap) -> Result<&'a TeamMember, ApiError> {
    let token = extract_bearer_token(headers)?;

    team.members
        .iter()
        .find(|member| member.auth_token == token)
        .ok_or_else(|| ApiError::Unauthorized("Invalid or expired team session token.".to_string()))
}

fn can_manage_task(actor: &TeamMember, todo: &TodoItem) -> bool {
    match actor.role {
        TeamRole::Owner | TeamRole::Admin => true,
        TeamRole::Member => {
            todo.created_by_member_id.as_deref() == Some(actor.id.as_str())
                || todo.assignee_member_id.as_deref() == Some(actor.id.as_str())
        }
    }
}

fn member_name_map(team: &TeamRecord) -> HashMap<String, String> {
    team.members
        .iter()
        .map(|member| (member.id.clone(), member.name.clone()))
        .collect()
}

fn normalize_assignee(todo: &mut TodoItem, member_names: &HashMap<String, String>) {
    match todo.assignee_member_id.clone() {
        Some(assignee_id) => {
            if let Some(name) = member_names.get(&assignee_id) {
                todo.assignee_member_id = Some(assignee_id);
                todo.assignee_member_name = Some(name.clone());
            } else {
                todo.assignee_member_id = None;
                todo.assignee_member_name = None;
            }
        }
        None => {
            todo.assignee_member_name = None;
        }
    }
}

fn push_activity(team: &mut TeamRecord, actor: &TeamMember, action: &str, details: String) {
    team.activities.push(TeamActivityItem {
        id: next_activity_id(),
        created_at: unix_millis_string(),
        actor_member_id: actor.id.clone(),
        actor_member_name: actor.name.clone(),
        action: action.to_string(),
        details,
    });

    if team.activities.len() > MAX_ACTIVITY_ITEMS {
        let overflow = team.activities.len() - MAX_ACTIVITY_ITEMS;
        team.activities.drain(0..overflow);
    }
}

fn normalize_database(db: &mut TeamDatabase) {
    for team in &mut db.teams {
        normalize_team(team);
    }
}

fn normalize_team(team: &mut TeamRecord) {
    if team.members.is_empty() {
        return;
    }

    let mut has_owner = false;
    for member in &mut team.members {
        if member.auth_token.trim().is_empty() {
            member.auth_token = next_auth_token();
        }
        if member.owner {
            member.role = TeamRole::Owner;
        }
        if member.role == TeamRole::Owner {
            member.owner = true;
            has_owner = true;
        } else {
            member.owner = false;
        }
    }

    if !has_owner {
        if let Some(first_member) = team.members.first_mut() {
            first_member.role = TeamRole::Owner;
            first_member.owner = true;
        }
    }

    let default_creator = team
        .members
        .iter()
        .find(|member| member.role == TeamRole::Owner)
        .cloned()
        .or_else(|| team.members.first().cloned());
    let member_names = member_name_map(team);

    for todo in &mut team.todos {
        if todo.created_by_member_id.is_none() {
            if let Some(member) = &default_creator {
                todo.created_by_member_id = Some(member.id.clone());
                todo.created_by_member_name = Some(member.name.clone());
            }
        } else if let Some(created_by_id) = &todo.created_by_member_id {
            if let Some(name) = member_names.get(created_by_id) {
                todo.created_by_member_name = Some(name.clone());
            }
        }
        normalize_assignee(todo, &member_names);
    }
}

fn normalize_member_name(raw: Option<String>, fallback: &str) -> String {
    let value = raw.unwrap_or_default();
    let trimmed = value.trim();

    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.chars().take(40).collect()
    }
}

fn normalize_member_id(value: &str) -> Result<String, ApiError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Err(ApiError::BadRequest("Member id is required.".to_string()))
    } else {
        Ok(trimmed.to_string())
    }
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

fn parse_team_code(value: &str) -> Result<String, ApiError> {
    let normalized = normalize_team_code(value);
    let valid = normalized.len() == 9
        && normalized.chars().nth(4) == Some('-')
        && normalized
            .chars()
            .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '-');

    if valid {
        Ok(normalized)
    } else {
        Err(ApiError::BadRequest(
            "Team code must be in format XXXX-XXXX.".to_string(),
        ))
    }
}

fn generate_team_code(db: &TeamDatabase) -> Result<String, ApiError> {
    let mut rng = rand::thread_rng();

    for _ in 0..500 {
        let mut raw = String::with_capacity(8);
        for _ in 0..8 {
            let idx = rng.gen_range(0..TEAM_CODE_CHARS.len());
            raw.push(TEAM_CODE_CHARS[idx] as char);
        }

        let code = format!("{}-{}", &raw[..4], &raw[4..]);
        let exists = db.teams.iter().any(|team| team.code == code);
        if !exists {
            return Ok(code);
        }
    }

    Err(ApiError::Internal(
        "Failed to generate unique team code.".to_string(),
    ))
}

fn next_member_id() -> String {
    let now = unix_millis_string();
    let mut rng = rand::thread_rng();
    let suffix: u32 = rng.gen_range(100_000..999_999);
    format!("member-{now}-{suffix}")
}

fn next_auth_token() -> String {
    let mut rng = rand::thread_rng();
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut token = String::with_capacity(48);
    for _ in 0..48 {
        token.push(CHARS[rng.gen_range(0..CHARS.len())] as char);
    }
    token
}

fn next_activity_id() -> String {
    let now = unix_millis_string();
    let mut rng = rand::thread_rng();
    let suffix: u32 = rng.gen_range(100_000..999_999);
    format!("activity-{now}-{suffix}")
}

fn unix_millis_string() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .to_string()
}

async fn load_database(path: &FsPath) -> Result<TeamDatabase, ApiError> {
    if !path.exists() {
        return Ok(TeamDatabase::default());
    }

    let raw = tokio::fs::read_to_string(path)
        .await
        .map_err(|err| ApiError::Internal(format!("Failed to read database: {err}")))?;

    if raw.trim().is_empty() {
        return Ok(TeamDatabase::default());
    }

    serde_json::from_str::<TeamDatabase>(&raw)
        .map_err(|err| ApiError::Internal(format!("Failed to parse database: {err}")))
}

async fn persist_database(path: &FsPath, db: &TeamDatabase) -> Result<(), ApiError> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|err| ApiError::Internal(format!("Failed to create DB folder: {err}")))?;
        }
    }

    let payload = serde_json::to_vec_pretty(db)
        .map_err(|err| ApiError::Internal(format!("Failed to serialize database: {err}")))?;
    tokio::fs::write(path, payload)
        .await
        .map_err(|err| ApiError::Internal(format!("Failed to write database: {err}")))?;

    Ok(())
}
