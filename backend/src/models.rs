use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TeamRole {
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TeamDatabase {
    pub teams: Vec<TeamRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamRecord {
    pub code: String,
    pub created_at: String,
    pub members: Vec<TeamMember>,
    pub todos: Vec<TodoItem>,
    pub ideas: Vec<IdeaItem>,
    #[serde(default)]
    pub activities: Vec<TeamActivityItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamMember {
    pub id: String,
    pub name: String,
    pub joined_at: String,
    #[serde(default)]
    pub auth_token: String,
    #[serde(default)]
    pub role: TeamRole,
    #[serde(default)]
    pub owner: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTeamRequest {
    pub display_name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JoinTeamRequest {
    pub code: String,
    pub display_name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveTodosRequest {
    pub todos: Vec<TodoItem>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveIdeasRequest {
    pub ideas: Vec<IdeaItem>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRoleRequest {
    pub role: TeamRole,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TodosPayload {
    pub todos: Vec<TodoItem>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdeasPayload {
    pub ideas: Vec<IdeaItem>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamContextPayload {
    pub session: TeamSession,
    pub members: Vec<TeamMemberInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityPayload {
    pub activities: Vec<TeamActivityItem>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorResponse {
    pub error: String,
}