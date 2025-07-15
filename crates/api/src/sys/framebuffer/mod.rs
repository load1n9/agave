pub mod display;
pub mod font;
pub mod shapes;

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

    #[allow(mismatched_lifetime_syntaxes)]
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

    /// Draw a gradient between two colors
    pub fn fill_gradient(
        &mut self,
        start: Coordinate,
        end: Coordinate,
        color1: RGBA,
        color2: RGBA,
    ) {
        let width = (end.x - start.x).abs() as f32;
        let height = (end.y - start.y).abs() as f32;
        let is_horizontal = width > height;

        if is_horizontal {
            for x in start.x..=end.x {
                let progress = (x - start.x) as f32 / width;
                let blended_color = self.blend_colors(color1, color2, progress);

                for y in start.y..=end.y {
                    let coord = Coordinate::new(x, y);
                    if self.contains(coord) {
                        self.set_pixel(coord, blended_color);
                    }
                }
            }
        } else {
            for y in start.y..=end.y {
                let progress = (y - start.y) as f32 / height;
                let blended_color = self.blend_colors(color1, color2, progress);

                for x in start.x..=end.x {
                    let coord = Coordinate::new(x, y);
                    if self.contains(coord) {
                        self.set_pixel(coord, blended_color);
                    }
                }
            }
        }
    }

    /// Blend two colors with alpha interpolation
    fn blend_colors(&self, color1: RGBA, color2: RGBA, progress: f32) -> RGBA {
        let progress = progress.max(0.0).min(1.0);
        let inv_progress = 1.0 - progress;

        RGBA {
            r: (color1.r as f32 * inv_progress + color2.r as f32 * progress) as u8,
            g: (color1.g as f32 * inv_progress + color2.g as f32 * progress) as u8,
            b: (color1.b as f32 * inv_progress + color2.b as f32 * progress) as u8,
            a: (color1.a as f32 * inv_progress + color2.a as f32 * progress) as u8,
        }
    }

    /// Draw an antialiased line using Xiaolin Wu's algorithm
    pub fn draw_line_aa(&mut self, start: Coordinate, end: Coordinate, color: RGBA) {
        let dx = (end.x - start.x).abs();
        let dy = (end.y - start.y).abs();
        let steep = dy > dx;

        let (x0, y0, x1, y1) = if steep {
            (start.y, start.x, end.y, end.x)
        } else {
            (start.x, start.y, end.x, end.y)
        };

        let (x0, y0, x1, y1) = if x0 > x1 {
            (x1, y1, x0, y0)
        } else {
            (x0, y0, x1, y1)
        };

        let dx = x1 - x0;
        let dy = y1 - y0;
        let gradient = if dx == 0 { 1.0 } else { dy as f32 / dx as f32 };

        // Handle first endpoint
        let xend = x0;
        let yend = y0 as f32 + gradient * (xend - x0) as f32;
        let xgap = 1.0 - ((x0 as f32 + 0.5) - x0 as f32);
        let xpxl1 = xend;
        let ypxl1 = yend as isize;

        if steep {
            self.plot_aa(ypxl1, xpxl1, color, (1.0 - (yend - ypxl1 as f32)) * xgap);
            self.plot_aa(ypxl1 + 1, xpxl1, color, (yend - ypxl1 as f32) * xgap);
        } else {
            self.plot_aa(xpxl1, ypxl1, color, (1.0 - (yend - ypxl1 as f32)) * xgap);
            self.plot_aa(xpxl1, ypxl1 + 1, color, (yend - ypxl1 as f32) * xgap);
        }

        let mut intery = yend + gradient;

        // Handle second endpoint
        let xend = x1;
        let yend = y1 as f32 + gradient * (xend - x1) as f32;
        let xgap = (x1 as f32 + 0.5) - x1 as f32;
        let xpxl2 = xend;
        let ypxl2 = yend as isize;

        if steep {
            self.plot_aa(ypxl2, xpxl2, color, (1.0 - (yend - ypxl2 as f32)) * xgap);
            self.plot_aa(ypxl2 + 1, xpxl2, color, (yend - ypxl2 as f32) * xgap);
        } else {
            self.plot_aa(xpxl2, ypxl2, color, (1.0 - (yend - ypxl2 as f32)) * xgap);
            self.plot_aa(xpxl2, ypxl2 + 1, color, (yend - ypxl2 as f32) * xgap);
        }

        // Main loop
        for x in (xpxl1 + 1)..xpxl2 {
            let intery_int = intery as isize;
            let fract = intery - intery_int as f32;
            if steep {
                self.plot_aa(intery_int, x, color, 1.0 - fract);
                self.plot_aa(intery_int + 1, x, color, fract);
            } else {
                self.plot_aa(x, intery_int, color, 1.0 - fract);
                self.plot_aa(x, intery_int + 1, color, fract);
            }
            intery += gradient;
        }
    }

    /// Plot a pixel with alpha blending for antialiasing
    fn plot_aa(&mut self, x: isize, y: isize, color: RGBA, alpha: f32) {
        let coord = Coordinate::new(x, y);
        if self.contains(coord) {
            let blended_color = RGBA {
                r: color.r,
                g: color.g,
                b: color.b,
                a: (color.a as f32 * alpha) as u8,
            };
            self.set_pixel_blend(coord, blended_color);
        }
    }

    /// Set pixel with alpha blending
    pub fn set_pixel_blend(&mut self, coord: Coordinate, color: RGBA) {
        if !self.contains(coord) {
            return;
        }

        let idx = coord.x as usize + self.w * coord.y as usize;
        let existing = self.pixels[idx];

        let alpha = color.a as f32 / 255.0;
        let inv_alpha = 1.0 - alpha;

        self.pixels[idx] = RGBA {
            r: (color.r as f32 * alpha + existing.r as f32 * inv_alpha) as u8,
            g: (color.g as f32 * alpha + existing.g as f32 * inv_alpha) as u8,
            b: (color.b as f32 * alpha + existing.b as f32 * inv_alpha) as u8,
            a: (color.a as f32 * alpha + existing.a as f32 * inv_alpha).min(255.0) as u8,
        };
    }

    /// Draw a rounded rectangle
    pub fn draw_rounded_rectangle(
        &mut self,
        coordinate: Coordinate,
        width: usize,
        height: usize,
        radius: usize,
        color: RGBA,
    ) {
        let radius = radius.min(width / 2).min(height / 2);

        // Draw the main rectangles
        self.fill_rectangle(
            coordinate + (radius as isize, 0),
            width - 2 * radius,
            height,
            color,
        );
        self.fill_rectangle(
            coordinate + (0, radius as isize),
            width,
            height - 2 * radius,
            color,
        );

        // Draw the rounded corners
        let corners = [
            (coordinate + (radius as isize, radius as isize)), // Top-left
            (coordinate + ((width - radius) as isize, radius as isize)), // Top-right
            (coordinate + (radius as isize, (height - radius) as isize)), // Bottom-left
            (coordinate + ((width - radius) as isize, (height - radius) as isize)), // Bottom-right
        ];

        for &corner in &corners {
            self.draw_circle_filled(corner, radius, color);
        }
    }

    /// Draw a filled circle
    pub fn draw_circle_filled(&mut self, center: Coordinate, radius: usize, color: RGBA) {
        let radius_sq = (radius * radius) as isize;
        let r = radius as isize;

        for y in -r..=r {
            for x in -r..=r {
                if x * x + y * y <= radius_sq {
                    let coord = center + (x, y);
                    if self.contains(coord) {
                        self.set_pixel(coord, color);
                    }
                }
            }
        }
    }

    /// Draw a shadow effect
    pub fn draw_shadow(
        &mut self,
        coordinate: Coordinate,
        width: usize,
        height: usize,
        blur_radius: usize,
        color: RGBA,
    ) {
        let shadow_offset = Coordinate::new(blur_radius as isize / 2, blur_radius as isize / 2);
        let shadow_coord = coordinate + (shadow_offset.x, shadow_offset.y);

        for y in 0..height + blur_radius {
            for x in 0..width + blur_radius {
                let coord = shadow_coord + (x as isize, y as isize);
                if self.contains(coord) {
                    // Simple shadow - could be enhanced with proper gaussian blur
                    let distance_from_edge = {
                        let dx = if x < blur_radius {
                            blur_radius - x
                        } else if x >= width {
                            x - width + 1
                        } else {
                            0
                        };
                        let dy = if y < blur_radius {
                            blur_radius - y
                        } else if y >= height {
                            y - height + 1
                        } else {
                            0
                        };
                        // Simple integer square root approximation
                        let dist_sq = dx * dx + dy * dy;
                        if dist_sq == 0 {
                            0.0
                        } else {
                            // Newton's method approximation
                            let mut guess = (dist_sq / 2) as f32;
                            for _ in 0..3 {
                                // 3 iterations for reasonable accuracy
                                guess = (guess + dist_sq as f32 / guess) / 2.0;
                            }
                            guess
                        }
                    };

                    if distance_from_edge <= blur_radius as f32 {
                        let alpha_factor = 1.0 - (distance_from_edge / blur_radius as f32);
                        let shadow_color = RGBA {
                            r: color.r,
                            g: color.g,
                            b: color.b,
                            a: (color.a as f32 * alpha_factor * 0.3) as u8,
                        };
                        self.set_pixel_blend(coord, shadow_color);
                    }
                }
            }
        }
    }

    /// Draw text
    pub fn draw_text(
        &mut self,
        coordinate: Coordinate,
        text: &str,
        color: RGBA,
        background: Option<RGBA>,
        scale: usize,
    ) {
        let scale = scale.max(1);
        let char_width = CHARACTER_WIDTH * scale;
        let char_height = CHARACTER_HEIGHT * scale;

        for (i, byte) in text.bytes().enumerate() {
            let char_coord = coordinate + ((i * char_width) as isize, 0);

            // Draw background if specified
            if let Some(bg_color) = background {
                self.fill_rectangle(char_coord, char_width, char_height, bg_color);
            }

            // Draw character with scaling
            self.print_ascii_character_scaled(byte, color, char_coord, scale);
        }
    }

    /// Print ASCII character with scaling support
    fn print_ascii_character_scaled(
        &mut self,
        character: Ascii,
        color: RGBA,
        coordinate: Coordinate,
        scale: usize,
    ) {
        let scale = scale.max(1);

        for i in 0..(CHARACTER_HEIGHT) {
            for j in 0..(CHARACTER_WIDTH - 1) {
                // -1 because font is 7 pixels wide
                let char_font = self::font::FONT_BASIC[character as usize][i];
                if get_bit(char_font as u8, j as isize) != 0 {
                    // Draw scaled pixel
                    for sy in 0..scale {
                        for sx in 0..scale {
                            let pixel_coord =
                                coordinate + ((j * scale + sx) as isize, (i * scale + sy) as isize);
                            if self.contains(pixel_coord) {
                                self.set_pixel(pixel_coord, color);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Draw a bezier curve
    pub fn draw_bezier_curve(
        &mut self,
        p0: Coordinate,
        p1: Coordinate,
        p2: Coordinate,
        p3: Coordinate,
        color: RGBA,
        segments: usize,
    ) {
        let segments = segments.max(10);
        let mut prev_point = p0;

        for i in 1..=segments {
            let t = i as f32 / segments as f32;
            let t2 = t * t;
            let t3 = t2 * t;
            let mt = 1.0 - t;
            let mt2 = mt * mt;
            let mt3 = mt2 * mt;

            let x = (mt3 * p0.x as f32)
                + (3.0 * mt2 * t * p1.x as f32)
                + (3.0 * mt * t2 * p2.x as f32)
                + (t3 * p3.x as f32);
            let y = (mt3 * p0.y as f32)
                + (3.0 * mt2 * t * p1.y as f32)
                + (3.0 * mt * t2 * p2.y as f32)
                + (t3 * p3.y as f32);

            let current_point = Coordinate::new(x as isize, y as isize);
            self.draw_line_aa(prev_point, current_point, color);
            prev_point = current_point;
        }
    }

    pub fn fill(&mut self, coordinate: Coordinate, width: usize, height: usize, color: RGBA) {
        if !self.overlaps_with(coordinate, width, height) {
            return;
        }

        let start_x = core::cmp::max(coordinate.x, 0) as usize;
        let start_y = core::cmp::max(coordinate.y, 0) as usize;
        let end_x = core::cmp::min(coordinate.x + width as isize, self.w as isize) as usize;
        let end_y = core::cmp::min(coordinate.y + height as isize, self.h as isize) as usize;

        // Use memset-like operation for better performance
        for y in start_y..end_y {
            let start_idx = y * self.w + start_x;
            let end_idx = y * self.w + end_x;

            for idx in start_idx..end_idx {
                self.pixels[idx] = color;
            }
        }
    }
}
fn get_bit(char_font: u8, i: isize) -> u8 {
    char_font & (0x80 >> i)
}

/// Animation and transition utilities for smooth UI effects
pub struct Animator {
    pub duration_ms: u64,
    pub start_time: u64,
    pub current_time: u64,
}

impl Animator {
    pub fn new(duration_ms: u64) -> Self {
        Self {
            duration_ms,
            start_time: 0,
            current_time: 0,
        }
    }

    pub fn start(&mut self, current_time: u64) {
        self.start_time = current_time;
        self.current_time = current_time;
    }

    pub fn update(&mut self, current_time: u64) {
        self.current_time = current_time;
    }

    pub fn progress(&self) -> f32 {
        if self.duration_ms == 0 {
            return 1.0;
        }

        let elapsed = self.current_time.saturating_sub(self.start_time);
        (elapsed as f32 / self.duration_ms as f32).min(1.0)
    }

    pub fn is_finished(&self) -> bool {
        self.progress() >= 1.0
    }

    /// Ease-in-out cubic animation curve
    pub fn ease_in_out(&self) -> f32 {
        let t = self.progress();
        if t < 0.5 {
            4.0 * t * t * t
        } else {
            let temp = -2.0 * t + 2.0;
            1.0 - (temp * temp * temp) / 2.0
        }
    }

    /// Bounce animation curve
    pub fn bounce(&self) -> f32 {
        let t = self.progress();
        if t < 1.0 / 2.75 {
            7.5625 * t * t
        } else if t < 2.0 / 2.75 {
            let t = t - 1.5 / 2.75;
            7.5625 * t * t + 0.75
        } else if t < 2.5 / 2.75 {
            let t = t - 2.25 / 2.75;
            7.5625 * t * t + 0.9375
        } else {
            let t = t - 2.625 / 2.75;
            7.5625 * t * t + 0.984375
        }
    }
}

/// Particle system for visual effects
pub struct Particle {
    pub position: Coordinate,
    pub velocity: Coordinate,
    pub color: RGBA,
    pub life: f32,
    pub max_life: f32,
    pub size: usize,
}

impl Particle {
    pub fn new(
        position: Coordinate,
        velocity: Coordinate,
        color: RGBA,
        life: f32,
        size: usize,
    ) -> Self {
        Self {
            position,
            velocity,
            color,
            life,
            max_life: life,
            size,
        }
    }

    pub fn update(&mut self, delta_time: f32) {
        self.position = self.position
            + (
                (self.velocity.x as f32 * delta_time) as isize,
                (self.velocity.y as f32 * delta_time) as isize,
            );
        self.life -= delta_time;
    }

    pub fn is_alive(&self) -> bool {
        self.life > 0.0
    }

    pub fn alpha(&self) -> u8 {
        ((self.life / self.max_life) * self.color.a as f32) as u8
    }
}

pub struct ParticleSystem {
    pub particles: Vec<Particle>,
    pub max_particles: usize,
}

impl ParticleSystem {
    pub fn new(max_particles: usize) -> Self {
        Self {
            particles: Vec::with_capacity(max_particles),
            max_particles,
        }
    }

    pub fn emit(&mut self, particle: Particle) {
        if self.particles.len() < self.max_particles {
            self.particles.push(particle);
        }
    }

    pub fn update(&mut self, delta_time: f32) {
        for particle in &mut self.particles {
            particle.update(delta_time);
        }
        self.particles.retain(|p| p.is_alive());
    }

    pub fn render(&self, fb: &mut FB) {
        for particle in &self.particles {
            let mut color = particle.color;
            color.a = particle.alpha();

            if particle.size <= 1 {
                fb.set_pixel_blend(particle.position, color);
            } else {
                fb.draw_circle_filled(particle.position, particle.size, color);
            }
        }
    }
}
