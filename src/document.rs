use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct Document {
    pub path: String,
    pub content: String,
    pub metadata: HashMap<String, serde_yaml::Value>,
}

pub fn parse_front_matter(raw: &str) -> (HashMap<String, serde_yaml::Value>, String) {
    if let Some(striped) = raw.strip_prefix("---") {
        if let Some(end) = striped.find("---") {
            let meta_str = &striped[..end];
            let body = &striped[end + 3..];
            let metadata: HashMap<String, serde_yaml::Value> =
                serde_yaml::from_str(meta_str).unwrap_or_default();
            return (metadata, body.trim().to_string());
        }
    }
    (HashMap::new(), raw.to_string())
}

pub fn load_document(splitmeta: bool, path_str: String) -> Document {
    let p = Path::new(&path_str).canonicalize().unwrap();
    let content =
        fs::read_to_string(p).unwrap_or_else(|_| panic!("Failed to read file {}", path_str));
    if splitmeta {
        let (metadata, body) = parse_front_matter(&content);
        Document {
            path: path_str,
            content: body,
            metadata,
        }
    } else {
        Document {
            path: path_str,
            content,
            metadata: HashMap::new(),
        }
    }
}
