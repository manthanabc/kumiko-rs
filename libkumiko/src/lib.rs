pub mod config;
pub mod panel;
pub mod processing;
pub mod utils;

pub use config::KumikoConfig;
pub use panel::SerializablePanel;
pub use processing::find_panels;

use std::fs;
use std::path::Path;

pub fn process_path(
    input_path: &Path,
    config: &KumikoConfig,
) -> Result<Vec<(String, (u32, u32), Vec<SerializablePanel>)>, Box<dyn std::error::Error>> {
    let mut results = Vec::new();

    if input_path.is_dir() {
        for entry in fs::read_dir(input_path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                if let Some(extension) = path.extension() {
                    if let Some(ext_str) = extension.to_str().map(|s| s.to_lowercase()) {
                        match ext_str.as_str() {
                            "jpg" | "jpeg" | "png" => {
                                let (size, panels) = find_panels(&path, config)?;
                                results.push((
                                    path.file_name().unwrap().to_str().unwrap().to_string(),
                                    size,
                                    panels,
                                ));
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    } else if input_path.is_file() {
        let (size, panels) = find_panels(input_path, config)?;
        results.push((
            input_path.file_name().unwrap().to_str().unwrap().to_string(),
            size,
            panels,
        ));
    } else {
        return Err(From::from(format!(
            "Invalid input path: {:?}",
            input_path
        )));
    }
    Ok(results)
}
