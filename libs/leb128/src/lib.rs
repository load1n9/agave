// ported from https://github.com/gimli-rs/leb128/blob/master/src/lib.rs
#![no_std]
#[doc(hidden)]
pub const CONTINUATION_BIT: u8 = 1 << 7;
#[doc(hidden)]
pub const SIGN_BIT: u8 = 1 << 6;

#[doc(hidden)]
#[inline]
pub fn low_bits_of_byte(byte: u8) -> u8 {
    byte & !CONTINUATION_BIT
}

#[doc(hidden)]
#[inline]
pub fn low_bits_of_u64(val: u64) -> u8 {
    let byte = val & (u8::MAX as u64);
    low_bits_of_byte(byte as u8)
}

pub mod read {
    use super::{low_bits_of_byte, CONTINUATION_BIT, SIGN_BIT};

    #[derive(Debug)]
    pub enum Error {
        IoError(core2::io::Error),
        Overflow,
    }

    impl From<core2::io::Error> for Error {
        fn from(e: core2::io::Error) -> Self {
            Error::IoError(e)
        }
    }

    impl core::fmt::Display for Error {
        fn fmt(&self, f: &mut core::fmt::Formatter) -> Result<(), core::fmt::Error> {
            match *self {
                Error::IoError(ref e) => e.fmt(f),
                Error::Overflow => {
                    write!(f, "The number being read is larger than can be represented")
                }
            }
        }
    }

    impl core2::error::Error for Error {
        fn source(&self) -> Option<&(dyn core2::error::Error + 'static)> {
            match *self {
                Error::IoError(ref e) => Some(e),
                Error::Overflow => None,
            }
        }
    }

    pub fn unsigned<R>(r: &mut R) -> Result<u64, Error>
    where
        R: ?Sized + core2::io::Read,
    {
        let mut result = 0;
        let mut shift = 0;

        loop {
            let mut buf = [0];
            r.read_exact(&mut buf)?;

            if shift == 63 && buf[0] != 0x00 && buf[0] != 0x01 {
                while buf[0] & CONTINUATION_BIT != 0 {
                    r.read_exact(&mut buf)?;
                }
                return Err(Error::Overflow);
            }

            let low_bits = low_bits_of_byte(buf[0]) as u64;
            result |= low_bits << shift;

            if buf[0] & CONTINUATION_BIT == 0 {
                return Ok(result);
            }

            shift += 7;
        }
    }

    pub fn signed<R>(r: &mut R) -> Result<i64, Error>
    where
        R: ?Sized + core2::io::Read,
    {
        let mut result = 0;
        let mut shift = 0;
        let size = 64;
        let mut byte;

        loop {
            let mut buf = [0];
            r.read_exact(&mut buf)?;

            byte = buf[0];
            if shift == 63 && byte != 0x00 && byte != 0x7f {
                while buf[0] & CONTINUATION_BIT != 0 {
                    r.read_exact(&mut buf)?;
                }
                return Err(Error::Overflow);
            }

            let low_bits = low_bits_of_byte(byte) as i64;
            result |= low_bits << shift;
            shift += 7;

            if byte & CONTINUATION_BIT == 0 {
                break;
            }
        }

        if shift < size && (SIGN_BIT & byte) == SIGN_BIT {
            // Sign extend the result.
            result |= !0 << shift;
        }

        Ok(result)
    }
}

pub mod write {
    use super::{low_bits_of_u64, CONTINUATION_BIT};
    pub fn unsigned<W>(w: &mut W, mut val: u64) -> Result<usize, core2::io::Error>
    where
        W: ?Sized + core2::io::Write,
    {
        let mut bytes_written = 0;
        loop {
            let mut byte = low_bits_of_u64(val);
            val >>= 7;
            if val != 0 {
                byte |= CONTINUATION_BIT;
            }

            let buf = [byte];
            w.write_all(&buf)?;
            bytes_written += 1;

            if val == 0 {
                return Ok(bytes_written);
            }
        }
    }

    pub fn signed<W>(w: &mut W, mut val: i64) -> Result<usize, core2::io::Error>
    where
        W: ?Sized + core2::io::Write,
    {
        let mut bytes_written = 0;
        loop {
            let mut byte = val as u8;
            val >>= 6;
            let done = val == 0 || val == -1;
            if done {
                byte &= !CONTINUATION_BIT;
            } else {
                val >>= 1;
                byte |= CONTINUATION_BIT;
            }

            let buf = [byte];
            w.write_all(&buf)?;
            bytes_written += 1;

            if done {
                return Ok(bytes_written);
            }
        }
    }
}
