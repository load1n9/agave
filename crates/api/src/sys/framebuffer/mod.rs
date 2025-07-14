use crate::sys::interrupts::global_time_ms;
use alloc::vec;
use alloc::{slice, vec::Vec};
use bootloader_api::info::{FrameBufferInfo, PixelFormat};
use core::{fmt, ptr};
use font_constants::BACKUP_CHAR;
use noto_sans_mono_bitmap::{
    get_raster, get_raster_width, FontWeight, RasterHeight, RasterizedChar,
};

use self::font::{CHARACTER_HEIGHT, CHARACTER_WIDTH};
use self::shapes::{Coordinate, Rectangle};

type Ascii = u8;

/// Additional vertical space between lines
#[allow(dead_code)]
const LINE_SPACING: usize = 2;

/// Additional horizontal space between characters.
#[allow(dead_code)]
const LETTER_SPACING: usize = 0;

/// Padding from the border. Prevent that font is too close to border.
const BORDER_PADDING: usize = 1;

/// Constants for the usage of the [`noto_sans_mono_bitmap`] crate.
mod font_constants {
    use super::*;

    /// Height of each char raster. The font size is ~0.84% of this. Thus, this is the line height that
    /// enables multiple characters to be side-by-side and appear optically in one line in a natural way.
    #[allow(dead_code)]
    pub const CHAR_RASTER_HEIGHT: RasterHeight = RasterHeight::Size16;

    /// The width of each single symbol of the mono space font.
    #[allow(dead_code)]
    pub const CHAR_RASTER_WIDTH: usize = get_raster_width(FontWeight::Regular, CHAR_RASTER_HEIGHT);

    /// Backup character if a desired symbol is not available by the font.
    /// The '�' character requires the feature "unicode-specials".
    #[allow(dead_code)]
    pub const BACKUP_CHAR: char = '�';

    #[allow(dead_code)]
    pub const FONT_WEIGHT: FontWeight = FontWeight::Regular;
}

/// Returns the raster of the given char or the raster of [`font_constants::BACKUP_CHAR`].
#[allow(dead_code)]
fn get_char_raster(c: char) -> RasterizedChar {
    fn get(c: char) -> Option<RasterizedChar> {
        get_raster(
            c,
            font_constants::FONT_WEIGHT,
            font_constants::CHAR_RASTER_HEIGHT,
        )
    }
    get(c).unwrap_or_else(|| get(BACKUP_CHAR).expect("Should get raster of backup char."))
}

/// Allows logging text to a pixel-based framebuffer.
pub struct FrameBufferWriter {
    framebuffer: &'static mut [u8],
    #[allow(dead_code)]
    info: FrameBufferInfo,
    x_pos: usize,
    y_pos: usize,
    pub level: usize,
}

impl FrameBufferWriter {
    /// Creates a new logger that uses the given framebuffer.
    pub fn new(framebuffer: &'static mut [u8], info: FrameBufferInfo) -> Self {
        let mut logger = Self {
            framebuffer,
            info,
            x_pos: 0,
            y_pos: 0,
            level: 0,
        };
        logger.clear();
        logger
    }

    #[allow(dead_code)]
    fn newline(&mut self) {
        self.y_pos += font_constants::CHAR_RASTER_HEIGHT.val() + LINE_SPACING;
        self.carriage_return()
    }

    #[allow(dead_code)]
    fn carriage_return(&mut self) {
        self.x_pos = BORDER_PADDING;
    }

    /// Erases all text on the screen. Resets `self.x_pos` and `self.y_pos`.
    pub fn clear(&mut self) {
        self.x_pos = BORDER_PADDING;
        self.y_pos = BORDER_PADDING;
        self.framebuffer.fill(0);
    }

    #[allow(dead_code)]
    fn width(&self) -> usize {
        self.info.width
    }

    #[allow(dead_code)]
    fn height(&self) -> usize {
        self.info.height
    }

