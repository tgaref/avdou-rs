pub mod shortcodes;
pub use shortcodes::{expand_shortcodes, Shortcode, SomeShortcode};

use anyhow::Result;
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

pub type Variables = HashMap<String, Value>;

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct Document {
    pub path: String,
    pub raw: Vec<u8>,
    pub content: String,
    pub metadata: HashMap<String, serde_yaml::Value>,
}

pub type Filter = Box<dyn Fn(Document) -> anyhow::Result<Document> + Send + Sync>;

#[derive(Eq, PartialEq, Debug)]
pub enum DocType {
    Bytes,
    Text,
}

pub struct Rule {
    pub pattern: Vec<String>,
    pub doc_type: DocType,
    pub filters: Vec<Filter>,
    pub context: Variables,
    pub template: Option<String>,
    pub route: Option<String>,
}

impl Rule {
    pub fn load_bytes(pattern: &[&str]) -> Self {
        Self {
            pattern: pattern.iter().map(|pat| pat.to_string()).collect(),
            doc_type: DocType::Bytes,
            filters: vec![],
            context: Variables::new(),
            template: None,
            route: None,
        }
    }

    pub fn load_text(pattern: &[&str]) -> Self {
        Self {
            pattern: pattern.iter().map(|pat| pat.to_string()).collect(),
            doc_type: DocType::Text,
            filters: vec![],
            context: Variables::new(),
            template: None,
            route: None,
        }
    }

    pub fn compiler(mut self, filter: Filter) -> Self {
        self.filters.push(Box::new(filter));
        self
    }

    pub fn context(mut self, ctx: Variables) -> Self {
        self.context = ctx;
        self
    }

    pub fn template(mut self, template: &str) -> Self {
        self.template = Some(template.to_string());
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
        let p = Path::new(&self.content_dir)
            .join(dir)
            .canonicalize()
            .unwrap();
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

    pub fn build(&mut self) -> Result<()> {
        for rule in &self.rules {
            let walker =
                globwalk::GlobWalkerBuilder::from_patterns(&self.content_dir, &rule.pattern)
                    .follow_links(true)
                    .build()?
                    .filter_map(Result::ok);

            for entry in walker {
                let path = entry.path();
                if path.is_file() {
                    let path_str = path.to_str().unwrap().to_string();
                    // Load document
                    let mut doc = if rule.doc_type == DocType::Bytes {
                        let p = Path::new(&path_str).canonicalize().unwrap();
                        let raw = fs::read(p)?;
                        Document {
                            path: path_str,
                            raw,
                            content: String::new(),
                            metadata: Variables::new(),
                        }
                    } else {
                        let p = Path::new(&path_str).canonicalize().unwrap();
                        let content = fs::read_to_string(p)?;
                        let (metadata, body) = parse_front_matter(&content);
                        Document {
                            path: path_str,
                            raw: vec![],
                            content: body,
                            metadata,
                        }
                    };

                    // Build context
                    let mut ctx = Context::new();
                    for (k, v) in rule.context.iter() {
                        ctx.insert(k, v);
                    }
                    for (k, v) in &doc.metadata {
                        ctx.insert(k, v);
                    }

                    // Apply template to markdown NEED TO ADD CONTEXT: ADD CONTEXT IN avdou_site THAT IS USED HERE!

                    self.tera.add_raw_template(&doc.path, &doc.content).unwrap();
                    let md = self.tera.render(&doc.path, &ctx)?;
                    doc.content = md;

                    // Apply filters
                    for f in &rule.filters {
                        doc = f(doc)?;
                    }

                    // Apply template
                    if let Some(template_name) = &rule.template {
                        ctx.insert("content", &doc.content);
                        let html = self.tera.render(template_name, &ctx)?;
                        doc.content = html;
                    }

                    // Determine output path

                    let final_path = if let Some(path) = &rule.route {
                        Path::new(&self.output_dir).join(nice_route(path, &self.content_dir, &doc))
                    } else {
                        Path::new(&self.output_dir).join(&doc.path)
                    };

                    if let Some(parent) = final_path.parent() {
                        fs::create_dir_all(parent).expect("Failed to create directories");
                    }

                    self.built_docs.push(doc.clone());
                    if !doc.raw.is_empty() {
                        fs::write(&final_path, doc.raw)?;
                    } else if !doc.content.is_empty() {
                        fs::write(&final_path, doc.content)?;
                    }
                }
            }
        }
        Ok(())
    }

    pub fn serve(self, port: u16, base: &'static str) -> Result<()> {
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
        let dir = warp::fs::dir(base);
        println!("Serving at http://localhost:{}", port);
        tokio::runtime::Runtime::new()?.block_on(warp::serve(dir).run(([127, 0, 0, 1], port)));
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

pub fn copy_file_compiler() -> Filter {
    Box::new(move |doc: Document| Ok(doc))
}

pub fn pandoc_markdown_compiler() -> Filter {
    Box::new(move |doc: Document| pandoc_markdown_to_html(doc))
}

pub fn expand_shortcodes_compiler(handlers: Vec<SomeShortcode>) -> Filter {
    Box::new(move |doc: Document| {
        let expanded = expand_shortcodes(&doc.content, &handlers);
        Ok(Document {
            content: expanded,
            ..doc
        })
    })
}

fn pandoc_markdown_to_html(doc: Document) -> Result<Document> {
    let mut pandoc = Pandoc::new();

    // enable extensions you care about
    let md_input_exts = vec![
        MarkdownExtension::Smart,            // smart quotes, dashes
        MarkdownExtension::LatexMacros,      // footnotes
        MarkdownExtension::FencedCodeBlocks, // triple backticks
        MarkdownExtension::PipeTables,       // GitHub-style tables
        MarkdownExtension::GridTables,       // grid-style tables
        MarkdownExtension::HeaderAttributes, // attributes after headers
        MarkdownExtension::AutoIdentifiers,  // generate id=... for headers
        MarkdownExtension::TexMathDollars,   // math $...$
        MarkdownExtension::RawHtml,
        MarkdownExtension::FencedDivs,
        //MarkdownExtension::MarkdownInHtmlBlocks,
        MarkdownExtension::Abbreviations,
        MarkdownExtension::NativeDivs,
    ];

    let md_output_exts = vec![MarkdownExtension::RawHtml];

    pandoc
        .set_input_format(InputFormat::Markdown, md_input_exts)
        .set_output_format(OutputFormat::Html5, md_output_exts)
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

pub fn nice_route(path: &str, base: &str, doc: &Document) -> String {
    let doc_path = Path::new(&doc.path);
    let rel = doc_path
        .parent()
        .unwrap()
        .strip_prefix(base)
        .unwrap()
        .to_str()
        .unwrap();
    let slug = doc_path.file_stem().unwrap().to_str().unwrap();
    let filename = doc_path.file_name().unwrap().to_str().unwrap();
    let p1 = path.replace("{path}", rel);
    let p2 = p1.replace("{slug}", slug);
    p2.replace("{filename}", &filename)
}
