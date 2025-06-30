use clap::Parser;
use libkumiko::config::{Gutters, KumikoConfig, ReadingDirection};
use libkumiko::process_path;

use serde::Serialize;
use std::path::PathBuf;
use std::time::Instant;

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
    #[arg(long, default_value_t = ReadingDirectionArg::Ltr)]
    reading_direction: ReadingDirectionArg,
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
struct OutputPanel {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

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
    let results = match process_path(&args.input_path, &config) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("❌ Error: {}", e);
            std::process::exit(1);
        }
    };

    let mut output_entries: Vec<OutputEntry> = Vec::new();

    for (filename, size, panels) in results {
        let output_panels: Vec<OutputPanel> = panels
            .into_iter()
            .map(|p| OutputPanel {
                x: p.x,
                y: p.y,
                width: p.width,
                height: p.height,
            })
            .collect();

        output_entries.push(OutputEntry {
            filename,
            size,
            numbering: format!("{:?}", args.reading_direction).to_lowercase(),
            gutters: (config.gutters.x.abs(), config.gutters.y.abs()), // Assuming x and y gutters are symmetric
            panels: output_panels,
            processing_time: start_time.elapsed().as_secs_f64(),
        });
    }

    match serde_json::to_string_pretty(&output_entries) {
        Ok(json) => println!("{}", json),
        Err(e) => {
            eprintln!("❌ Error serializing JSON: {}", e);
            std::process::exit(1);
        }
    }
}