    /// Writes a single char to the framebuffer. Takes care of special control characters, such as
    /// newlines and carriage returns.
    #[allow(dead_code)]
    fn write_char(&mut self, c: char) {
        match c {
            '\n' => self.newline(),
            '\r' => self.carriage_return(),
            c => {
                let new_xpos = self.x_pos + font_constants::CHAR_RASTER_WIDTH;
                if new_xpos >= self.width() {
                    self.newline();
                }
                let new_ypos =
                    self.y_pos + font_constants::CHAR_RASTER_HEIGHT.val() + BORDER_PADDING;
                if new_ypos >= self.height() {
                    self.clear();
                }
                self.write_rendered_char(get_char_raster(c));
            }
        }
    }

    /// Prints a rendered char into the framebuffer.
    /// Updates `self.x_pos`.
    fn write_rendered_char(&mut self, rendered_char: RasterizedChar) {
        for (y, row) in rendered_char.raster().iter().enumerate() {
            for (x, byte) in row.iter().enumerate() {
                self.write_pixel(self.x_pos + x, self.y_pos + y, *byte);
            }
        }
        self.x_pos += rendered_char.width() + LETTER_SPACING;
    }

    fn write_pixel(&mut self, x: usize, y: usize, intensity: u8) {
        let pixel_offset = y * self.info.stride + x;

        let r = intensity;
        let g = [intensity, 0][self.level];
        let b = [intensity / 2, 0][self.level];
        let color = match self.info.pixel_format {
            PixelFormat::Rgb => [r, g, b, 0],
            PixelFormat::Bgr => [b, g, r, 0],
            PixelFormat::U8 => [if intensity > 200 { 0xf } else { 0 }, 0, 0, 0],
            other => {
                // set a supported (but invalid) pixel format before panicking to avoid a double
                // panic; it might not be readable though
                self.info.pixel_format = PixelFormat::Rgb;
                panic!("pixel format {:?} not supported in logger", other)
            }
        };
        let bytes_per_pixel = self.info.bytes_per_pixel;
        let byte_offset = pixel_offset * bytes_per_pixel;
        self.framebuffer[byte_offset..(byte_offset + bytes_per_pixel)]
            .copy_from_slice(&color[..bytes_per_pixel]);
        let _ = unsafe { ptr::read_volatile(&self.framebuffer[byte_offset]) };
    }
}

unsafe impl Send for FrameBufferWriter {}
unsafe impl Sync for FrameBufferWriter {}

impl fmt::Write for FrameBufferWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            self.write_char(c);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct RGBA {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[derive(Clone)]
#[repr(C)]
pub struct FB {
    pub pixels: Vec<RGBA>,
    pub backbuffer: Vec<RGBA>,
    pub bytes_per_pixel: usize,
    pub stride: usize,
    pub w: usize,
    pub h: usize,
}

#[repr(C)]
pub struct FBShare<'a> {
    pub pixels: &'a mut [RGBA],
    pub w: usize,
    pub h: usize,
}

impl FB {
    pub fn new(info: &FrameBufferInfo) -> Self {
        let w = info.width;
        let h = info.height;
        let mut pixels = Vec::with_capacity(w * h);
        let mut backbuffer = Vec::with_capacity(w * h);
        let bytes_per_pixel = info.bytes_per_pixel;
        let stride = info.stride;
        for y in 0..h {
            for x in 0..w {
                pixels.push(RGBA {
                    r: x as u8,
                    g: y as u8,
                    b: 0,
                    a: 0,
                });
                backbuffer.push(pixels[pixels.len() - 1]);
            }
        }
        FB {
            pixels,
            w,
            h,
            backbuffer,
            bytes_per_pixel,
            stride,
        }
    }

    pub fn update(&mut self, vec: *mut RGBA, w: usize, h: usize) {
        self.pixels = unsafe { Vec::from_raw_parts(vec, h * w, w * h) };
        self.w = w;
        self.h = h;
    }

    pub fn share(&mut self) -> FBShare {
        FBShare {
            pixels: &mut self.pixels[..],
            w: self.w,
            h: self.h,
        }
    }

