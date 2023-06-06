use core::marker::PhantomData;

#[cfg(feature = "x86_64")]
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
mod x86;

#[cfg(feature = "x86_64")]
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub use x86::{inb, inl, inw, outb, outl, outw};

pub trait PortIn {
    unsafe fn port_in(port: u16) -> Self;
}

pub trait PortOut {
    unsafe fn port_out(port: u16, value: Self);
}

impl PortOut for u8 {
    unsafe fn port_out(port: u16, value: Self) {
        outb(value, port);
    }
}
impl PortOut for u16 {
    unsafe fn port_out(port: u16, value: Self) {
        outw(value, port);
    }
}
impl PortOut for u32 {
    unsafe fn port_out(port: u16, value: Self) {
        outl(value, port);
    }
}

impl PortIn for u8 {
    unsafe fn port_in(port: u16) -> Self {
        inb(port)
    }
}
impl PortIn for u16 {
    unsafe fn port_in(port: u16) -> Self {
        inw(port)
    }
}
impl PortIn for u32 {
    unsafe fn port_in(port: u16) -> Self {
        inl(port)
    }
}

#[derive(Debug)]
pub struct Port<T: PortIn + PortOut> {
    port: u16,
    _phantom: PhantomData<T>,
}
impl<T: PortIn + PortOut> Port<T> {
    pub const fn new(port: u16) -> Port<T> {
        Port {
            port: port,
            _phantom: PhantomData,
        }
    }

    pub const fn port_address(&self) -> u16 {
        self.port
    }

    pub fn read(&self) -> T {
        unsafe { T::port_in(self.port) }
    }

    pub unsafe fn write(&self, value: T) {
        T::port_out(self.port, value);
    }
}

#[derive(Debug)]
pub struct PortReadOnly<T: PortIn> {
    port: u16,
    _phantom: PhantomData<T>,
}
impl<T: PortIn> PortReadOnly<T> {
    pub const fn new(port: u16) -> PortReadOnly<T> {
        PortReadOnly {
            port: port,
            _phantom: PhantomData,
        }
    }

    pub const fn port_address(&self) -> u16 {
        self.port
    }

    pub fn read(&self) -> T {
        unsafe { T::port_in(self.port) }
    }
}

#[derive(Debug)]
pub struct PortWriteOnly<T: PortOut> {
    port: u16,
    _phantom: PhantomData<T>,
}
impl<T: PortOut> PortWriteOnly<T> {
    pub const fn new(port: u16) -> PortWriteOnly<T> {
        PortWriteOnly {
            port: port,
            _phantom: PhantomData,
        }
    }

    pub const fn port_address(&self) -> u16 {
        self.port
    }

    pub unsafe fn write(&self, value: T) {
        T::port_out(self.port, value);
    }
}
