// Example code that deserializes and serializes the model.
// extern crate serde;
// #[macro_use]
// extern crate serde_derive;
// extern crate serde_json;
//
// use generated_module::Activity;
//
// fn main() {
//     let json = r#"{"answer": 42}"#;
//     let model: Activity = serde_json::from_str(&json).unwrap();
// }

use serde::Deserialize;

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
