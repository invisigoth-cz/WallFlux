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

    /// Looks where Steam usually puts it.
    ///
    /// ponytail: fixed list of the usual roots; parse `libraryfolders.vdf` if
    /// someone keeps their library on another drive and reports it missing.
    pub fn locate() -> Option<Self> {
        let home = std::env::var_os("HOME").map(PathBuf::from)?;
        const ROOTS: [&str; 3] = [
            ".steam/steam/steamapps/common/wallpaper_engine/assets",
            ".local/share/Steam/steamapps/common/wallpaper_engine/assets",
            ".var/app/com.valvesoftware.Steam/data/Steam/steamapps/common/wallpaper_engine/assets",
        ];
        ROOTS.iter().find_map(|r| Self::at(home.join(r)))
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
