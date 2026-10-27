//! Walks a directory of wallpapers, reads every package and texture in it, and
//! reports what was found.
//!
//! With a second argument, also writes each texture's full-size surface there
//! as a PNG, which is how the readers get checked against something a person
//! can look at.
//!
//! ```text
//! cargo run --example dump -- ~/.steam/steam/steamapps/workshop/content/431960 /tmp/out
//! ```

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use wpe_format::tex::{FreeImage, PixelFormat};
use wpe_format::{Package, Texture};

fn main() {
    let mut args = std::env::args_os().skip(1);
    let Some(root) = args.next().map(PathBuf::from) else {
        eprintln!("usage: dump <wallpaper-dir> [png-out-dir]");
        std::process::exit(2);
    };
    let out = args.next().map(PathBuf::from);
    if let Some(dir) = &out
        && let Err(e) = std::fs::create_dir_all(dir)
    {
        eprintln!("cannot create {}: {e}", dir.display());
        std::process::exit(1);
    }

    let mut packages = 0usize;
    let mut textures = 0usize;
    let mut textures_ok = 0usize;
    let mut failures = Vec::new();
    let mut written = 0usize;
    let mut skipped: BTreeMap<String, usize> = BTreeMap::new();
    let mut formats: BTreeMap<String, usize> = BTreeMap::new();
    let mut blocks: BTreeMap<String, usize> = BTreeMap::new();
    let mut sprites: BTreeMap<String, usize> = BTreeMap::new();

    for pkg_path in packages_under(&root) {
        let package = match Package::read(&pkg_path) {
            Ok(p) => p,
            Err(e) => {
                failures.push((pkg_path.display().to_string(), e.to_string()));
                continue;
            }
        };
        packages += 1;
        let wallpaper = pkg_path
            .parent()
            .and_then(|p| p.file_name())
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();

        for (name, bytes) in package.iter() {
            if !name.ends_with(".tex") {
                continue;
            }
            textures += 1;
            let texture = match Texture::parse(bytes) {
                Ok(t) => {
                    textures_ok += 1;
                    t
                }
                Err(e) => {
                    failures.push((format!("{wallpaper}/{name}"), e.to_string()));
                    continue;
                }
            };
            *formats.entry(format!("{:?}", texture.format)).or_default() += 1;
            *blocks.entry(texture.block_container.clone()).or_default() += 1;
            if let Some(s) = &texture.sprite {
                *sprites.entry(s.version.clone()).or_default() += 1;
            }

            let Some(dir) = &out else { continue };
            let stem = format!("{wallpaper}_{}", name.replace(['/', '\\'], "_"));
            match write_surface(dir, &stem, &texture) {
                Ok(true) => written += 1,
                Ok(false) => *skipped.entry(format!("{:?}", texture.format)).or_default() += 1,
                Err(e) => failures.push((format!("{wallpaper}/{name}"), e)),
            }
        }
    }

    println!("packages read   {packages}");
    println!("textures read   {textures_ok}/{textures}");
    println!("block versions  {blocks:?}");
    println!("pixel formats   {formats:?}");
    println!("sprite sheets   {sprites:?}");
    if out.is_some() {
        println!("images written  {written}");
        if !skipped.is_empty() {
            println!("not yet decoded {skipped:?}");
        }
    }
    if failures.is_empty() {
        println!("\nno failures");
    } else {
        println!("\n{} failures:", failures.len());
        for (what, why) in failures.iter().take(20) {
            println!("  {what}: {why}");
        }
        std::process::exit(1);
    }
}

fn packages_under(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "pkg") {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

/// Writes the texture's full-size surface. Returns `false` for formats that
/// have no decoder yet.
fn write_surface(dir: &Path, stem: &str, texture: &Texture) -> Result<bool, String> {
    let Some(mip) = texture.base() else {
        return Ok(false);
    };
    let data = mip.decompress().map_err(|e| e.to_string())?;

    // A scene can hold video as a texture; hand that over as the video it is.
    if texture.is_video() {
        std::fs::write(dir.join(format!("{stem}.mp4")), data.as_ref())
            .map_err(|e| e.to_string())?;
        return Ok(true);
    }

    // An encoded image is already a file; hand it over untouched so what lands
    // on disk is the wallpaper's own bytes rather than anything this re-encoded.
    let encoded = match texture.free_image {
        FreeImage::Jpeg => Some("jpg"),
        FreeImage::Png => Some("png"),
        FreeImage::Bmp => Some("bmp"),
        _ => None,
    };
    if let Some(ext) = encoded {
        std::fs::write(dir.join(format!("{stem}.{ext}")), data.as_ref())
            .map_err(|e| e.to_string())?;
        return Ok(true);
    }

    let (w, h) = (mip.width, mip.height);
    // Block-compressed surfaces are decoded only to look at them. The renderer
    // uploads these to the GPU still compressed, which is the point of them.
    //
    // Checked against ffmpeg by wrapping the same blocks in a DDS container: of
    // 36 textures, 2 matched byte for byte and the rest never drifted by more
    // than 1/255, which is where two correct interpolators round differently.
    let block = match texture.format {
        PixelFormat::Dxt1 => Some(texpresso::Format::Bc1),
        PixelFormat::Dxt3 => Some(texpresso::Format::Bc2),
        PixelFormat::Dxt5 => Some(texpresso::Format::Bc3),
        _ => None,
    };
    let rgba = match (block, texture.format) {
        (Some(format), _) => {
            let mut out = vec![0u8; (w as usize) * (h as usize) * 4];
            format.decompress(&data, w as usize, h as usize, &mut out);
            out
        }
        (None, PixelFormat::Argb8888) => data.into_owned(),
        (None, PixelFormat::R8) => data.iter().flat_map(|&v| [v, v, v, 255]).collect(),
        (None, PixelFormat::Rg88) => {
            let (pairs, _) = data.as_chunks::<2>();
            pairs.iter().flat_map(|&[r, g]| [r, g, 0, 255]).collect()
        }
        _ => return Ok(false),
    };

    let expected = (w as usize) * (h as usize) * 4;
    if rgba.len() != expected {
        return Err(format!(
            "{} bytes of pixels, expected {expected}",
            rgba.len()
        ));
    }

    let file = std::fs::File::create(dir.join(format!("{stem}.png"))).map_err(|e| e.to_string())?;
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), w, h);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .and_then(|mut w| w.write_image_data(&rgba))
        .map_err(|e| e.to_string())?;
    Ok(true)
}
