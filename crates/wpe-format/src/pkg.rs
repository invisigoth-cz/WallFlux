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
    /// For a package opened rather than read: the file, with `entries` as
    /// places in it, and each entry once it has been asked for.
    file: Option<std::fs::File>,
    loaded: BTreeMap<String, std::cell::OnceCell<Option<Vec<u8>>>>,
}

/// What the front of a package says: its version, each entry's name, offset
/// and size, and where the payload the offsets count from begins. `bytes`
/// needs to hold no more than the table.
type Table = (String, Vec<(String, usize, usize)>, usize);

fn table(bytes: &[u8]) -> Result<Table> {
    let mut r = Reader::new(bytes);
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
    // Offsets are relative to here.
    Ok((version, table, r.pos()))
}

/// One entry of the package at `path`, read without the rest of it, or
/// nothing where the package has no such entry.
///
/// A scene's package runs to a few hundred megabytes -- 277 MB for the
/// largest here -- and asking whether it carries a sound needs only its
/// scene.json, so the table is read off the front, a quarter of a megabyte
/// at a time until it is all there, and then the one entry.
pub fn read_entry(path: impl AsRef<std::path::Path>, name: &str) -> std::io::Result<Option<Vec<u8>>> {
    use std::io::{Read, Seek, SeekFrom};
    let invalid = |e: Error| std::io::Error::new(std::io::ErrorKind::InvalidData, e);

    let mut file = std::fs::File::open(path)?;
    let length = file.metadata()?.len();
    let (_, table, base) = front(&mut file, length)?;
    let Some(&(_, offset, size)) = table.iter().find(|(entry, _, _)| entry == name) else {
        return Ok(None);
    };
    let start = (base + offset) as u64;
    if start + size as u64 > length {
        return Err(invalid(Error::Eof {
            need: base + offset + size,
            have: length as usize,
            at: base,
        }));
    }
    file.seek(SeekFrom::Start(start))?;
    let mut entry = vec![0; size];
    file.read_exact(&mut entry)?;
    Ok(Some(entry))
}

/// The table off the front of `file`, a quarter of a megabyte at a time
/// until it is all there.
fn front(file: &mut std::fs::File, length: u64) -> std::io::Result<Table> {
    use std::io::{Read, Seek, SeekFrom};
    let mut front = Vec::new();
    let mut wanted: u64 = 256 * 1024;
    loop {
        front.clear();
        file.seek(SeekFrom::Start(0))?;
        file.take(wanted).read_to_end(&mut front)?;
        match table(&front) {
            Ok(found) => return Ok(found),
            Err(Error::Eof { .. }) if wanted < length => wanted *= 4,
            Err(e) => return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
        }
    }
}

impl Package {
    /// The package at `path` with only its table read, and each entry read
    /// off the disk the first time it is asked for.
    ///
    /// A scene uses a part of what its package carries, and reading all of
    /// it was most of what loading a scene again cost after its textures:
    /// 105 ms of DOOM's 190, for 290 MB of which the scene reads a fraction.
    pub fn open(path: impl AsRef<std::path::Path>) -> std::io::Result<Self> {
        let invalid = |e: Error| std::io::Error::new(std::io::ErrorKind::InvalidData, e);
        let mut file = std::fs::File::open(path)?;
        let length = file.metadata()?.len() as usize;
        let (version, table, base) = front(&mut file, length as u64)?;
        let mut entries = BTreeMap::new();
        let mut loaded = BTreeMap::new();
        for (name, offset, size) in table {
            let end = base
                .checked_add(offset)
                .and_then(|start| start.checked_add(size))
                .ok_or_else(|| invalid(Error::Absurd { what: "entry extent", value: offset as i64, at: base }))?;
            if end > length {
                return Err(invalid(Error::Eof { need: end, have: length, at: base }));
            }
            loaded.insert(name.clone(), std::cell::OnceCell::new());
            entries.insert(name, (base + offset)..end);
        }
        Ok(Self { version, data: Vec::new(), entries, file: Some(file), loaded })
    }

