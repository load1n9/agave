use modular_bitfield::{bitfield, specifiers::B3};

#[derive(Debug, Clone)]
pub struct MouseMovementRelative {
    pub x_movement: i16,
    pub y_movement: i16,
    pub scroll_movement: i8,
}

impl MouseMovementRelative {
    pub fn new(x_movement: i16, y_movement: i16, scroll_movement: i8) -> Self {
        Self {
            x_movement,
            y_movement,
            scroll_movement,
        }
    }
}

#[bitfield(bits = 8)]
#[derive(Debug, Clone)]
pub struct MouseButtons {
    pub left: bool,
    pub right: bool,
    pub middle: bool,
    pub fourth: bool,
    pub fifth: bool,
    #[skip]
    __: B3,
}

#[derive(Debug, Clone)]
pub struct MouseEvent {
    pub buttons: MouseButtons,
    pub movement: MouseMovementRelative,
}

impl MouseEvent {
    pub fn new(buttons: MouseButtons, movement: MouseMovementRelative) -> MouseEvent {
        MouseEvent { buttons, movement }
    }
}
