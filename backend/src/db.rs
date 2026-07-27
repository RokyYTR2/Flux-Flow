use std::path::Path;

use rusqlite::Connection;

use crate::error::ApiError;
use crate::helpers::*;
use crate::models::*;

pub fn open_database(path: &Path) -> Result<Connection, ApiError> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|err| ApiError::Internal(format!("Failed to create DB folder: {err}")))?;
        }
    }

    let conn = Connection::open(path)
        .map_err(|err| ApiError::Internal(format!("Failed to open database: {err}")))?;
    conn.pragma_update(None, "journal_mode", "WAL")
        .map_err(|err| ApiError::Internal(format!("Failed to enable WAL: {err}")))?;
    conn.pragma_update(None, "synchronous", "NORMAL")
        .map_err(|err| ApiError::Internal(format!("Failed to set synchronous: {err}")))?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS teams (code TEXT PRIMARY KEY, data TEXT NOT NULL)",
        [],
    )
    .map_err(|err| ApiError::Internal(format!("Failed to create schema: {err}")))?;

    Ok(conn)
}

pub fn load_database(conn: &Connection) -> Result<TeamDatabase, ApiError> {
    let mut stmt = conn
        .prepare("SELECT data FROM teams")
        .map_err(|err| ApiError::Internal(format!("Failed to query teams: {err}")))?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|err| ApiError::Internal(format!("Failed to read teams: {err}")))?;

    let mut teams = Vec::new();
    for row in rows {
        let raw = row.map_err(|err| ApiError::Internal(format!("Failed to read team row: {err}")))?;
        let team = serde_json::from_str::<TeamRecord>(&raw)
            .map_err(|err| ApiError::Internal(format!("Failed to parse team record: {err}")))?;
        teams.push(team);
    }

    Ok(TeamDatabase { teams })
}

pub fn persist_team(conn: &Connection, team: &TeamRecord) -> Result<(), ApiError> {
    let payload = serde_json::to_string(team)
        .map_err(|err| ApiError::Internal(format!("Failed to serialize team: {err}")))?;
    conn.execute(
        "INSERT INTO teams (code, data) VALUES (?1, ?2)
         ON CONFLICT(code) DO UPDATE SET data = excluded.data",
        rusqlite::params![team.code, payload],
    )
    .map_err(|err| ApiError::Internal(format!("Failed to persist team: {err}")))?;

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
