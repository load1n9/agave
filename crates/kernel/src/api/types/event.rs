// ported from https://github.com/theseus-os/Theseus/blob/theseus_main/kernel/event_types/src/lib.rs
use crate::api::data::keycodes::KeyEvent;
use crate::api::data::mouse::MouseEvent;
use crate::api::shapes::{Coord, Rect};
use alloc::string::String;

#[derive(Debug, Clone)]
pub struct MousePositionEvent {
    pub coordinate: Coord,
    pub gcoordinate: Coord,
    pub scrolling_up: bool,
    pub scrolling_down: bool,
    pub left_button_hold: bool,
    pub right_button_hold: bool,
    pub fourth_button_hold: bool,
    pub fifth_button_hold: bool,
}

impl Default for MousePositionEvent {
    fn default() -> Self {
        MousePositionEvent {
            coordinate: Coord::new(0, 0),
            gcoordinate: Coord::new(0, 0),
            scrolling_up: false,
            scrolling_down: false,
            left_button_hold: false,
            right_button_hold: false,
            fourth_button_hold: false,
            fifth_button_hold: false,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    KeyboardEvent(KeyboardInputEvent),
    MouseMovementEvent(MouseEvent),
    OutputEvent(String),
    WindowResizeEvent(Rect),
    MousePositionEvent(MousePositionEvent),
    ExitEvent,
}

impl Event {
    pub fn new_keyboard_event(kev: KeyEvent) -> Event {
        Event::KeyboardEvent(KeyboardInputEvent::new(kev))
    }

    pub fn new_output_event<S>(s: S) -> Event
    where
        S: Into<String>,
    {
        Event::OutputEvent(s.into())
    }

    pub fn new_window_resize_event(new_position: Rect) -> Event {
        Event::WindowResizeEvent(new_position)
    }
}

#[derive(Debug, Clone)]
pub struct KeyboardInputEvent {
    pub key_event: KeyEvent,
}

impl KeyboardInputEvent {
    pub fn new(key_event: KeyEvent) -> KeyboardInputEvent {
        KeyboardInputEvent { key_event }
    }
}