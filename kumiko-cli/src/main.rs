use clap::Parser;
use libkumiko::config::{Gutters, KumikoConfig};
use libkumiko::process_path;
use std::path::PathBuf;

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
    };

    if let Err(e) = process_path(&args.input_path, &config) {
        eprintln!("❌ Error: {}", e);
        std::process::exit(1);
    }
}