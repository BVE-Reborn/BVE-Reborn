pub use as_u32::*;
use cgmath::{Vector1, Vector2, Vector3, Vector4};
pub use hex::*;

mod as_u32;
mod hex;

/// R color: Unsigned 8-bit integer per channel
pub type ColorU8R = Vector1<u8>;
/// RG color: Unsigned 8-bit integer per channel
pub type ColorU8RG = Vector2<u8>;
/// RGB color: Unsigned 8-bit integer per channel
pub type ColorU8RGB = Vector3<u8>;
/// RGBA color: Unsigned 8-bit integer per channel
pub type ColorU8RGBA = Vector4<u8>;

/// R color: Unsigned 16-bit integer per channel
pub type ColorU16R = Vector1<u16>;
/// RG color: Unsigned 16-bit integer per channel
pub type ColorU16RG = Vector2<u16>;
/// RGB color: Unsigned 16-bit integer per channel
pub type ColorU16RGB = Vector3<u16>;
/// RGBA color: Unsigned 16-bit integer per channel
pub type ColorU16RGBA = Vector4<u16>;

/// R color: 32-bit float per channel
pub type ColorF32R = Vector1<f32>;
/// RG color: 32-bit float per channel
pub type ColorF32RG = Vector2<f32>;
/// RGB color: 32-bit float per channel
pub type ColorF32RGB = Vector3<f32>;
/// RGBA color: 32-bit float per channel
pub type ColorF32RGBA = Vector4<f32>;

#[repr(C)]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct UVec2 {
    pub x: u32,
    pub y: u32,
}

impl UVec2 {
    #[must_use]
    pub const fn new(x: u32, y: u32) -> Self {
        Self { x, y }
    }

    #[must_use]
    pub const fn into_array(self) -> [u32; 2] {
        [self.x, self.y]
    }

    #[must_use]
    pub const fn from_array([x, y]: [u32; 2]) -> Self {
        Self { x, y }
    }

    pub fn map(self, mut f: impl FnMut(u32) -> u32) -> Self {
        Self {
            x: f(self.x),
            y: f(self.y),
        }
    }
}