    pub fn parse(bytes: Vec<u8>) -> Result<Self> {
        let (version, table, base) = table(&bytes)?;
        // The payload is whatever is left after the table.
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
            file: None,
            loaded: BTreeMap::new(),
        })
    }

    pub fn read(path: impl AsRef<std::path::Path>) -> std::io::Result<Self> {
        let bytes = std::fs::read(path)?;
        Self::parse(bytes).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    /// A wallpaper that was never packed, the way the ones Wallpaper Engine
    /// ships are laid out: its files as they would be named inside a package.
    pub fn from_folder(dir: impl AsRef<std::path::Path>) -> std::io::Result<Self> {
        fn walk(
            dir: &std::path::Path,
            prefix: &str,
            data: &mut Vec<u8>,
            entries: &mut BTreeMap<String, Range<usize>>,
        ) -> std::io::Result<()> {
            for entry in std::fs::read_dir(dir)? {
                let entry = entry?;
                let name = format!("{prefix}{}", entry.file_name().to_string_lossy());
                if entry.file_type()?.is_dir() {
                    walk(&entry.path(), &format!("{name}/"), data, entries)?;
                } else {
                    let start = data.len();
                    data.extend(std::fs::read(entry.path())?);
                    entries.insert(name, start..data.len());
                }
            }
            Ok(())
        }
        let (mut data, mut entries) = (Vec::new(), BTreeMap::new());
        walk(dir.as_ref(), "", &mut data, &mut entries)?;
        Ok(Self { version: "folder".into(), data, entries, file: None, loaded: BTreeMap::new() })
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn get(&self, name: &str) -> Option<&[u8]> {
        let range = self.entries.get(name)?;
        let Some(file) = &self.file else {
            return Some(&self.data[range.clone()]);
        };
        self.loaded
            .get(name)?
            .get_or_init(|| {
                use std::os::unix::fs::FileExt;
                let mut entry = vec![0; range.len()];
                match file.read_exact_at(&mut entry, range.start as u64) {
                    Ok(()) => Some(entry),
                    Err(e) => {
                        eprintln!("wpe-format: {name} could not be read from its package -- {e}");
                        None
                    }
                }
            })
            .as_deref()
    }

    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.entries.keys().map(String::as_str)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &[u8])> {
        self.entries.keys().filter_map(|name| Some((name.as_str(), self.get(name)?)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_folder_reads_as_a_package_of_its_files() {
        let dir = std::env::temp_dir().join(format!("wpe-from-folder-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("shaders/effects")).unwrap();
        std::fs::write(dir.join("scene.json"), b"{}").unwrap();
        std::fs::write(dir.join("shaders/effects/a.frag"), b"abc").unwrap();
        let package = Package::from_folder(&dir).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(package.get("scene.json"), Some(&b"{}"[..]));
        assert_eq!(package.get("shaders/effects/a.frag"), Some(&b"abc"[..]));
    }

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

    /// The table is read a piece at a time, so one longer than the first
    /// piece has to be read whole before the entry is found -- and the entry
    /// has to be the same bytes a whole read of the package finds.
    #[test]
    fn reads_one_entry_past_a_table_longer_than_the_first_read() {
        let names: Vec<String> = (0..4000).map(|i| format!("materials/{i:0>90}.json")).collect();
        let mut v = Vec::new();
        v.extend_from_slice(&8i32.to_le_bytes());
        v.extend_from_slice(b"PKGV0019");
        v.extend_from_slice(&(names.len() as i32).to_le_bytes());
        for (i, name) in names.iter().enumerate() {
            v.extend_from_slice(&(name.len() as i32).to_le_bytes());
            v.extend_from_slice(name.as_bytes());
            v.extend_from_slice(&(i as i32 * 2).to_le_bytes());
            v.extend_from_slice(&2i32.to_le_bytes());
        }
        assert!(v.len() > 256 * 1024);
        for i in 0..names.len() {
            v.extend_from_slice(&(i as u16).to_le_bytes());
        }
        let path = std::env::temp_dir().join(format!("wpe-format-{}.pkg", std::process::id()));
        std::fs::write(&path, &v).unwrap();

        let last = names.last().unwrap();
        let read = read_entry(&path, last).unwrap();
        let absent = read_entry(&path, "scene.json").unwrap();
        let opened = Package::open(&path).unwrap();
        std::fs::remove_file(&path).unwrap();

        let whole = Package::parse(v).unwrap();
        assert_eq!(read.as_deref(), whole.get(last));
        assert_eq!(read.as_deref(), Some(&3999u16.to_le_bytes()[..]));
        assert_eq!(absent, None);
        // Opened, every entry is the bytes a whole read finds, read off a
        // file that is no longer even there by name.
        assert_eq!(opened.version(), "PKGV0019");
        assert!(opened.iter().eq(whole.iter()));
        assert_eq!(opened.get("scene.json"), None);
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
