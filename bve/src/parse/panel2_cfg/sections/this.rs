use crate::HexColorRGB;
use bve_derive::FromKVPSection;
use glam::Vec2;

#[derive(Debug, Clone, PartialEq, FromKVPSection)]
pub struct ThisSection {
    pub resolution: f32,
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
    pub daytime_image: String,
    pub nighttime_image: String,
    #[kvp(alias = "Transparent")]
    pub transparent_color: HexColorRGB,
    pub center: Vec2,
    pub origin: Vec2,
}

impl Default for ThisSection {
    fn default() -> Self {
        Self {
            resolution: 0.0,
            left: 0.0,
            right: 0.0,
            top: 0.0,
            bottom: 0.0,
            daytime_image: String::default(),
            nighttime_image: String::default(),
            transparent_color: HexColorRGB::new(0, 0, 255),
            center: Vec2::zero(),
            origin: Vec2::zero(),
        }
    }
}
