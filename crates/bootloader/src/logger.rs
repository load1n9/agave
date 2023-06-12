use core::{
    fmt::{self, Write},
    ptr,
};
use font_constants::BACKUP_CHAR;
use noto_sans_mono_bitmap::{
    get_raster, get_raster_width, FontWeight, RasterHeight, RasterizedChar,
};
use spin::{Mutex, Once};
use uefi_bootloader_api::{FrameBufferInfo, PixelFormat};

/// The global logger instance used for the `log` crate.
pub(crate) static LOGGER: Once<LockedLogger> = Once::new();

/// A [`Logger`] instance protected by a spinlock.
pub(crate) struct LockedLogger(Mutex<Logger>);

/// Additional vertical space between lines
const LINE_SPACING: usize = 2;
/// Additional horizontal space between characters.
const LETTER_SPACING: usize = 0;

/// Padding from the border. Prevent that font is too close to border.
const BORDER_PADDING: usize = 1;

/// Constants for the usage of the [`noto_sans_mono_bitmap`] crate.
mod font_constants {
    use super::{get_raster_width, FontWeight, RasterHeight};

    /// Height of each char raster. The font size is ~0.84% of this. Thus, this
    /// is the line height that enables multiple characters to be
    /// side-by-side and appear optically in one line in a natural way.
    pub(crate) const CHAR_RASTER_HEIGHT: RasterHeight = RasterHeight::Size16;

    /// The width of each single symbol of the mono space font.
    pub(crate) const CHAR_RASTER_WIDTH: usize =
        get_raster_width(FontWeight::Regular, CHAR_RASTER_HEIGHT);

    /// Backup character if a desired symbol is not available by the font.
    /// The '�' character requires the feature "unicode-specials".
    pub(crate) const BACKUP_CHAR: char = '�';

    pub(crate) const FONT_WEIGHT: FontWeight = FontWeight::Regular;
}

/// Returns the raster of the given char or the raster of
/// [`font_constants::BACKUP_CHAR`].
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

impl LockedLogger {
    /// Create a new instance that logs to the given framebuffer.
    pub(crate) fn new(framebuffer: &'static mut [u8], info: FrameBufferInfo) -> Self {
        LockedLogger(Mutex::new(Logger::new(framebuffer, info)))
    }

    /// Force-unlocks the logger to prevent a deadlock.
    ///
    /// # Safety
    ///
    /// The caller must ensure no other thread could simultaneously access the
    /// underlying logger.
    pub(crate) unsafe fn force_unlock(&self) {
        // SAFETY: Guaranteed by caller.
        unsafe { self.0.force_unlock() };
    }
}

impl log::Log for LockedLogger {
    fn enabled(&self, _metadata: &log::Metadata<'_>) -> bool {
        true
    }

    fn log(&self, record: &log::Record<'_>) {
        let mut logger = self.0.lock();
        writeln!(logger, "{:5}: {}", record.level(), record.args()).unwrap();
    }

    fn flush(&self) {}
}

/// Allows logging text to a pixel-based framebuffer.
pub(crate) struct Logger {
    framebuffer: &'static mut [u8],
    info: FrameBufferInfo,
    x_pos: usize,
    y_pos: usize,
}

impl Logger {
    /// Creates a new logger that uses the given framebuffer.
    pub(crate) fn new(framebuffer: &'static mut [u8], info: FrameBufferInfo) -> Self {
        let mut logger = Self {
            framebuffer,
            info,
            x_pos: 0,
            y_pos: 0,
        };
        logger.clear();
        logger
    }

    fn newline(&mut self) {
        self.y_pos += font_constants::CHAR_RASTER_HEIGHT.val() + LINE_SPACING;
        self.carriage_return();
    }

    fn carriage_return(&mut self) {
        self.x_pos = BORDER_PADDING;
    }

    /// Erases all text on the screen. Resets `self.x_pos` and `self.y_pos`.
    pub(crate) fn clear(&mut self) {
        self.x_pos = BORDER_PADDING;
        self.y_pos = BORDER_PADDING;
        self.framebuffer.fill(0);
    }

    fn width(&self) -> usize {
        self.info.width
    }

    fn height(&self) -> usize {
        self.info.height
    }

    /// Writes a single char to the framebuffer. Takes care of special control
    /// characters, such as newlines and carriage returns.
    #[allow(clippy::same_name_method, clippy::similar_names)]
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
                self.write_rendered_char(&get_char_raster(c));
            }
        }
    }

    /// Prints a rendered char into the framebuffer.
    /// Updates `self.x_pos`.
    fn write_rendered_char(&mut self, rendered_char: &RasterizedChar) {
        for (y, row) in rendered_char.raster().iter().enumerate() {
            for (x, byte) in row.iter().enumerate() {
                self.write_pixel(self.x_pos + x, self.y_pos + y, *byte);
            }
        }
        self.x_pos += rendered_char.width() + LETTER_SPACING;
    }

    fn write_pixel(&mut self, x: usize, y: usize, intensity: u8) {
        let pixel_offset = y * self.info.stride + x;
        let color = match self.info.pixel_format {
            PixelFormat::Rgb => [intensity, intensity, intensity / 2, 0],
            PixelFormat::Bgr => [intensity / 2, intensity, intensity, 0],
        };
        let bytes_per_pixel = self.info.bytes_per_pixel;
        let byte_offset = pixel_offset * bytes_per_pixel;
        self.framebuffer[byte_offset..(byte_offset + bytes_per_pixel)]
            .copy_from_slice(&color[..bytes_per_pixel]);
        // SAFETY: The frame buffer is valid.
        let _ = unsafe { ptr::read_volatile(&self.framebuffer[byte_offset]) };
    }
}

// SAFETY: 🤷
unsafe impl Send for Logger {}
// SAFETY: 🤷
unsafe impl Sync for Logger {}

impl Write for Logger {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            self.write_char(c);
        }
        Ok(())
    }
}
