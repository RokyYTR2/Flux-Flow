use axum::extract::{ConnectInfo, Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use tracing::info;

use crate::db::persist_team;
use crate::error::ApiError;
use crate::helpers::*;
use crate::models::*;
use crate::state::{AppState, RATE_LIMIT_MAX_CREATE_ATTEMPTS, RATE_LIMIT_MAX_JOIN_ATTEMPTS};

pub async fn health() -> &'static str {
    "ok"
}

pub async fn create_team(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
    Json(request): Json<CreateTeamRequest>,
) -> Result<Json<TeamSession>, ApiError> {
    state.create_limiter.lock().await.check(addr.ip(), RATE_LIMIT_MAX_CREATE_ATTEMPTS)?;
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
    let team_snapshot = team.clone();
    db.teams.push(team);
    drop(db);
    persist_team(&*state.conn.lock().await, &team_snapshot)?;
    info!(team_code = %team_code, owner = %owner_name, "Created team");

    Ok(Json(build_session(&team_code, &owner_member, 1)))
}

pub async fn join_team(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
    Json(request): Json<JoinTeamRequest>,
) -> Result<Json<TeamSession>, ApiError> {
    state.join_limiter.lock().await.check(addr.ip(), RATE_LIMIT_MAX_JOIN_ATTEMPTS)?;
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

    let team_snapshot = team.clone();
    drop(db);
    persist_team(&*state.conn.lock().await, &team_snapshot)?;
    info!(team_code = %code, member = %member_name, member_count, "Joined team");

    Ok(Json(build_session(&code, &member, member_count)))
}

pub async fn load_team_context(
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

pub async fn load_team_activity(
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

pub async fn update_member_role(
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

    let team_snapshot = team.clone();
    drop(db);
    persist_team(&*state.conn.lock().await, &team_snapshot)?;
    info!(
        team_code = %code,
        actor_member_id = %actor.id,
        target_member_id = %target_id,
        role = %role_label(&request.role),
        "Updated member role"
    );

    Ok(StatusCode::NO_CONTENT)
}

pub async fn load_todos(
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

pub async fn save_todos(
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

    let team_snapshot = team.clone();
    drop(db);
    persist_team(&*state.conn.lock().await, &team_snapshot)?;
    info!(team_code = %code, actor_member_id = %actor.id, "Saved team todos");

    Ok(StatusCode::NO_CONTENT)
}

pub async fn load_ideas(
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

pub async fn save_ideas(
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
    let team_snapshot = team.clone();
    drop(db);
    persist_team(&*state.conn.lock().await, &team_snapshot)?;
    info!(team_code = %code, actor_member_id = %actor.id, "Saved team ideas");

    Ok(StatusCode::NO_CONTENT)
}