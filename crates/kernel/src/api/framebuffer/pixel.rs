use crate::api::color::Color;
use core::hash::Hash;
use zerocopy::FromBytes;

pub trait Pixel: Copy + Hash + FromBytes {
    fn composite_buffer(src: &[Self], dest: &mut [Self]);

    fn blend(self, other: Self) -> Self;

    fn weight_blend(origin: Self, other: Self, blend: f32) -> Self;
}

#[derive(Hash, Debug, Clone, Copy, FromBytes)]
pub struct RGBPixel {
    pub blue: u8,
    pub green: u8,
    pub red: u8,
    _channel: u8,
}

#[derive(Hash, Debug, Clone, Copy, FromBytes)]
pub struct AlphaPixel {
    pub blue: u8,
    pub green: u8,
    pub red: u8,
    pub alpha: u8,
}

impl Pixel for RGBPixel {
    #[inline]
    fn composite_buffer(src: &[Self], dest: &mut [Self]) {
        dest.copy_from_slice(src)
    }

    #[inline]
    fn blend(self, _other: Self) -> Self {
        self
    }

    fn weight_blend(origin: Self, other: Self, blend: f32) -> Self {
        let blend = if blend < 0f32 {
            0f32
        } else if blend > 1f32 {
            1f32
        } else {
            blend
        };

        let new_red = ((origin.red as f32) * blend + (other.red as f32) * (1f32 - blend)) as u8;
        let new_green =
            ((origin.green as f32) * blend + (other.green as f32) * (1f32 - blend)) as u8;
        let new_blue = ((origin.blue as f32) * blend + (other.blue as f32) * (1f32 - blend)) as u8;

        RGBPixel {
            _channel: 0,
            red: new_red,
            green: new_green,
            blue: new_blue,
        }
    }
}

impl From<Color> for RGBPixel {
    fn from(color: Color) -> Self {
        RGBPixel {
            _channel: 0,
            red: color.red(),
            green: color.green(),
            blue: color.blue(),
        }
    }
}

impl Pixel for AlphaPixel {
    fn composite_buffer(src: &[Self], dest: &mut [Self]) {
        for i in 0..src.len() {
            dest[i] = src[i].blend(dest[i]);
        }
    }

    fn blend(self, other: Self) -> Self {
        let alpha = self.alpha as u16;
        let red = self.red;
        let green = self.green;
        let blue = self.blue;
        let ori_red = other.red;
        let ori_green = other.green;
        let ori_blue = other.blue;
        let new_red = (((red as u16) * (255 - alpha) + (ori_red as u16) * alpha) / 255) as u8;
        let new_green = (((green as u16) * (255 - alpha) + (ori_green as u16) * alpha) / 255) as u8;
        let new_blue = (((blue as u16) * (255 - alpha) + (ori_blue as u16) * alpha) / 255) as u8;
        AlphaPixel {
            alpha: alpha as u8,
            red: new_red,
            green: new_green,
            blue: new_blue,
        }
    }

    fn weight_blend(origin: Self, other: Self, blend: f32) -> Self {
        let blend = if blend < 0f32 {
            0f32
        } else if blend > 1f32 {
            1f32
        } else {
            blend
        };

        let new_channel =
            ((origin.alpha as f32) * blend + (other.alpha as f32) * (1f32 - blend)) as u8;
        let new_red = ((origin.red as f32) * blend + (other.red as f32) * (1f32 - blend)) as u8;
        let new_green =
            ((origin.green as f32) * blend + (other.green as f32) * (1f32 - blend)) as u8;
        let new_blue = ((origin.blue as f32) * blend + (other.blue as f32) * (1f32 - blend)) as u8;
        AlphaPixel {
            alpha: new_channel,
            red: new_red,
            green: new_green,
            blue: new_blue,
        }
    }
}

impl From<Color> for AlphaPixel {
    fn from(color: Color) -> Self {
        AlphaPixel {
            alpha: color.transparency(),
            red: color.red(),
            green: color.green(),
            blue: color.blue(),
        }
    }
}