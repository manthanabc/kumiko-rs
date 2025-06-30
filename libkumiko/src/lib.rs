pub mod config;
pub mod panel;
pub mod processing;
pub mod utils;

pub use config::KumikoConfig;
pub use panel::SerializablePanel;
pub use processing::find_panels_from_image;



pub fn find_panels_from_bytes(
    image_bytes: &[u8],
    config: &KumikoConfig,
) -> Result<((u32, u32), Vec<SerializablePanel>), Box<dyn std::error::Error>> {
    let img = image::load_from_memory(image_bytes)?;
    processing::find_panels_from_image(img, config)
}
