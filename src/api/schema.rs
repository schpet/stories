use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct Story {
    pub name: String,
    pub id: u32,
    pub current_state: StoryState,
    pub story_type: StoryType,
    pub url: String,
    #[serde(default)]
    pub estimate: Option<u32>,
    pub labels: Vec<Label>,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum StoryType {
    #[serde(rename = "bug")]
    Bug,
    #[serde(rename = "feature")]
    Feature,
    #[serde(rename = "chore")]
    Chore,
    #[serde(rename = "release")]
    Release,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum StoryState {
    #[serde(rename = "accepted")]
    Accepted,
    #[serde(rename = "delivered")]
    Delivered,
    #[serde(rename = "finished")]
    Finished,
    #[serde(rename = "started")]
    Started,
    #[serde(rename = "rejected")]
    Rejected,
    #[serde(rename = "planned")]
    Planned,
    #[serde(rename = "unstarted")]
    Unstarted,
    #[serde(rename = "unscheduled")]
    Unscheduled,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct StoryDetail {
    pub name: String,
    pub id: u32,
    pub current_state: StoryState,
    pub story_type: StoryType,
    pub url: String,
    #[serde(default)]
    pub estimate: Option<u32>,
    pub labels: Vec<Label>,
    pub description: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Label {
    pub id: u64,
    pub project_id: u64,
    pub kind: String,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
}

// https://www.pivotaltracker.com/help/api/rest/v5#activity_resource

#[derive(Deserialize, Debug)]
pub struct Activity {
    pub kind: String,
    pub message: String,
    pub highlight: String,
    pub primary_resources: Vec<EntityReference>,
    pub project: EntityReference,
    pub occurred_at: String,
}

#[derive(Deserialize, Debug)]
pub struct EntityReference {
    pub kind: String,
    pub id: u64,
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Me {
    pub id: u64,
    pub name: String,
    pub initials: String,
    pub username: String,
    pub email: String,
}

/// example error:
///
/// {
///   "code": "unfound_resource",
///   "kind": "error",
///   "error": "The object you tried to access could not be found.  It may have been removed by another user, you may be using the ID of another object type, or you may be trying to access a sub-resource at the wrong point in a tree."
/// }
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiError {
    #[serde(rename = "code")]
    pub code: String,

    #[serde(rename = "kind")]
    pub kind: String,

    #[serde(rename = "error")]
    pub error: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MaybeStoryDetail {
    StoryDetail(StoryDetail),
    ApiError(ApiError)
}
