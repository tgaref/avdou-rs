pub mod rule;
pub use rule::{Copy, Rule};

pub mod document;
pub use document::Document;

pub mod context;

pub mod shortcodes;
pub use shortcodes::{expand_shortcodes, Shortcode};

pub mod route;

use anyhow::Result;
use notify::{recommended_watcher, RecursiveMode, Watcher};

use std::fs;
use std::path::{Path};
use std::sync::{Arc, Mutex};
use tera::Tera;
use tokio::runtime::Runtime;
use warp::hyper::Body;
use warp::{
    http::{StatusCode},
    Filter, Reply,
};

pub struct Site {
    pub site_dir: String,
    pub public_dir: String,
    pub rules: Vec<Rule>,
    pub copies: Vec<Copy>,
    pub tera: Tera,
}

impl Site {
    pub fn new(site_dir: &str, public_dir: &str) -> Self {
        Self {
            site_dir: site_dir.to_string(),
            public_dir: public_dir.to_string(),
            rules: vec![],
            copies: vec![],
            tera: Tera::default(),
        }
    }

    pub fn rule(mut self, rule: Rule) -> Self {
        self.rules.push(rule);
        self
    }

    pub fn copy(mut self, copy: Copy) -> Self {
        self.copies.push(copy);
        self
    }

    pub fn load_templates(mut self, dir: &str) -> Self {
        let mut tera = Tera::default();
        let p = Path::new(&self.site_dir).join(dir).canonicalize().unwrap();

        for entry in fs::read_dir(p).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "html" {
                        let name = path.file_name().unwrap().to_string_lossy();
                        let contents = fs::read_to_string(&path).unwrap();
                        tera.add_raw_template(&name, &contents).unwrap();
                    }
                }
            }
        }
        self.tera = tera;
        self
    }

    pub fn clean(&self) -> Result<()> {
    let dir_path = Path::new(&self.public_dir);

    // Attempt to remove the directory and its contents
    if dir_path.exists() {
        fs::remove_dir_all(dir_path)?;
        println!(
            "Directory '{}' and its contents removed successfully.",
            dir_path.display()
        );
    } else {
        println!("Directory '{}' does not exist.", dir_path.display());
    }

    Ok(())
}

    
    pub fn build(&mut self) -> Result<()> {
        for rule in &self.rules {
            rule.execute(&self.site_dir, &self.public_dir, &mut self.tera)?;
        }

        for copy in &self.copies {
            copy.execute(&self.site_dir, &self.public_dir)?;
        }

        Ok(())
    }

    pub fn serve(self, port: u16) -> Result<()> {
        let site = Arc::new(Mutex::new(self));	
	
        // Watch for changes
        let site_clone = site.clone();
        let mut watcher = recommended_watcher(move |_| {
            let mut site = site_clone.lock().unwrap();
            println!("Rebuilding site...");
            let _ = site.build();
        })?;

        // Watch the public_dir
        let public_dir = {
            let site = site.lock().unwrap();
            site.public_dir.clone()
        };
	println!("Watching: {:?}", Path::new(&public_dir));
	watcher.watch(Path::new(&public_dir), RecursiveMode::Recursive)?;
	
        // Route to serve files
	let files = warp::path::full()
            .and(warp::any().map(move || site.clone()))
            .and_then(move |full_path: warp::path::FullPath, site: Arc<Mutex<Site>>| {
                async move {
                    // Clone public_dir for async block
                    let public_dir = {
                        let site = site.lock().unwrap();
                        site.public_dir.clone()
                    };

                    // Build requested relative path
                    let mut rel_path = full_path.as_str().trim_start_matches('/').to_string();
                    if rel_path.is_empty() {
                        rel_path.push_str("index.html");
                    }

                    let mut file_path = Path::new(&public_dir).join(&rel_path);

                    // If the path is a directory, serve index.html
                    if file_path.is_dir() {
                        file_path = file_path.join("index.html");
                    }

                    // Return 404 if file doesn't exist
                    if !file_path.exists() {
                        return Ok::<_, warp::Rejection>(
                            warp::reply::with_status("Not Found", StatusCode::NOT_FOUND)
                                .into_response(),
                        );
                    }

                    // Read file bytes
                    let data = match tokio::fs::read(&file_path).await {
                        Ok(d) => d,
                        Err(_) => {
                            return Ok::<_, warp::Rejection>(
                                warp::reply::with_status("Not Found", StatusCode::NOT_FOUND)
                                    .into_response(),
                            )
                        }
                    };

                    // Detect MIME type
                    let mime = mime_guess::from_path(&file_path).first_or_octet_stream();

                    // Build response
                    let resp = warp::http::Response::builder()
                        .header("Content-Type", mime.as_ref())
                        .body(Body::from(data))
                        .unwrap();

                    Ok::<_, warp::Rejection>(resp)
                }
            });

        // Fallback 404 route
        let not_found = warp::any().map(|| {
            warp::reply::with_status("Not Found", StatusCode::NOT_FOUND).into_response()
        });

        // Combine routes
        let routes = files.or(not_found);

        println!("Serving at http://localhost:{}", port);
        Runtime::new()?.block_on(warp::serve(routes).run(([127, 0, 0, 1], port)));

        Ok(())
    }    
}
