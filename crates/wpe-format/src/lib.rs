//! Readers for the binary containers Wallpaper Engine ships its wallpapers in.
//!
//! The layouts here were derived from wallpaper files, not from any other
//! implementation. Everything the readers consume is untrusted input, so every
//! read is bounds-checked and every length is capped before it reaches an
//! allocation.

pub mod assets;
pub mod effect;
pub mod material;
pub mod pkg;
pub mod scene;
pub mod tex;

pub use assets::{Assets, Resolver};
pub use effect::Effect;
pub use material::{Material, Model};
pub use pkg::Package;
pub use scene::Scene;
pub use tex::Texture;

use std::fmt;

#[derive(Debug)]
pub enum Error {
    /// Ran out of bytes part-way through a field.
    Eof { need: usize, have: usize, at: usize },
    /// A container tag was not one this reader knows.
    UnknownTag { tag: String, at: usize },
    /// A container tag was not valid ASCII, or had no terminator.
    BadTag { at: usize },
    /// A count or dimension exceeded what a real wallpaper can hold.
    Absurd {
        what: &'static str,
        value: i64,
        at: usize,
    },
    /// Decompression produced a different number of bytes than declared.
    SizeMismatch { got: usize, want: usize },
    /// Decompression itself failed.
    Decompress(String),
    /// Bytes were left over after the last known container.
    Trailing { at: usize, left: usize },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Eof { need, have, at } => {
                write!(
                    f,
                    "unexpected end of data at byte {at}: need {need}, have {have}"
                )
            }
            Self::UnknownTag { tag, at } => write!(f, "unknown container tag {tag:?} at byte {at}"),
            Self::BadTag { at } => write!(f, "malformed container tag at byte {at}"),
            Self::Absurd { what, value, at } => {
                write!(f, "implausible {what} {value} at byte {at}")
            }
            Self::SizeMismatch { got, want } => {
                write!(f, "decompressed to {got} bytes, header declared {want}")
            }
            Self::Decompress(e) => write!(f, "decompression failed: {e}"),
            Self::Trailing { at, left } => {
                write!(f, "{left} unread bytes after last container, at byte {at}")
            }
        }
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

/// A bounds-checked cursor over a byte slice.
///
/// Every accessor either returns the value or an [`Error::Eof`] naming the
/// offset, so a truncated file reports where it ran out rather than panicking.
pub(crate) struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    pub(crate) fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    pub(crate) fn pos(&self) -> usize {
        self.pos
    }

    pub(crate) fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.remaining() == 0
    }

    pub(crate) fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self.pos.checked_add(n).ok_or(Error::Eof {
            need: n,
            have: self.remaining(),
            at: self.pos,
        })?;
        let out = self.data.get(self.pos..end).ok_or(Error::Eof {
            need: n,
            have: self.remaining(),
            at: self.pos,
        })?;
        self.pos = end;
        Ok(out)
    }

    pub(crate) fn i32(&mut self) -> Result<i32> {
        let b = self.take(4)?;
        Ok(i32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    pub(crate) fn u32(&mut self) -> Result<u32> {
        Ok(self.i32()? as u32)
    }

    pub(crate) fn f32(&mut self) -> Result<f32> {
        Ok(f32::from_bits(self.u32()?))
    }

    /// A NUL-terminated ASCII tag, as the containers use for their version marks.
    pub(crate) fn tag(&mut self) -> Result<&'a str> {
        let at = self.pos;
        let rest = &self.data[self.pos..];
        let end = rest
            .iter()
            .position(|&b| b == 0)
            .ok_or(Error::BadTag { at })?;
        let s = std::str::from_utf8(&rest[..end]).map_err(|_| Error::BadTag { at })?;
        if !s.is_ascii() {
            return Err(Error::BadTag { at });
        }
        self.pos += end + 1;
        Ok(s)
    }

    /// A length-prefixed string: a byte count, then that many bytes.
    ///
    /// This is how `.pkg` stores its strings. The `.tex` containers instead use
    /// NUL-terminated tags, so the two are deliberately separate calls.
    pub(crate) fn pstr(&mut self, max: i32) -> Result<String> {
        let len = self.count("string length", max)?;
        Ok(String::from_utf8_lossy(self.take(len)?).into_owned())
    }

    /// A count read from the file, rejected if it could not describe real data.
    pub(crate) fn count(&mut self, what: &'static str, max: i32) -> Result<usize> {
        let at = self.pos;
        let n = self.i32()?;
        if n < 0 || n > max {
            return Err(Error::Absurd {
                what,
                value: n as i64,
                at,
            });
        }
        Ok(n as usize)
    }
}
