use crate::syscall;

use alloc::string::{String, ToString};
use alloc::vec;
use core2::io::ErrorKind;

pub struct Stdin;
pub struct Stdout;
pub struct Stderr;

impl Stdin {
    fn new() -> Self {
        Self {}
    }

    pub fn read(&self, buf: &mut [u8]) -> Result<usize, ErrorKind> {
        match syscall::read(0, buf) {
            Some(res) => Ok(res),
            None => Err(ErrorKind::Interrupted),
        }
    }

    pub fn read_char(&self) -> Option<char> {
        let mut buf = vec![0; 4];
        if let Some(bytes) = syscall::read(0, &mut buf) {
            if bytes > 0 {
                buf.resize(bytes, 0);
                return Some(String::from_utf8_lossy(&buf).to_string().remove(0));
            }
        }
        None
    }

    pub fn read_line(&self) -> String {
        let mut buf = vec![0; 256];
        if let Some(bytes) = syscall::read(0, &mut buf) {
            buf.resize(bytes, 0);
            String::from_utf8_lossy(&buf).to_string()
        } else {
            String::new()
        }
    }

    pub fn read_to_string(&self) -> String {
        let mut buf = vec![0; 256];
        if let Some(bytes) = syscall::read(0, &mut buf) {
            buf.resize(bytes, 0);
            String::from_utf8_lossy(&buf).to_string()
        } else {
            String::new()
        }
    }

    pub fn read_exact(&self, mut buf: &mut [u8]) -> Result<(), ErrorKind> {
        while !buf.is_empty() {
            match self.read(buf) {
                Ok(0) => break,
                Ok(n) => {
                    let tmp = buf;
                    buf = &mut tmp[n..];
                }
                Err(ref e) if e == &ErrorKind::Interrupted => {}
                Err(e) => return Err(e),
            }
        }
        if !buf.is_empty() {
            Err(ErrorKind::UnexpectedEof)
        } else {
            Ok(())
        }
    }
}

impl Stdout {
    fn new() -> Self {
        Self {}
    }

    pub fn write(&self, s: &str) {
        syscall::write(1, s.as_bytes());
    }

    pub fn write_all(&self, s: &[u8]) -> Result<usize, ()> {
        match syscall::write(1, s) {
            Some(res) => Ok(res),
            None => Err(()),
        }
    }
}

impl Stderr {
    fn new() -> Self {
        Self {}
    }

    pub fn write(&self, s: &str) {
        syscall::write(2, s.as_bytes());
    }

    pub fn write_all(&self, s: &[u8]) -> Result<usize, ()> {
        match syscall::write(2, s) {
            Some(res) => Ok(res),
            None => Err(()),
        }
    }
}

pub fn stdin() -> Stdin {
    Stdin::new()
}

pub fn empty() -> Stdin {
    Stdin::new()
}

pub fn stdout() -> Stdout {
    Stdout::new()
}

pub fn sync() -> Stdout {
    Stdout::new()
}

pub fn stderr() -> Stderr {
    Stderr::new()
}