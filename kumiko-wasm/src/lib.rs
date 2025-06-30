use wasm_bindgen::prelude::*;
use libkumiko::{find_panels_from_bytes, SerializablePanel};
use serde::{Serialize, Deserialize};

#[wasm_bindgen]
#[derive(Serialize, Deserialize)]
pub struct WasmSerializablePanel {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl From<SerializablePanel> for WasmSerializablePanel {
    fn from(panel: SerializablePanel) -> Self {
        WasmSerializablePanel {
            x: panel.x,
            y: panel.y,
            width: panel.width,
            height: panel.height,
        }
    }
}

#[wasm_bindgen]
pub fn find_panels(
    image_bytes: &[u8],
    rdp_epsilon: f64,
    small_panel_ratio: f64,
    reading_direction: String,
    gutter_x: i32,
    gutter_y: i32,
    gutter_r: i32,
    gutter_b: i32,
) -> JsValue {
    let reading_direction = match reading_direction.as_str() {
        "ltr" => libkumiko::config::ReadingDirection::Ltr,
        "rtl" => libkumiko::config::ReadingDirection::Rtl,
        _ => libkumiko::config::ReadingDirection::Ltr, // Default
    };
    let kumiko_config = libkumiko::config::KumikoConfig {
        rdp_epsilon,
        small_panel_ratio,
        reading_direction,
        gutters: libkumiko::config::Gutters {
            x: gutter_x,
            y: gutter_y,
            r: gutter_r,
            b: gutter_b,
        },
    };
    match find_panels_from_bytes(image_bytes, &kumiko_config) {
        Ok((img_size, panels)) => {
            let wasm_panels: Vec<WasmSerializablePanel> = panels.into_iter().map(Into::into).collect();
            serde_json::to_string(&(img_size, wasm_panels))
                .map(|s| JsValue::from_str(&s))
                .unwrap_or_else(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
        }
        Err(e) => JsValue::from_str(&format!("Error: {}", e)),
    }
}
