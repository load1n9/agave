// ported from https://github.com/rust-osdev/ps2-mouse
use bitflags::bitflags;
use x86_64::instructions::port::Port;

const ADDRESS_PORT_ADDRESS: u16 = 0x64;
const DATA_PORT_ADDRESS: u16 = 0x60;
const GET_STATUS_BYTE: u8 = 0x20;
const SET_STATUS_BYTE: u8 = 0x60;

bitflags! {
    /// Represents the flags currently set for the mouse.
    #[derive(Default)]
    pub struct MouseFlags: u8 {
        /// Whether or not the left mouse button is pressed.
        const LEFT_BUTTON = 0b0000_0001;

        /// Whether or not the right mouse button is pressed.
        const RIGHT_BUTTON = 0b0000_0010;

        /// Whether or not the middle mouse button is pressed.
        const MIDDLE_BUTTON = 0b0000_0100;

        /// Whether or not the packet is valid or not.
        const ALWAYS_ONE = 0b0000_1000;

        /// Whether or not the x delta is negative.
        const X_SIGN = 0b0001_0000;

        /// Whether or not the y delta is negative.
        const Y_SIGN = 0b0010_0000;

        /// Whether or not the x delta overflowed.
        const X_OVERFLOW = 0b0100_0000;

        /// Whether or not the y delta overflowed.
        const Y_OVERFLOW = 0b1000_0000;
    }
}

#[repr(u8)]
enum Command {
    EnablePacketStreaming = 0xF4,
    SetDefaults = 0xF6,
}

/// A basic interface to interact with a PS2 mouse.
#[derive(Debug)]
pub struct Mouse {
    command_port: Port<u8>,
    data_port: Port<u8>,
    current_packet: u8,
    current_state: MouseState,
    completed_state: MouseState,
    on_complete: Option<fn(MouseState)>,
}

impl Default for Mouse {
    fn default() -> Mouse {
        Mouse::new()
    }
}

/// A snapshot of the mouse flags, x delta and y delta.
#[derive(Debug, Copy, Clone, Default)]
pub struct MouseState {
    flags: MouseFlags,
    x: i16,
    y: i16,
}

impl MouseState {
    /// Returns a new `MouseState`.
    pub const fn new() -> MouseState {
        MouseState {
            flags: MouseFlags::empty(),
            x: 0,
            y: 0,
        }
    }

    /// Returns true if the left mouse button is currently down.
    pub fn left_button_down(&self) -> bool {
        self.flags.contains(MouseFlags::LEFT_BUTTON)
    }

    /// Returns true if the left mouse button is currently up.
    pub fn left_button_up(&self) -> bool {
        !self.flags.contains(MouseFlags::LEFT_BUTTON)
    }

    /// Returns true if the right mouse button is currently down.
    pub fn right_button_down(&self) -> bool {
        self.flags.contains(MouseFlags::RIGHT_BUTTON)
    }

    /// Returns true if the right mouse button is currently up.
    pub fn right_button_up(&self) -> bool {
        !self.flags.contains(MouseFlags::RIGHT_BUTTON)
    }

    /// Returns true if the x axis has moved.
    pub fn x_moved(&self) -> bool {
        self.x != 0
    }

    /// Returns true if the y axis has moved.
    pub fn y_moved(&self) -> bool {
        self.y != 0
    }

    /// Returns true if the x or y axis has moved.
    pub fn moved(&self) -> bool {
        self.x_moved() || self.y_moved()
    }

    /// Returns the x delta of the mouse state.
    pub fn get_x(&self) -> i16 {
        self.x
    }

    /// Returns the y delta of the mouse state.
    pub fn get_y(&self) -> i16 {
        self.y
    }
}

impl Mouse {
    /// Creates a new `Mouse`.
    pub const fn new() -> Mouse {
        Mouse {
            command_port: Port::new(ADDRESS_PORT_ADDRESS),
            data_port: Port::new(DATA_PORT_ADDRESS),
            current_packet: 0,
            current_state: MouseState::new(),
            completed_state: MouseState::new(),
            on_complete: None,
        }
    }

