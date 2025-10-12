use super::context::Variables;
use super::document::Document;
use super::route::{id_route, Route};
use super::shortcodes::{expand_shortcodes, Shortcode};

use anyhow::Result;
use pandoc::{
    InputFormat, InputKind, MarkdownExtension, OutputFormat, OutputKind, Pandoc, PandocOption,
    PandocOutput,
};
use std::collections::HashMap;
use std::fs::{self, Permissions};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use tera::{Context, Tera};

pub type Filter = Box<dyn Fn(Document) -> anyhow::Result<Document> + Send + Sync>;

pub struct Rule {
    pub pattern: Vec<String>,
    pub filters: Vec<Filter>,
    pub context: Variables,
    pub template: Option<String>,
    pub route: Route,
}

impl Rule {
    pub fn load(pattern: &[&str]) -> Self {
        Self {
            pattern: pattern.iter().map(|pat| pat.to_string()).collect(),
            filters: vec![],
            context: Variables::new(),
            template: None,
            route: Box::new(id_route),
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

    pub fn route(
        mut self,
        router: impl Fn(&Path, &str, &str) -> PathBuf + Send + Sync + 'static,
    ) -> Self {
        self.route = Box::new(router);
        self
    }

    pub fn execute(&self, site_dir: &str, public_dir: &str, tera: &mut Tera) -> Result<()> {
        let walker = globwalk::GlobWalkerBuilder::from_patterns(site_dir, &self.pattern)
            .follow_links(true)
            .build()?
            .filter_map(Result::ok);

        for entry in walker {
            let path = entry.path();
            if path.is_file() {
                let path_str = path.to_str().unwrap().to_string();
                // Load document
                let mut doc = {
                    let p = Path::new(&path_str).canonicalize().unwrap();
                    let content = fs::read_to_string(p)?;
                    let (metadata, body) = parse_front_matter(&content);
                    Document {
                        path: path_str,
                        content: body,
                        metadata,
                    }
                };

                // Build context
                let mut ctx = Context::new();
                for (k, v) in self.context.iter() {
                    ctx.insert(k, v);
                }
                for (k, v) in &doc.metadata {
                    ctx.insert(k, v);
                }

                // Apply template to markdown NEED TO ADD CONTEXT: ADD CONTEXT IN avdou_site THAT IS USED HERE!

                tera.add_raw_template(&doc.path, &doc.content).unwrap();
                let md = tera.render(&doc.path, &ctx)?;
                doc.content = md;

                // Apply filters
                for f in &self.filters {
                    doc = f(doc)?;
                }

                // Apply template
                if let Some(template_name) = &self.template {
                    ctx.insert("content", &doc.content);
                    let html = tera.render(template_name, &ctx)?;
                    doc.content = html;
                }

                // Determine output path

                let f = &self.route;
                let final_path = f(path, site_dir, public_dir);
                if let Some(parent) = final_path.parent() {
                    fs::create_dir_all(parent).expect("Failed to create directories");
                    fs::set_permissions(parent, Permissions::from_mode(0o755))?;
                }

                fs::write(&final_path, doc.content)?;
            }
        }
        Ok(())
    }
}

pub struct Copy {
    pub pattern: Vec<String>,
    pub route: Route,
}

impl Copy {
    pub fn source(pattern: &[&str]) -> Self {
        Self {
            pattern: pattern.iter().map(|pat| pat.to_string()).collect(),
            route: Box::new(id_route),
        }
    }

    pub fn route(
        mut self,
        router: impl Fn(&Path, &str, &str) -> PathBuf + Send + Sync + 'static,
    ) -> Self {
        self.route = Box::new(router);
        self
    }

    pub fn execute(&self, site_dir: &str, public_dir: &str) -> Result<()> {
        let walker = globwalk::GlobWalkerBuilder::from_patterns(site_dir, &self.pattern)
            .follow_links(true)
            .build()?
            .filter_map(Result::ok);

        for entry in walker {
            let path = entry.path();
            if path.is_file() {
                let f = &self.route;
                let final_path = f(path, site_dir, public_dir);

                if let Some(parent) = final_path.parent() {
                    fs::create_dir_all(parent).expect("Failed to create directories");
                    fs::set_permissions(parent, Permissions::from_mode(0o755))?;
                }
                fs::copy(path, &final_path)?;
            }
        }
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

pub fn pandoc_markdown_compiler() -> Filter {
    Box::new(move |doc: Document| pandoc_markdown_to_html(doc))
}

pub fn expand_shortcodes_compiler(handlers: Vec<Shortcode>) -> Filter {
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
