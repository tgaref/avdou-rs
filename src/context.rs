use anyhow::Result;
use serde_yaml::Value;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::document::parse_front_matter;
use crate::Document;

pub type Variables = HashMap<String, Value>;
pub type Miner = Box<dyn Fn(&Document) -> Variables>;

pub trait VarsExt {
    fn var<K: Into<String>, V: Into<serde_yaml::Value>>(self, k: K, v: V) -> Self;
}

impl VarsExt for Variables {
    fn var<K: Into<String>, V: Into<serde_yaml::Value>>(mut self, k: K, v: V) -> Self {
        self.insert(k.into(), v.into());
        self
    }
}

pub type Data = HashMap<String, Variables>;

pub struct Mine {
    pub pattern: Vec<String>,
    pub miners: Vec<Miner>,
}

impl Mine {
    pub fn new() -> Self {
        Mine {
            pattern: vec![],
            miners: vec![],
        }
    }

    pub fn pattern(mut self, pattern: &[&str]) -> Self {
        self.pattern = pattern.iter().map(|pat| pat.to_string()).collect();
        self
    }

    pub fn miner<T>(mut self, f: T) -> Self
    where
        T: Fn(&Document) -> Variables + 'static,
    {
        self.miners.push(Box::new(f));
        self
    }

    pub fn execute(&self, site_dir: &str) -> Result<Data> {
        let walker = globwalk::GlobWalkerBuilder::from_patterns(site_dir, &self.pattern)
            .follow_links(true)
            .build()?
            .filter_map(Result::ok);

        let mut data = Data::new();
        for entry in walker {
            let path = entry.path();
            if path.is_file() {
                let path_str = path.to_str().unwrap().to_string();
                // Load document
                let doc = {
                    let p = Path::new(&path_str).canonicalize().unwrap();
                    let content = fs::read_to_string(p)?;
                    let (metadata, body) = parse_front_matter(&content);
                    Document {
                        path: path_str.clone(),
                        content: body,
                        metadata,
                    }
                };

                let mut ctx = Variables::new();
                for f in &self.miners {
                    ctx.extend(f(&doc));
                }
                data.insert(path_str, ctx);
            }
        }
        Ok(data)
    }
}
