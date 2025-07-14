use alloc::string::String;

use crate::sys::framebuffer::{
    font::{CHARACTER_HEIGHT, CHARACTER_WIDTH},
    shapes::{Coordinate, Rectangle},
    FB, RGBA,
};

use super::Displayable;

/// A text displayable profiles the size and color of a block of text. It can display in a framebuffer.
#[derive(Debug)]
pub struct TextDisplay {
    width: usize,
    height: usize,
    /// The position where the next character will be displayed.
    /// This is updated after each `display()` invocation, and is useful for optimization.
    next_col: usize,
    next_line: usize,
    text: String,
    fg_color: RGBA,
    bg_color: RGBA,
    /// The cache of the text that was last displayed.
    cache: String,
}

impl Displayable for TextDisplay {
    fn display(
        &mut self,
        coordinate: Coordinate,
        framebuffer: &mut FB,
    ) -> Result<Rectangle, &'static str> {
        let (string, col, line) =
            if !self.cache.is_empty() && self.text.starts_with(self.cache.as_str()) {
                (
                    &self.text.as_str()[self.cache.len()..self.text.len()],
                    self.next_col,
                    self.next_line,
                )
            } else {
                (self.text.as_str(), 0, 0)
            };

        let (next_col, next_line, mut bounding_box) = framebuffer.print_string(
            coordinate,
            self.width as isize,
            self.height as isize,
            string,
            self.fg_color.into(),
            self.bg_color.into(),
            col as isize,
            line as isize,
        );

        if next_line < self.next_line as isize {
            bounding_box.bottom_right.y = ((self.next_line + 1) * CHARACTER_HEIGHT) as isize
        }

        self.next_col = next_col as usize;
        self.next_line = next_line as usize;
        self.cache = self.text.clone();

        Ok(bounding_box + coordinate)
    }

    fn set_size(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;
    }

    fn get_size(&self) -> (usize, usize) {
        (self.width, self.height)
    }
}

impl TextDisplay {
    /// Creates a new text displayable.
    /// # Arguments
    /// * `width`, `height`: the dimensions of the text area, in number of characters.
    /// * `fg_color`, `bg_color`: the color of the text and the background behind the text, respectively.
    pub fn new(
        width: usize,
        height: usize,
        fg_color: RGBA,
        bg_color: RGBA,
    ) -> Result<TextDisplay, &'static str> {
        Ok(TextDisplay {
            width,
            height,
            next_col: 0,
            next_line: 0,
            text: String::new(),
            fg_color,
            bg_color,
            cache: String::new(),
        })
    }

    /// Gets the background color of the text area
    pub fn get_bg_color(&self) -> RGBA {
        self.bg_color
    }

    /// Clear the cache of the text displayable.
    pub fn reset_cache(&mut self) {
        self.cache = String::new();
    }

    /// Translate the index of a character in the text to the location of the text displayable. Return (column, line).
    pub fn get_location(&self, index: usize) -> (usize, usize) {
        let text_width = self.width / CHARACTER_WIDTH;
        (index % text_width, index / text_width)
    }

    /// Translate the location of a character to its index in the text.
    pub fn get_index(&self, column: usize, line: usize) -> usize {
        let text_width = self.width / CHARACTER_WIDTH;
        line * text_width + column
    }

    /// Gets the size of a text displayable in number of characters.
    pub fn get_dimensions(&self) -> (usize, usize) {
        (self.width / CHARACTER_WIDTH, self.height / CHARACTER_HEIGHT)
    }

    /// Gets the index of next character to be displayabled. It is the position next to existing printed characters in the text displayable.
    pub fn get_next_index(&self) -> usize {
        let col_num = self.width / CHARACTER_WIDTH;
        self.next_line * col_num + self.next_col
    }

    /// Sets the text of the text displayable
    pub fn set_text(&mut self, text: &str) {
        self.text = String::from(text);
    }
}
