pub mod config;
pub mod panel;
pub mod processing;
pub mod utils;

use std::fs;
use std::path::Path;

use config::KumikoConfig;
use processing::find_panels;

pub fn process_path(
    input_path: &Path,
    config: &KumikoConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    if input_path.is_dir() {
        for entry in fs::read_dir(input_path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                if let Some(extension) = path.extension() {
                    if let Some(ext_str) = extension.to_str().map(|s| s.to_lowercase()) {
                        match ext_str.as_str() {
                            "jpg" | "jpeg" | "png" => {
                                let panels = find_panels(&path, config)?;
                                let output_dir = Path::new("output_panels");
                                fs::create_dir_all(&output_dir)?;
                                let output_file_name =
                                    path.file_stem().unwrap().to_str().unwrap().to_owned()
                                        + ".json";
                                let output_path = output_dir.join(output_file_name);

                                let json_output = serde_json::to_string_pretty(&panels)?;
                                fs::write(&output_path, json_output)?;
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    } else if input_path.is_file() {
        let panels = find_panels(input_path, config)?;
        let output_dir = Path::new("output_panels");
        fs::create_dir_all(&output_dir)?;
        let output_file_name =
            input_path.file_stem().unwrap().to_str().unwrap().to_owned() + ".json";
        let output_path = output_dir.join(output_file_name);

        let json_output = serde_json::to_string_pretty(&panels)?;
        fs::write(&output_path, json_output)?;
    } else {
        return Err(From::from(format!(
            "Invalid input path: {:?}",
            input_path
        )));
    }
    Ok(())
}