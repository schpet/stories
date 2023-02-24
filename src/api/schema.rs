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
