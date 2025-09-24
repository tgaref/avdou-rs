use std::path::{Path, PathBuf};

pub type Route = Box<dyn Fn(&Path, &str, &str) -> PathBuf + Send + Sync + 'static>;

pub fn id_route(path: &Path, site_dir: &str, public_dir: &str) -> PathBuf {
    let rel = path.strip_prefix(site_dir).unwrap();
    Path::new(&site_dir)
        .join(public_dir)
        .join(rel)
        .to_path_buf()
}

pub fn nice_route(path: &Path, site_dir: &str, public_dir: &str) -> PathBuf {
    let slug = path.file_stem().unwrap();
    let rel_base = path.parent().unwrap().strip_prefix(site_dir).unwrap();

    Path::new(&site_dir)
        .join(public_dir)
        .join(rel_base)
        .join(slug)
        .join("index.html")
        .to_path_buf()
}

pub fn set_extension(
    ext: &'static str,
) -> impl Fn(&Path, &str, &str) -> PathBuf + Send + Sync + 'static {
    move |path: &Path, site_dir: &str, public_dir: &str| {
        let rel = path.strip_prefix(site_dir).unwrap();
        Path::new(&site_dir)
            .join(public_dir)
            .join(rel)
            .with_extension(&ext)
    }
}