    pub fn flush(&mut self, framebuffer: &mut [u8], _info: &FrameBufferInfo) {
        let todraw = &self.pixels;

        let _start = global_time_ms();

        // match _info.pixel_format {
        //     PixelFormat::Bgr => {
        //         for (idx, &i) in self.pixels.iter().enumerate() {
        //             self.backbuffer[idx].r = i.b;
        //             self.backbuffer[idx].g = i.g;
        //             self.backbuffer[idx].b = i.r;
        //         }
        //         // todraw = &self.backbuffer;
        //     }
        //     _ => {}
        // }

        // let time0 = _get_time_ms() - start;

        // let start = get_time_ms();
        framebuffer.copy_from_slice(unsafe {
            slice::from_raw_parts(todraw.as_ptr() as *const u8, framebuffer.len())
        });
        // log::info!("step 0 FB {}ms", time0);
        // log::info!("step 1 FB {}ms", get_time_ms() - start);
        // for y in 0..self.h {
        //     for x in 0..self.w {
        //         let RGBA { r, g, b, a: _ } = self.pixels[x + self.w * y];
        //         let pixel_offset = y * _info.stride + x;

        //         let color = match _info.pixel_format {
        //             PixelFormat::Rgb => [r, g, b, 0],
        //             PixelFormat::Bgr => [b, g, r, 0],
        //             PixelFormat::U8 => [if (g + b + r) as usize > 200 { 0xf } else { 0 }, 0, 255, 0],
        //             other => {
        //                 // set a supported (but invalid) pixel format before panicking to avoid a double
        //                 // panic; it might not be readable though
        //                 // info.pixel_format = PixelFormat::Rgb;
        //                 panic!("pixel format {:?} not supported in logger", other)
        //             }
        //         };
        //         let bytes_per_pixel = _info.bytes_per_pixel;
        //         let byte_offset = pixel_offset * bytes_per_pixel;
        //         framebuffer[byte_offset..(byte_offset + bytes_per_pixel)]
        //             .copy_from_slice(&color[..bytes_per_pixel]);
        //     }
        // }
    }

    pub fn index_of(&self, coord: Coordinate) -> Option<usize> {
        if self.contains(coord) {
            Some((coord.x + self.w as isize * coord.y) as usize)
        } else {
            None
        }
    }

    pub fn set(&mut self, x: usize, y: usize, color: RGBA) {
        let idx = x + self.w * y;
        self.pixels[idx] = color;
    }

    pub fn set_pixel(&mut self, coord: Coordinate, color: RGBA) {
        let idx = coord.x as usize + self.w * coord.y as usize;
        self.pixels[idx] = color;
    }

    pub fn contains(&self, coordinate: Coordinate) -> bool {
        coordinate.x >= 0
            && coordinate.x < self.w as isize
            && coordinate.y >= 0
            && coordinate.y < self.h as isize
    }

    pub fn overlaps_with(&self, coordinate: Coordinate, w: usize, h: usize) -> bool {
        coordinate.x < self.w as isize
            && coordinate.x + w as isize >= 0
            && coordinate.y < self.h as isize
            && coordinate.y + h as isize >= 0
    }

    pub fn composite_buffer(&mut self, buffer: &[RGBA], start: usize) {
        for (idx, &i) in buffer.iter().enumerate() {
            self.pixels[start + idx] = i;
        }
    }

    pub fn fill_blank(&mut self, blank: &mut Rectangle, pixel: RGBA) {
        let (width, height) = (self.w as isize, self.h as isize);
        blank.top_left.x = core::cmp::max(0, blank.top_left.x);
        blank.top_left.y = core::cmp::max(0, blank.top_left.y);
        blank.bottom_right.x = core::cmp::min(blank.bottom_right.x, width as isize);
        blank.bottom_right.y = core::cmp::min(blank.bottom_right.y, height as isize);

        if blank.top_left.x >= blank.bottom_right.x || blank.top_left.y >= blank.bottom_right.y {
            return;
        }

        let fill = vec![pixel; (blank.bottom_right.x - blank.top_left.x) as usize];
        let mut coordinate = blank.top_left;
        loop {
            if coordinate.y == blank.bottom_right.y {
                return;
            }
            if let Some(start) = self.index_of(coordinate) {
                self.composite_buffer(&fill, start);
            }
            coordinate.y += 1;
        }
    }

