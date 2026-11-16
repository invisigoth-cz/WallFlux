// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

//! Finding the files a wallpaper refers to but does not contain.
//!
//! Scene wallpapers name shaders, models and materials that ship with Wallpaper
//! Engine rather than with the wallpaper. Those files belong to Wallpaper
//! Engine and cannot be redistributed, so they are read from the viewer's own
//! installation. A wallpaper's own copy always wins, which is how a wallpaper
//! overrides a built-in.

use crate::Package;
use std::borrow::Cow;
use std::path::{Path, PathBuf};

/// Wallpaper Engine's `assets` directory.
pub struct Assets {
    root: PathBuf,
}

impl Assets {
    /// Uses the given `assets` directory, if it looks like one.
    pub fn at(root: impl Into<PathBuf>) -> Option<Self> {
        let root = root.into();
        root.join("shaders").is_dir().then_some(Self { root })
    }

    /// Looks where Steam usually puts it: in each usual Steam install, and in
    /// every library those installs list in `steamapps/libraryfolders.vdf` --
    /// a second library on another drive is where Wallpaper Engine may be.
    pub fn locate() -> Option<Self> {
        let home = std::env::var_os("HOME").map(PathBuf::from)?;
        const INSTALLS: [&str; 6] = [
            ".local/share/Steam",
            ".steam/steam",
            ".steam/root",
            ".var/app/com.valvesoftware.Steam/.local/share/Steam",
            ".var/app/com.valvesoftware.Steam/data/Steam",
            "Steam",
        ];
        INSTALLS
            .iter()
            .map(|install| home.join(install))
            .flat_map(|install| {
                let listed = std::fs::read_to_string(install.join("steamapps/libraryfolders.vdf"))
                    .map(|vdf| library_paths(&vdf))
                    .unwrap_or_default();
                std::iter::once(install).chain(listed)
            })
            .find_map(|library| Self::at(library.join("steamapps/common/wallpaper_engine/assets")))
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Reads a file by its package-relative path, rejecting anything that tries
    /// to climb out of the assets directory.
    pub fn get(&self, path: &str) -> Option<Vec<u8>> {
        let mut full = self.root.clone();
        for part in path.split('/') {
            if part.is_empty() || part == "." || part == ".." {
                return None;
            }
            full.push(part);
        }
        std::fs::read(full).ok()
    }
}

/// The libraries a `libraryfolders.vdf` lists: each `"path"  "..."` line, with
/// the backslash escapes Steam writes before a backslash or a quote undone.
fn library_paths(vdf: &str) -> Vec<PathBuf> {
    vdf.lines()
        .filter_map(|line| line.trim().strip_prefix("\"path\""))
        .filter_map(|rest| rest.trim().strip_prefix('"')?.strip_suffix('"'))
        .map(|quoted| {
            let mut path = String::new();
            let mut chars = quoted.chars();
            while let Some(c) = chars.next() {
                path.push(if c == '\\' { chars.next().unwrap_or(c) } else { c });
            }
            PathBuf::from(path)
        })
        .collect()
}

/// Resolves a reference against a wallpaper first, then the installation.
pub struct Resolver<'a> {
    package: &'a Package,
    assets: Option<&'a Assets>,
}

impl<'a> Resolver<'a> {
    pub fn new(package: &'a Package, assets: Option<&'a Assets>) -> Self {
        Self { package, assets }
    }

    pub fn get(&self, path: &str) -> Option<Cow<'a, [u8]>> {
        if let Some(bytes) = self.package.get(path) {
            return Some(Cow::Borrowed(bytes));
        }
        self.assets?.get(path).map(Cow::Owned)
    }

    /// Finds a shader's source, trying the wallpaper before the installation.
    /// `stage` is `"frag"` or `"vert"`.
    pub fn shader(&self, name: &str, stage: &str) -> Option<Cow<'a, [u8]>> {
        self.get(&format!("shaders/{name}.{stage}"))
    }

    pub fn has(&self, path: &str) -> bool {
        self.package.get(path).is_some() || self.assets.is_some_and(|a| a.get(path).is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_every_library_steam_lists() {
        let vdf = "\"libraryfolders\"\n{\n\t\"0\"\n\t{\n\t\t\"path\"\t\t\"/home/a/.local/share/Steam\"\n\t\t\"label\"\t\t\"\"\n\t}\n\t\"1\"\n\t{\n\t\t\"path\"\t\t\"/mnt/Games \\\"2\\\" \\\\x\"\n\t\t\"apps\"\n\t\t{\n\t\t\t\"431960\"\t\t\"1\"\n\t\t}\n\t}\n}\n";
        assert_eq!(
            library_paths(vdf),
            [PathBuf::from("/home/a/.local/share/Steam"), PathBuf::from("/mnt/Games \"2\" \\x")]
        );
    }

    #[test]
    fn rejects_paths_climbing_out_of_the_assets_directory() {
        let Some(assets) = Assets::locate() else {
            return;
        };
        assert!(assets.get("../config.json").is_none());
        assert!(assets.get("shaders/../../config.json").is_none());
        assert!(assets.get("").is_none());
    }

    #[test]
    fn finds_a_built_in_shader_when_an_installation_is_present() {
        let Some(assets) = Assets::locate() else {
            return;
        };
        assert!(assets.get("shaders/genericimage2.frag").is_some());
    }
}