    /// Returns the last completed state of the mouse.
    pub fn get_state(&self) -> MouseState {
        self.completed_state
    }

    /// Attempts to initialize a `Mouse`. If successful, interrupts will be generated
    /// as `PIC offset + 12`.
    pub fn init(&mut self) -> Result<(), &'static str> {
        self.write_command_port(GET_STATUS_BYTE)?;
        let status = self.read_data_port()? | 0x02;
        self.write_command_port(SET_STATUS_BYTE)?;
        self.write_data_port(status & 0xDF)?;
        self.send_command(Command::SetDefaults)?;
        self.send_command(Command::EnablePacketStreaming)?;
        Ok(())
    }

    /// Attempts to process a packet.
    pub fn process_packet(&mut self, packet: u8) {
        match self.current_packet {
            0 => {
                let flags = MouseFlags::from_bits_truncate(packet);
                if !flags.contains(MouseFlags::ALWAYS_ONE) {
                    return;
                }
                self.current_state.flags = flags;
            }
            1 => self.process_x_movement(packet),
            2 => {
                self.process_y_movement(packet);
                self.completed_state = self.current_state;
                if let Some(on_complete) = self.on_complete {
                    on_complete(self.completed_state);
                }
            }
            _ => unreachable!(),
        }
        self.current_packet = (self.current_packet + 1) % 3;
    }

    /// Sets the `on_complete` function to be called when a packet is completed.
    pub fn set_on_complete(&mut self, handler: fn(MouseState)) {
        self.on_complete = Some(handler);
    }

    fn process_x_movement(&mut self, packet: u8) {
        if !self.current_state.flags.contains(MouseFlags::X_OVERFLOW) {
            self.current_state.x = if self.current_state.flags.contains(MouseFlags::X_SIGN) {
                self.sign_extend(packet)
            } else {
                packet as i16
            };
        }
    }

    fn process_y_movement(&mut self, packet: u8) {
        if !self.current_state.flags.contains(MouseFlags::Y_OVERFLOW) {
            self.current_state.y = if self.current_state.flags.contains(MouseFlags::Y_SIGN) {
                self.sign_extend(packet)
            } else {
                packet as i16
            };
        }
    }

    fn read_data_port(&mut self) -> Result<u8, &'static str> {
        self.wait_for_read()?;
        Ok(unsafe { self.data_port.read() })
    }

    fn send_command(&mut self, command: Command) -> Result<(), &'static str> {
        self.write_command_port(0xD4)?;
        self.write_data_port(command as u8)?;
        if self.read_data_port()? != 0xFA {
            return Err("mouse did not respond to the command");
        }
        Ok(())
    }

    fn sign_extend(&self, packet: u8) -> i16 {
        ((packet as u16) | 0xFF00) as i16
    }

    fn write_command_port(&mut self, value: u8) -> Result<(), &'static str> {
        self.wait_for_write()?;
        unsafe {
            self.command_port.write(value);
        }
        Ok(())
    }

    fn write_data_port(&mut self, value: u8) -> Result<(), &'static str> {
        self.wait_for_write()?;
        unsafe {
            self.data_port.write(value);
        }
        Ok(())
    }

    fn wait_for_read(&mut self) -> Result<(), &'static str> {
        let timeout = 100_000;
        for _ in 0..timeout {
            let value = unsafe { self.command_port.read() };
            if (value & 0x1) == 0x1 {
                return Ok(());
            }
        }
        Err("wait for mouse read timeout")
    }

    fn wait_for_write(&mut self) -> Result<(), &'static str> {
        let timeout = 100_000;
        for _ in 0..timeout {
            let value = unsafe { self.command_port.read() };
            if (value & 0x2) == 0x0 {
                return Ok(());
            }
        }
        Err("wait for mouse write timeout")
    }
}
