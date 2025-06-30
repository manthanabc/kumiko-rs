use clap::Parser;
use libkumiko::config::{Gutters, KumikoConfig, ReadingDirection};
use libkumiko::find_panels_from_bytes;

use serde::Serialize;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

mod html_report;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the image or directory to process
    #[arg(required = true)]
    input_path: PathBuf,

    /// Gutter x size
    #[arg(long, default_value_t = -2)]
    gutter_x: i32,

    /// Gutter y size
    #[arg(long, default_value_t = -2)]
    gutter_y: i32,

    /// Gutter r size
    #[arg(long, default_value_t = 2)]
    gutter_r: i32,

    /// Gutter b size
    #[arg(long, default_value_t = 2)]
    gutter_b: i32,

    /// Small panel ratio
    #[arg(long, default_value_t = 1.0 / 15.0)]
    small_panel_ratio: f64,

    /// RDP epsilon
    #[arg(long, default_value_t = 0.01)]
    rdp_epsilon: f64,

    /// Reading direction (ltr or rtl)
    #[arg(long, default_value = "ltr")]
    reading_direction: ReadingDirectionArg,

    /// Generate HTML report
    #[arg(long, short = 'b')] // Using -b for --html as per user's request
    html: bool,

    /// Open HTML report in browser (requires --html)
    #[arg(long, requires = "html")]
    open_browser: bool,
}

#[derive(clap::ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
enum ReadingDirectionArg {
    Ltr,
    Rtl,
}

impl std::fmt::Display for ReadingDirectionArg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<ReadingDirectionArg> for ReadingDirection {
    fn from(val: ReadingDirectionArg) -> Self {
        match val {
            ReadingDirectionArg::Ltr => ReadingDirection::Ltr,
            ReadingDirectionArg::Rtl => ReadingDirection::Rtl,
        }
    }
}

#[derive(Serialize, Debug)]
struct OutputPanel(i32, i32, i32, i32);

#[derive(Serialize, Debug)]
struct OutputEntry {
    filename: String,
    size: (u32, u32),
    numbering: String,
    gutters: (i32, i32),
    panels: Vec<OutputPanel>,
    processing_time: f64,
}

fn main() {
    let args = Args::parse();

    let config = KumikoConfig {
        gutters: Gutters {
            x: args.gutter_x,
            y: args.gutter_y,
            r: args.gutter_r,
            b: args.gutter_b,
        },
        small_panel_ratio: args.small_panel_ratio,
        rdp_epsilon: args.rdp_epsilon,
        reading_direction: args.reading_direction.into(),
    };

    let start_time = Instant::now();

    let image_bytes = match fs::read(&args.input_path) {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("❌ Error reading image file: {}", e);
            std::process::exit(1);
        }
    };

    let (size, panels) = match libkumiko::find_panels_from_bytes(&image_bytes, &config) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("❌ Error processing image: {}", e);
            std::process::exit(1);
        }
    };

    let output_panels: Vec<OutputPanel> = panels
        .into_iter()
        .map(|p| OutputPanel(p.x, p.y, p.width, p.height))
        .collect();

    let output_entry = OutputEntry {
        filename: args.input_path.file_name().unwrap().to_string_lossy().to_string(),
        size,
        numbering: format!("{:?}", args.reading_direction).to_lowercase(),
        gutters: (config.gutters.x.abs(), config.gutters.y.abs()), // Assuming x and y gutters are symmetric
        panels: output_panels,
        processing_time: start_time.elapsed().as_secs_f64(),
    };

    let output_entries = vec![output_entry];

    if args.html {
        let output_dir = PathBuf::from("kumiko_report");
        if let Err(e) = fs::create_dir_all(&output_dir) {
            eprintln!("❌ Error creating output directory: {}", e);
            std::process::exit(1);
        }

        // Copy the input image to the report directory
        let image_filename = args.input_path.file_name().unwrap().to_str().unwrap();
        let dest_image_path = output_dir.join(image_filename);
        if let Err(e) = fs::copy(&args.input_path, &dest_image_path) {
            eprintln!("❌ Error copying input image to report directory: {}", e);
            std::process::exit(1);
        }

        let html_file_path = output_dir.join("report.html");
        let mut html_content = String::new();

        html_content.push_str(&html_report::header("Kumiko Panel Detection Report", "./"));

        let json_data =
            serde_json::to_value(&output_entries).unwrap_or_else(|_| serde_json::json!([]));
        html_content.push_str(&html_report::reader(&json_data, "./"));

        html_content.push_str(&html_report::footer());

        if let Err(e) = fs::write(&html_file_path, html_content) {
            eprintln!("❌ Error writing HTML report: {}", e);
            std::process::exit(1);
        }

        // Copy assets
        let assets_dir = PathBuf::from("kumiko-cli/assets");
        let js_files = ["jquery-3.2.1.min.js", "reader.js", "style.css"];

        for file_name in &js_files {
            let src_path = assets_dir.join(file_name);
            let dest_path = output_dir.join(file_name);
            if src_path.exists() {
                if let Err(e) = fs::copy(&src_path, &dest_path) {
                    eprintln!("❌ Error copying {}: {}", file_name, e);
                    // Don't exit, just warn
                }
            } else {
                eprintln!("⚠️ Warning: Asset file not found: {}", src_path.display());
            }
        }

        println!("HTML report generated at: {}", html_file_path.display());

        if args.open_browser {
            if let Err(e) = opener::open(&html_file_path) {
                eprintln!("❌ Error opening browser: {}", e);
            }
        }
    } else {
        match serde_json::to_string_pretty(&output_entries) {
            Ok(json) => println!("{}", json),
            Err(e) => {
                eprintln!("❌ Error serializing JSON: {}", e);
                std::process::exit(1);
            }
        }
    }
}
