use std::collections::HashMap;

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct Document {
    pub path: String,
    pub content: String,
    pub metadata: HashMap<String, serde_yaml::Value>,
}
