use anyhow::Result;
use globber::Pattern;
use notify::{recommended_watcher, RecursiveMode, Watcher};
use pandoc::{
    InputFormat, InputKind, MarkdownExtension, OutputFormat, OutputKind, Pandoc, PandocOption,
    PandocOutput,
};
use pulldown_cmark::{html, Parser};
use serde_yaml::Value;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tera::{Context, Tera};
use walkdir::WalkDir;

pub type Variables = HashMap<String, Value>;

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct Document {
    pub path: String,
    pub content: String,
    pub metadata: HashMap<String, serde_yaml::Value>,
}

pub type Filter = fn(Document) -> Result<Document>;

pub struct Rule {
    pub pattern: String,
    pub filters: Vec<Filter>,
    pub template: (Option<String>, HashMap<String, serde_yaml::Value>),
    pub route: Option<String>,
}

impl Rule {
    pub fn new(pattern: &str) -> Self {
        Self {
            pattern: pattern.to_string(),
            filters: vec![],
            template: (None, Variables::new()),
            route: None,
        }
    }

    pub fn compiler(mut self, filter: Filter) -> Self {
        self.filters.push(filter);
        self
    }

    pub fn template(mut self, template: &str, context: Variables) -> Self {
        self.template = (Some(template.to_string()), context);
        self
    }

    pub fn route(mut self, router: &str) -> Self {
        self.route = Some(router.to_string());
        self
    }
}

pub struct Site {
    pub content_dir: String,
    pub output_dir: String,
    pub rules: Vec<Rule>,
    pub tera: Tera,
    pub built_docs: Vec<Document>,
}

impl Site {
    pub fn new(content_dir: &str, output_dir: &str) -> Result<Self> {
        Ok(Self {
            content_dir: content_dir.to_string(),
            output_dir: output_dir.to_string(),
            rules: vec![],
            tera: Tera::default(),
            built_docs: vec![],
        })
    }

    pub fn rule(mut self, rule: Rule) -> Self {
        self.rules.push(rule);
        self
    }

    pub fn load_templates(mut self, dir: &str) -> Self {
        let mut tera = Tera::default();

        for entry in fs::read_dir(dir).unwrap() {
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

    pub fn build(&mut self) -> Result<()> {
        for rule in &self.rules {
            let pattern = Pattern::new(&rule.pattern).unwrap();
            for entry in WalkDir::new(&self.content_dir)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if entry.file_type().is_file() {
                    let path = entry.path().canonicalize().unwrap();
                    let path_str = path.to_str().unwrap().to_string();
                    let rel = entry.path().strip_prefix(&self.content_dir).unwrap();
                    let rel_str = rel.to_str().unwrap().to_string();
                    if pattern.matches(&rel_str) {
                        // Load document
                        let content = fs::read_to_string(&path_str)?;
                        let (metadata, body) = parse_front_matter(&content);
                        let mut doc = Document {
                            path: path_str,
                            content: body,
                            metadata,
                        };

                        // Apply filters
                        for f in &rule.filters {
                            doc = f(doc)?;
                        }

                        // Apply template
                        if let Some(template_name) = &rule.template.0 {
                            let mut ctx = Context::new();
                            ctx.insert("content", &doc.content);
                            for (k, v) in &rule.template.1 {
                                ctx.insert(k, v);
                            }
                            for (k, v) in &doc.metadata {
                                ctx.insert(k, v);
                            }
                            let html = self.tera.render(template_name, &ctx)?;
                            doc.content = html;
                        }

                        // Determine output path
                        let out = if let Some(route) = &rule.route {
                            let p = Path::new(&doc.path).to_path_buf();
                            let slug = p.file_stem().unwrap();
                            let filename = p.file_name().unwrap();
                            let tmp = route.replace("{slug}", slug.to_str().unwrap());
                            tmp.replace("{filename}", filename.to_str().unwrap())
                        } else {
                            doc.path.clone()
                        };
                        let out_path = Path::new(&out);
                        let final_path = Path::new(&self.output_dir).join(out_path);
                        if let Some(parent) = final_path.parent() {
                            fs::create_dir_all(parent).expect("Failed to create directories");
                        }

                        self.built_docs.push(doc.clone());
                        fs::write(&final_path, doc.content)?;
                    }
                }
            }
        }
        Ok(())
    }

    pub fn serve(self) -> Result<()> {
        let site = Arc::new(Mutex::new(self));
        let site_clone = site.clone();

        // Watch for changes in content/
        let mut watcher = recommended_watcher(move |_| {
            let mut site = site_clone.lock().unwrap();
            println!("Rebuilding site...");
            let _ = site.build();
        })?;
        watcher.watch(
            Path::new(&site.lock().unwrap().content_dir),
            RecursiveMode::Recursive,
        )?;

        // Serve public/ via warp
        let dir = warp::fs::dir("public/");
        println!("Serving at http://localhost:8000");
        tokio::runtime::Runtime::new()?.block_on(warp::serve(dir).run(([127, 0, 0, 1], 8000)));
        Ok(())
    }
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

pub fn markdown_to_html(doc: Document) -> Result<Document> {
    let parser = Parser::new(&doc.content);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    Ok(Document {
        content: html_output,
        ..doc
    })
}

pub fn copy_file_compiler(doc: Document) -> Result<Document> {
    Ok(doc)
}

pub fn pandoc_markdown_compiler(doc: Document) -> Result<Document> {
    let mut pandoc = Pandoc::new();

    // enable extensions you care about
    let md_exts = vec![
        MarkdownExtension::Smart,            // smart quotes, dashes
        MarkdownExtension::LatexMacros,      // footnotes
        MarkdownExtension::FencedCodeBlocks, // triple backticks
        MarkdownExtension::PipeTables,       // GitHub-style tables
        MarkdownExtension::GridTables,       // grid-style tables
        MarkdownExtension::HeaderAttributes, // attributes after headers
        MarkdownExtension::AutoIdentifiers,  // generate id=... for headers
        MarkdownExtension::TexMathDollars,   // math $...$
    ];

    pandoc
        .set_input_format(InputFormat::Markdown, md_exts)
        .set_output_format(OutputFormat::Html, vec![])
        .set_input(InputKind::Pipe(doc.content.clone()))
        .set_output(OutputKind::Pipe);

    pandoc.add_option(PandocOption::MathJax(None));

    match pandoc.execute()? {
        PandocOutput::ToBuffer(html) => Ok(Document {
            content: html,
            ..doc
        }),
        PandocOutput::ToFile(path) => Err(anyhow::anyhow!(
            "Unexpected Pandoc output written to file: {:?}",
            path
        )),
        PandocOutput::ToBufferRaw(_) => todo!(),
    }
}
