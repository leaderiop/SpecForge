use crate::coverage::GraphExport;
use std::path::Path;

pub trait ReportWriter {
    fn write_report(&self, path: &Path, content: &str) -> std::io::Result<()>;
    fn create_dir_all(&self, path: &Path) -> std::io::Result<()>;
}

pub trait GraphReader {
    fn read_graph(&self, path: &Path) -> Option<GraphExport>;
}

pub struct RealFs;

impl ReportWriter for RealFs {
    fn write_report(&self, path: &Path, content: &str) -> std::io::Result<()> {
        std::fs::write(path, content)
    }

    fn create_dir_all(&self, path: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(path)
    }
}

impl GraphReader for RealFs {
    fn read_graph(&self, path: &Path) -> Option<GraphExport> {
        let content = std::fs::read_to_string(path).ok()?;
        serde_json::from_str(&content).ok()
    }
}
