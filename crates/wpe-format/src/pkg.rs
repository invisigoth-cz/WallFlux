// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

//! The `.pkg` archive a scene wallpaper ships in.
//!
//! Layout: a version tag, an entry count, then that many `(name, offset, size)`
//! records. The offsets are relative to the end of the table, where the payload
//! begins. Tags seen in the wild run from `PKGV0001` to `PKGV0023`; the layout
//! has not changed across them, so the version is recorded but not branched on.

use crate::{Error, Reader, Result};
use std::collections::BTreeMap;
use std::ops::Range;

/// Entry counts and name lengths above these cannot describe a real wallpaper,
/// and are rejected before they reach an allocation.
const MAX_ENTRIES: i32 = 65_536;
const MAX_NAME_LEN: i32 = 4096;

pub struct Package {
    version: String,
    data: Vec<u8>,
    entries: BTreeMap<String, Range<usize>>,
}

impl Package {
    pub fn parse(bytes: Vec<u8>) -> Result<Self> {
        let mut r = Reader::new(&bytes);
        let version = r.pstr(MAX_NAME_LEN)?;
        if !version.starts_with("PKGV") {
            return Err(Error::UnknownTag {
                tag: version,
                at: 0,
            });
        }

        let count = r.count("package entry count", MAX_ENTRIES)?;
        let mut table = Vec::with_capacity(count.min(1024));
        for _ in 0..count {
            let name = r.pstr(MAX_NAME_LEN)?;
            let at = r.pos();
            let offset = r.i32()?;
            let size = r.i32()?;
            if offset < 0 || size < 0 {
                return Err(Error::Absurd {
                    what: "entry extent",
                    value: offset as i64,
                    at,
                });
            }
            table.push((name, offset as usize, size as usize));
        }

        // Offsets are relative to here, so the payload is whatever is left.
        let base = r.pos();
        let payload = bytes.len() - base;

        let mut entries = BTreeMap::new();
        for (name, offset, size) in table {
            let end = offset.checked_add(size).ok_or(Error::Absurd {
                what: "entry extent",
                value: offset as i64,
                at: base,
            })?;
            if end > payload {
                return Err(Error::Eof {
                    need: end,
                    have: payload,
                    at: base,
                });
            }
            entries.insert(name, (base + offset)..(base + end));
        }

        Ok(Self {
            version,
            data: bytes,
            entries,
        })
    }

    pub fn read(path: impl AsRef<std::path::Path>) -> std::io::Result<Self> {
        let bytes = std::fs::read(path)?;
        Self::parse(bytes).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn get(&self, name: &str) -> Option<&[u8]> {
        self.entries.get(name).map(|r| &self.data[r.clone()])
    }

    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.entries.keys().map(String::as_str)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &[u8])> {
        self.entries
            .iter()
            .map(|(name, range)| (name.as_str(), &self.data[range.clone()]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds the smallest archive the reader should accept: one named entry.
    fn archive(tag: &str, name: &str, payload: &[u8]) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(&(tag.len() as i32).to_le_bytes());
        v.extend_from_slice(tag.as_bytes());
        v.extend_from_slice(&1i32.to_le_bytes());
        v.extend_from_slice(&(name.len() as i32).to_le_bytes());
        v.extend_from_slice(name.as_bytes());
        v.extend_from_slice(&0i32.to_le_bytes());
        v.extend_from_slice(&(payload.len() as i32).to_le_bytes());
        v.extend_from_slice(payload);
        v
    }

    #[test]
    fn reads_an_entry() {
        let p = Package::parse(archive("PKGV0001", "scene.json", b"{}")).unwrap();
        assert_eq!(p.version(), "PKGV0001");
        assert_eq!(p.get("scene.json"), Some(&b"{}"[..]));
        assert_eq!(p.get("absent"), None);
    }

    #[test]
    fn rejects_entry_reaching_past_the_payload() {
        let mut v = archive("PKGV0001", "a", b"xx");
        let n = v.len();
        v[n - 6..n - 2].copy_from_slice(&99i32.to_le_bytes()); // size 99, payload 2
        assert!(matches!(Package::parse(v), Err(Error::Eof { .. })));
    }

    #[test]
    fn rejects_absurd_entry_count() {
        let mut v = Vec::new();
        v.extend_from_slice(&8i32.to_le_bytes());
        v.extend_from_slice(b"PKGV0001");
        v.extend_from_slice(&i32::MAX.to_le_bytes());
        assert!(matches!(Package::parse(v), Err(Error::Absurd { .. })));
    }

    #[test]
    fn rejects_truncated_header() {
        let mut v = Vec::new();
        v.extend_from_slice(&8i32.to_le_bytes());
        v.extend_from_slice(b"PKGV0001");
        v.push(1);
        assert!(Package::parse(v).is_err());
    }

    #[test]
    fn rejects_foreign_container() {
        let mut v = Vec::new();
        v.extend_from_slice(&7i32.to_le_bytes());
        v.extend_from_slice(b"NOTAPKG");
        v.extend_from_slice(&0i32.to_le_bytes());
        assert!(matches!(Package::parse(v), Err(Error::UnknownTag { .. })));
    }

    /// The version is length-prefixed, not NUL-terminated. Reading it the other
    /// way yields the length byte as the tag, which is how this was first found.
    #[test]
    fn version_is_length_prefixed() {
        let v = archive("PKGV0023", "a", b"x");
        assert_eq!(&v[..4], &8i32.to_le_bytes());
        assert_eq!(Package::parse(v).unwrap().version(), "PKGV0023");
    }
}
