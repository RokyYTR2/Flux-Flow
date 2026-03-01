use axum::http::{header, HeaderMap};
use rand::Rng;
use std::collections::HashMap;

use crate::error::ApiError;
use crate::models::*;

pub const TEAM_CODE_CHARS: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
pub const MAX_ACTIVITY_ITEMS: usize = 500;

pub fn build_session(team_code: &str, member: &TeamMember, member_count: usize) -> TeamSession {
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

pub fn to_member_info(member: &TeamMember) -> TeamMemberInfo {
    TeamMemberInfo {
        id: member.id.clone(),
        name: member.name.clone(),
        role: member.role.clone(),
        joined_at: member.joined_at.clone(),
    }
}

pub fn role_rank(role: &TeamRole) -> usize {
    match role {
        TeamRole::Owner => 0,
        TeamRole::Admin => 1,
        TeamRole::Member => 2,
    }
}

pub fn role_label(role: &TeamRole) -> &'static str {
    match role {
        TeamRole::Owner => "owner",
        TeamRole::Admin => "admin",
        TeamRole::Member => "member",
    }
}

pub fn extract_bearer_token(headers: &HeaderMap) -> Result<String, ApiError> {
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

pub fn authenticate_member<'a>(team: &'a TeamRecord, headers: &HeaderMap) -> Result<&'a TeamMember, ApiError> {
    let token = extract_bearer_token(headers)?;

    team.members
        .iter()
        .find(|member| member.auth_token == token)
        .ok_or_else(|| ApiError::Unauthorized("Invalid or expired team session token.".to_string()))
}

pub fn can_manage_task(actor: &TeamMember, todo: &TodoItem) -> bool {
    match actor.role {
        TeamRole::Owner | TeamRole::Admin => true,
        TeamRole::Member => {
            todo.created_by_member_id.as_deref() == Some(actor.id.as_str())
                || todo.assignee_member_id.as_deref() == Some(actor.id.as_str())
        }
    }
}

pub fn member_name_map(team: &TeamRecord) -> HashMap<String, String> {
    team.members
        .iter()
        .map(|member| (member.id.clone(), member.name.clone()))
        .collect()
}

pub fn normalize_assignee(todo: &mut TodoItem, member_names: &HashMap<String, String>) {
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

pub fn push_activity(team: &mut TeamRecord, actor: &TeamMember, action: &str, details: String) {
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

pub fn normalize_member_name(raw: Option<String>, fallback: &str) -> String {
    let value = raw.unwrap_or_default();
    let trimmed = value.trim();

    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.chars().take(40).collect()
    }
}

pub fn normalize_member_id(value: &str) -> Result<String, ApiError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Err(ApiError::BadRequest("Member id is required.".to_string()))
    } else {
        Ok(trimmed.to_string())
    }
}

pub fn normalize_team_code(value: &str) -> String {
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

pub fn parse_team_code(value: &str) -> Result<String, ApiError> {
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

pub fn generate_team_code(db: &TeamDatabase) -> Result<String, ApiError> {
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

pub fn next_member_id() -> String {
    let now = unix_millis_string();
    let mut rng = rand::thread_rng();
    let suffix: u32 = rng.gen_range(100_000..999_999);
    format!("member-{now}-{suffix}")
}

pub fn next_auth_token() -> String {
    let mut rng = rand::thread_rng();
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut token = String::with_capacity(48);
    for _ in 0..48 {
        token.push(CHARS[rng.gen_range(0..CHARS.len())] as char);
    }
    token
}

pub fn next_activity_id() -> String {
    let now = unix_millis_string();
    let mut rng = rand::thread_rng();
    let suffix: u32 = rng.gen_range(100_000..999_999);
    format!("activity-{now}-{suffix}")
}

pub fn unix_millis_string() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .to_string()
}