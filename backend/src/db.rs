use std::path::Path;

use crate::error::ApiError;
use crate::helpers::*;
use crate::models::*;

pub async fn load_database(path: &Path) -> Result<TeamDatabase, ApiError> {
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

pub async fn persist_database(path: &Path, db: &TeamDatabase) -> Result<(), ApiError> {
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

pub fn normalize_database(db: &mut TeamDatabase) {
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