    pub fn print_ascii_character(
        &mut self,
        character: Ascii,
        fg_pixel: RGBA,
        bg_pixel: RGBA,
        coordinate: Coordinate,
        column: isize,
        line: isize,
    ) {
        let start = coordinate
            + (
                (column * CHARACTER_WIDTH as isize) as isize,
                (line * CHARACTER_HEIGHT as isize) as isize,
            );
        if !self.overlaps_with(start, CHARACTER_WIDTH, CHARACTER_HEIGHT) {
            return;
        }
        let (buffer_width, buffer_height) = (self.w, self.h);
        let off_set_x: isize = if start.x < 0 {
            -(start.x as isize) as isize
        } else {
            0
        };
        let off_set_y: isize = if start.y < 0 {
            -(start.y as isize) as isize
        } else {
            0
        };
        let mut j = off_set_x;
        let mut i = off_set_y;
        loop {
            let coordinate = start + (j as isize, i as isize);
            if self.contains(coordinate) {
                let pixel = if j >= 1 {
                    let index = j - 1;
                    let char_font = self::font::FONT_BASIC[character as usize][i as usize];
                    if get_bit(char_font as u8, index) != 0 {
                        fg_pixel
                    } else {
                        bg_pixel
                    }
                } else {
                    bg_pixel
                };
                self.set(coordinate.x as usize, coordinate.y as usize, pixel);
            }
            j += 1;
            if j == CHARACTER_WIDTH as isize || start.x + j as isize == buffer_width as isize {
                i += 1;
                if i == CHARACTER_HEIGHT as isize || start.y + i as isize == buffer_height as isize
                {
                    return;
                }
                j = off_set_x;
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn print_string(
        &mut self,
        coordinate: Coordinate,
        width: isize,
        height: isize,
        slice: &str,
        fg_pixel: RGBA,
        bg_pixel: RGBA,
        column: isize,
        line: isize,
    ) -> (isize, isize, Rectangle) {
        let buffer_width = width / CHARACTER_WIDTH as isize;
        let buffer_height = height / CHARACTER_HEIGHT as isize;
        let (x, y) = (coordinate.x, coordinate.y);

        let mut curr_line = line;
        let mut curr_column = column;

        let top_left = Coordinate::new(0, (curr_line * CHARACTER_HEIGHT as isize) as isize);

        for byte in slice.bytes() {
            if byte == b'\n' {
                let mut blank = Rectangle {
                    top_left: Coordinate::new(
                        coordinate.x + (curr_column * CHARACTER_WIDTH as isize) as isize,
                        coordinate.y + (curr_line * CHARACTER_HEIGHT as isize) as isize,
                    ),
                    bottom_right: Coordinate::new(
                        coordinate.x + width as isize,
                        coordinate.y + ((curr_line + 1) * CHARACTER_HEIGHT as isize) as isize,
                    ),
                };
                self.fill_blank(&mut blank, bg_pixel);
                curr_column = 0;
                curr_line += 1;
                if curr_line == buffer_height {
                    break;
                }
            } else {
                if curr_column == buffer_width {
                    curr_column = 0;
                    curr_line += 1;
                    if curr_line == buffer_height {
                        break;
                    }
                }
                self.print_ascii_character(
                    byte,
                    fg_pixel,
                    bg_pixel,
                    coordinate,
                    curr_column,
                    curr_line,
                );
                curr_column += 1;
            }
        }

        let mut blank = Rectangle {
            top_left: Coordinate::new(
                x + (curr_column * CHARACTER_WIDTH as isize) as isize,
                y + (curr_line * CHARACTER_HEIGHT as isize) as isize,
            ),
            bottom_right: Coordinate::new(
                x + width as isize,
                y + ((curr_line + 1) * CHARACTER_HEIGHT as isize) as isize,
            ),
        };
        self.fill_blank(&mut blank, bg_pixel);

        let bottom_right = Coordinate::new(
            (buffer_width * CHARACTER_WIDTH as isize) as isize,
            ((curr_line + 1) * CHARACTER_HEIGHT as isize) as isize,
        );

        let update_area = Rectangle {
            top_left,
            bottom_right,
        };

        blank = Rectangle {
            top_left: Coordinate::new(
                x,
                y + ((curr_line + 1) * CHARACTER_HEIGHT as isize) as isize,
            ),
            bottom_right: Coordinate::new(x + width as isize, y + height as isize),
        };
        self.fill_blank(&mut blank, bg_pixel);

        (curr_column, curr_line, update_area)
    }

    pub fn draw_line(&mut self, start: Coordinate, end: Coordinate, pixel: RGBA) {
        let width: isize = end.x - start.x;
        let height: isize = end.y - start.y;

        let mut line_in_buffer = false;

        if width.abs() > height.abs() {
            let mut y;
            let mut x = start.x;

            let step = if width > 0 { 1 } else { -1 };
            loop {
                if x == end.x {
                    break;
                }
                y = (x - start.x) * height / width + start.y;
                let coordinate = Coordinate::new(x, y);
                if self.contains(coordinate) {
                    line_in_buffer = true;
                    self.set_pixel(coordinate, pixel);
                } else if line_in_buffer {
                    break;
                }
                x += step;
            }
        } else {
            let mut x;
            let mut y = start.y;
            let step = if height > 0 { 1 } else { -1 };
            loop {
                if y == end.y {
                    break;
                }
                x = (y - start.y) * width / height + start.x;
                let coordinate = Coordinate::new(x, y);
                if self.contains(coordinate) {
                    line_in_buffer = true;
                    self.set_pixel(coordinate, pixel);
                } else if line_in_buffer {
                    break;
                }
                y += step;
            }
        }
    }

    pub fn draw_rectangle(
        &mut self,
        coordinate: Coordinate,
        width: usize,
        height: usize,
        pixel: RGBA,
    ) {
        let (buffer_width, buffer_height) = (self.w, self.h);

        if !self.overlaps_with(coordinate, width, height) {
            return;
        }

        let start_x = core::cmp::max(coordinate.x, 0);
        let start_y = core::cmp::max(coordinate.y, 0);
        let end_x = core::cmp::min(coordinate.x + width as isize, buffer_width as isize);
        let end_y = core::cmp::min(coordinate.y + height as isize, buffer_height as isize);

        let mut top = Coordinate::new(start_x, start_y);
        let end_y_offset = end_y - start_y - 1;
        loop {
            if top.x == end_x {
                break;
            }
            if coordinate.y >= 0 {
                self.set_pixel(top, pixel);
            }
            if (coordinate.y + height as isize) < buffer_height as isize {
                self.set_pixel(top + (0, end_y_offset), pixel);
            }
            top.x += 1;
        }

        let mut left = Coordinate::new(start_x, start_y);
        let end_x_offset = end_x - start_x - 1;
        loop {
            if left.y == end_y {
                break;
            }
            if coordinate.x >= 0 {
                self.set_pixel(left, pixel);
            }
            if (coordinate.x + width as isize) < buffer_width as isize {
                self.set_pixel(left + (end_x_offset, 0), pixel);
            }
            left.y += 1;
        }
    }

    pub fn fill_rectangle(
        &mut self,
        coordinate: Coordinate,
        width: usize,
        height: usize,
        pixel: RGBA,
    ) {
        let (buffer_width, buffer_height) = (self.w, self.h);
        if !self.overlaps_with(coordinate, width, height) {
            return;
        }

        let start_x = core::cmp::max(coordinate.x, 0);
        let start_y = core::cmp::max(coordinate.y, 0);
        let end_x = core::cmp::min(coordinate.x + width as isize, buffer_width as isize);
        let end_y = core::cmp::min(coordinate.y + height as isize, buffer_height as isize);

        let mut coordinate = Coordinate::new(start_x, start_y);
        loop {
            loop {
                self.set_pixel(coordinate, pixel);
                coordinate.x += 1;
                if coordinate.x == end_x {
                    break;
                }
            }
            coordinate.y += 1;
            if coordinate.y == end_y {
                break;
            }
            coordinate.x = start_x;
        }
    }

    pub fn draw_circle(&mut self, center: Coordinate, r: usize, pixel: RGBA) {
        let r2 = (r * r) as isize;
        for y in center.y - r as isize..center.y + r as isize {
            for x in center.x - r as isize..center.x + r as isize {
                let coordinate = Coordinate::new(x, y);
                if self.contains(coordinate) {
                    let d = coordinate - center;
                    if d.x * d.x + d.y * d.y <= r2 {
                        self.set_pixel(coordinate, pixel);
                    }
                }
            }
        }
    }
}

fn get_bit(char_font: u8, i: isize) -> u8 {
    char_font & (0x80 >> i)
}

pub mod display;
pub mod font;
pub mod shapes;
