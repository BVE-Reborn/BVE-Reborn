use crate::{
    parse::panel2_cfg::{Subject, SubjectTarget},
    HexColorRGB,
};
use bve_derive::FromKVPSection;
use glam::Vec2;

#[derive(Debug, Clone, PartialEq, FromKVPSection)]
pub struct DigitalNumberSection {
    pub subject: Subject,
    pub location: Vec2,
    pub daytime_image: String,
    pub nighttime_image: String,
    #[kvp(alias = "Transparent")]
    pub transparent_color: HexColorRGB,
    pub interval: f32,
    pub layer: i64,
}

impl Default for DigitalNumberSection {
    fn default() -> Self {
        Self {
            subject: Subject::from_target(SubjectTarget::True),
            location: Vec2::zero(),
            daytime_image: String::new(),
            nighttime_image: String::new(),
            transparent_color: HexColorRGB::new(0, 0, 255),
            interval: 0.0,
            layer: 0,
        }
    }
}
