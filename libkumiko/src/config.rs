#[derive(Debug, Clone, Copy)]
pub struct Gutters {
    pub x: i32,
    pub y: i32,
    pub r: i32,
    pub b: i32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ReadingDirection {
    Ltr,
    Rtl,
}

#[derive(Debug, Clone, Copy)]
pub struct KumikoConfig {
    pub gutters: Gutters,
    pub small_panel_ratio: f64,
    pub rdp_epsilon: f64,
    pub reading_direction: ReadingDirection,
}

impl Default for KumikoConfig {
    fn default() -> Self {
        Self {
            gutters: Gutters {
                x: -2,
                y: -2,
                r: 2,
                b: 2,
            },
            small_panel_ratio: 1.0 / 15.0,
            rdp_epsilon: 0.01,
            reading_direction: ReadingDirection::Ltr,
        }
    }
}