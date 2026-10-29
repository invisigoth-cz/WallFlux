// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

//! The sound a scene plays, unpacked for something else to play it.
//!
//! The renderer draws and does not play: the program around it already has
//! a media player, the one video wallpapers use. What it lacks is a way into
//! `scene.pkg`, which is where a scene keeps its music. So this reads the
//! scene's sound objects, writes their files out beside each other in a
//! folder the caller names, and says how each object wants them played.

use std::path::{Component, Path, PathBuf};

use wpe_format::scene::ObjectKind;
use wpe_format::{Package, Scene as SceneFile};

/// One sound object, ready to be played.
#[derive(Debug, PartialEq)]
pub struct Sound {
    /// The files written out, in the order the scene lists them.
    pub files: Vec<PathBuf>,
    pub mode: String,
    pub volume: f32,
    pub mintime: f32,
    pub maxtime: f32,
    pub startsilent: bool,
}

/// Writes the files of every sound object the scene in `folder` plays into
/// `cache`, and says how each is to be played.
///
/// A file already there at the same size is left alone, so choosing the same
/// wallpaper again writes nothing. A hidden sound object -- one a setting has
/// switched off -- is left out.
pub fn unpack(folder: &Path, cache: &Path) -> Result<Vec<Sound>, String> {
    let package = Package::read(crate::render::find_package(folder)?)
        .map_err(|e| format!("reading package: {e}"))?;
    let project = std::fs::read(folder.join("project.json")).ok();
    let entry = crate::render::scene_entry(project.as_deref());
    let scene = package.get(&entry).ok_or_else(|| format!("no {entry} in this wallpaper"))?;
    let scene = SceneFile::parse_with_properties(scene, project.as_deref())
        .map_err(|e| format!("scene.json: {e}"))?;

    let mut sounds = Vec::new();
    for object in &scene.objects {
        let ObjectKind::Sound(names) = object.kind() else {
            continue;
        };
        if !object.visible.value {
            continue;
        }
        let mut files = Vec::new();
        for name in names {
            let Some(inside) = safely_inside(cache, &name) else {
                eprintln!("wallflux scene: a sound named {name:?} would land outside the cache");
                continue;
            };
            let Some(bytes) = package.get(&name) else {
                eprintln!("wallflux scene: {name} is not in the package");
                continue;
            };
            let unchanged = std::fs::metadata(&inside).is_ok_and(|m| m.len() == bytes.len() as u64);
            if !unchanged {
                if let Some(parent) = inside.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
                }
                std::fs::write(&inside, bytes).map_err(|e| format!("{}: {e}", inside.display()))?;
            }
            files.push(inside);
        }
        if files.is_empty() {
            continue;
        }
        sounds.push(Sound {
            files,
            mode: object.playbackmode.clone(),
            volume: object.volume.value.clamp(0.0, 1.0),
            mintime: object.mintime.max(0.0),
            maxtime: object.maxtime.max(object.mintime.max(0.0)),
            startsilent: object.startsilent,
        });
    }
    Ok(sounds)
}

/// Whether the scene in `folder` plays a sound of its own -- what `unpack`
/// would find, asked without reading the package whole or writing anything.
pub fn carries_sound(folder: &Path) -> Result<bool, String> {
    let package = crate::render::find_package(folder)?;
    let project = std::fs::read(folder.join("project.json")).ok();
    let entry = crate::render::scene_entry(project.as_deref());
    let scene = wpe_format::pkg::read_entry(&package, &entry)
        .map_err(|e| format!("reading package: {e}"))?
        .ok_or_else(|| format!("no {entry} in this wallpaper"))?;
    let scene = SceneFile::parse_with_properties(&scene, project.as_deref())
        .map_err(|e| format!("scene.json: {e}"))?;
    Ok(scene.objects.iter().any(|object| {
        object.visible.value
            && matches!(object.kind(), ObjectKind::Sound(names) if !names.is_empty())
    }))
}

/// Where `name` goes under `cache`, or nothing if it would go anywhere else.
///
/// The names come out of somebody's package off the internet; one written
/// as `../../.bashrc` is not a sound.
fn safely_inside(cache: &Path, name: &str) -> Option<PathBuf> {
    let relative = Path::new(name);
    let plain = relative.components().all(|c| matches!(c, Component::Normal(_)));
    (plain && !name.is_empty()).then(|| cache.join(relative))
}

/// The sounds as JSON, which is what crosses into C++.
pub fn to_json(sounds: &[Sound]) -> String {
    let list: Vec<_> = sounds
        .iter()
        .map(|s| {
            serde_json::json!({
                "files": s.files.iter().map(|f| f.to_string_lossy()).collect::<Vec<_>>(),
                "mode": s.mode,
                "volume": s.volume,
                "mintime": s.mintime,
                "maxtime": s.maxtime,
                "startsilent": s.startsilent,
            })
        })
        .collect();
    serde_json::Value::Array(list).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_that_climbs_out_of_the_cache_is_refused() {
        let cache = Path::new("/tmp/cache");
        assert_eq!(
            safely_inside(cache, "sounds/a.mp3"),
            Some(PathBuf::from("/tmp/cache/sounds/a.mp3"))
        );
        assert_eq!(safely_inside(cache, "../a.mp3"), None);
        assert_eq!(safely_inside(cache, "sounds/../../a.mp3"), None);
        assert_eq!(safely_inside(cache, "/etc/passwd"), None);
        assert_eq!(safely_inside(cache, ""), None);
    }
}
