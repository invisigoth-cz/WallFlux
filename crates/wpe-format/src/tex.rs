// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

//! The `.tex` texture container.
//!
//! Three containers nest inside one file:
//!
//! - `TEXV0005` / `TEXI0001` carry the pixel format and both sizes. The texture
//!   size is rounded up to a power of two; the image sits in its top-left
//!   corner at the image size.
//! - `TEXB000{2,3,4}` carries the images and their mipmaps. **`TEXB0002` has no
//!   `free_image` field and `TEXB0004` carries one extra word after it** —
//!   reading the wrong number of words here desynchronises the whole file.
//! - `TEXS000{2,3}` is optional and carries sprite sheet animation.
//!   **`TEXS0002` has no frame size**; `TEXS0003` added it. Frames are 32 bytes
//!   in both.
//!
//! Pixels are RGBA, not BGRA. `free_image` is FreeImage's `FREE_IMAGE_FORMAT`
//! enum, so a mipmap may hold an encoded JPEG or PNG instead of raw pixels.

use crate::{Error, Reader, Result};

const MAX_IMAGES: i32 = 64;
const MAX_MIPS: i32 = 32;
const MAX_FRAMES: i32 = 262_144;
const MAX_DIM: i32 = 32_768;
/// No wallpaper texture decompresses to more than this; a larger claim is a
/// malformed or hostile file rather than something to allocate for.
const MAX_DECOMPRESSED: usize = 512 << 20;

/// Set in [`Texture::flags`] when the payload is an encoded video.
const FLAG_VIDEO: i32 = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    Argb8888,
    Rgb888,
    Rgb565,
    Dxt5,
    Dxt3,
    Dxt1,
    Rg88,
    R8,
    Unknown(i32),
}

impl PixelFormat {
    fn from_id(id: i32) -> Self {
        match id {
            0 => Self::Argb8888,
            1 => Self::Rgb888,
            2 => Self::Rgb565,
            4 => Self::Dxt5,
            6 => Self::Dxt3,
            7 => Self::Dxt1,
            8 => Self::Rg88,
            9 => Self::R8,
            other => Self::Unknown(other),
        }
    }

    /// The number the format is written as, which is also the number the
    /// shaders compare against: `common_fragment.h` declares the same list as
    /// `FORMAT_RGBA8888`, `FORMAT_RG88` and so on. A shader not told which of
    /// them it is given reads a two-channel texture as red and green instead
    /// of as grey with an alpha.
    pub fn id(self) -> i32 {
        match self {
            Self::Argb8888 => 0,
            Self::Rgb888 => 1,
            Self::Rgb565 => 2,
            Self::Dxt5 => 4,
            Self::Dxt3 => 6,
            Self::Dxt1 => 7,
            Self::Rg88 => 8,
            Self::R8 => 9,
            Self::Unknown(other) => other,
        }
    }

    /// Whether the format stores 4x4 blocks rather than individual pixels.
    pub fn is_block_compressed(self) -> bool {
        matches!(self, Self::Dxt1 | Self::Dxt3 | Self::Dxt5)
    }

    /// Bytes one pixel occupies, as eighths so the half-byte block formats stay
    /// exact. `None` where the size is not fixed.
    fn eighths_per_pixel(self) -> Option<usize> {
        Some(match self {
            Self::Argb8888 => 32,
            Self::Rgb888 => 24,
            Self::Rgb565 | Self::Rg88 => 16,
            Self::Dxt3 | Self::Dxt5 | Self::R8 => 8,
            Self::Dxt1 => 4,
            Self::Unknown(_) => return None,
        })
    }

    /// Bytes a full `width` x `height` surface occupies, with block formats
    /// rounded up to whole 4x4 blocks.
    pub fn surface_size(self, width: u32, height: u32) -> Option<usize> {
        let (w, h) = if self.is_block_compressed() {
            (width.div_ceil(4) * 4, height.div_ceil(4) * 4)
        } else {
            (width, height)
        };
        let bits = (w as usize)
            .checked_mul(h as usize)?
            .checked_mul(self.eighths_per_pixel()?)?;
        Some(bits / 8)
    }
}

/// FreeImage's `FREE_IMAGE_FORMAT`, as stored. `None` means the mipmaps hold
/// raw pixels in the texture's [`PixelFormat`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreeImage {
    None,
    Bmp,
    Jpeg,
    Png,
    Other(i32),
}

impl FreeImage {
    fn from_id(id: i32) -> Self {
        match id {
            -1 => Self::None,
            0 => Self::Bmp,
            2 => Self::Jpeg,
            13 => Self::Png,
            other => Self::Other(other),
        }
    }
}

pub struct Mipmap {
    pub width: u32,
    pub height: u32,
    compressed: bool,
    decompressed_size: usize,
    data: Vec<u8>,
}

impl Mipmap {
    /// The bytes as stored, still LZ4-compressed if the file says so.
    pub fn raw(&self) -> &[u8] {
        &self.data
    }

    pub fn is_compressed(&self) -> bool {
        self.compressed
    }

    /// The bytes with any LZ4 wrapper removed.
    ///
    /// The result is either raw pixels or an encoded image, depending on the
    /// texture's [`Texture::free_image`].
    pub fn decompress(&self) -> Result<std::borrow::Cow<'_, [u8]>> {
        if !self.compressed {
            return Ok(std::borrow::Cow::Borrowed(&self.data));
        }
        let out = lz4_flex::decompress(&self.data, self.decompressed_size)
            .map_err(|e| Error::Decompress(e.to_string()))?;
        if out.len() != self.decompressed_size {
            return Err(Error::SizeMismatch {
                got: out.len(),
                want: self.decompressed_size,
            });
        }
        Ok(std::borrow::Cow::Owned(out))
    }
}

/// One frame of a sprite sheet animation.
pub struct Frame {
    /// Which of the texture's images this frame sits on. A sheet too large for
    /// one image is spread over several.
    pub image: i32,
    /// Seconds this frame is held.
    ///
    /// The **first** frame of a `TEXS0002` sheet states something here that is
    /// plainly not a duration -- 26 where every other frame of the same sheet
    /// says a 25th of a second -- so a player should take the interval the
    /// frames agree on rather than trusting any single one.
    pub duration: f32,
    /// Where the frame sits on its image, in pixels.
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    /// Two words between the width and the height, zero in every frame of
    /// every texture in the library. Carried rather than dropped, since a
    /// reader that assumed them away would misread any file that used them.
    pub unknown: [f32; 2],
}

pub struct Sprite {
    pub version: String,
    /// One frame's size in pixels, present only from `TEXS0003` onwards.
    ///
    /// Not a count of rows and columns, which is the obvious reading and the
    /// wrong one: a sheet 38 by 7600 with 200 frames states `38, 38`, and
    /// 200 frames of 38 pixels is exactly its height. Read as a grid it would
    /// claim 1444 frames. The same holds for the 53 and 100 pixel sheets in
    /// the library.
    pub frame_size: Option<(u32, u32)>,
    pub frames: Vec<Frame>,
}

/// What the `.tex-json` beside a texture says about it.
///
/// A texture that ships with Wallpaper Engine states its sprite sheet layout
/// here. It matters for the ones whose own `TEXS` block is the older version
/// that gives no frame size: without the sidecar there is nothing to lay the
/// sheet out from, and the whole sheet gets drawn at once on every particle.
///
/// Wallpapers' own textures are the other way round: all 384 in the library
/// carry a `TEXS` block and not one ships a sidecar.
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(default)]
pub struct Sidecar {
    #[serde(rename = "spritesheetsequences")]
    pub sequences: Vec<Sequence>,
    /// Whether the texture stops at its edges or starts over. Of the textures
    /// that ship with Wallpaper Engine, 121 clamp and 61 repeat, and a cloud
    /// layer that clamps when it meant to repeat smears its edge across the
    /// sky in stripes.
    #[serde(rename = "clampuvs")]
    pub clamp: Option<bool>,
}

/// One sprite sheet animation described by a sidecar.
#[derive(Debug, Clone, Copy, Default, serde::Deserialize)]
#[serde(default)]
pub struct Sequence {
    pub frames: u32,
    /// One frame's width in pixels.
    pub width: u32,
    /// One frame's height in pixels.
    pub height: u32,
    /// Seconds the whole sequence runs for.
    pub duration: f32,
}

impl Sidecar {
    pub fn parse(bytes: &[u8]) -> serde_json::Result<Self> {
        serde_json::from_slice(bytes)
    }

    /// The sheet this texture animates through, if it says it has one.
    pub fn sheet(&self) -> Option<Sequence> {
        self.sequences
            .iter()
            .copied()
            .find(|s| s.frames > 0 && s.width > 0 && s.height > 0)
    }
}

pub struct Texture {
    pub container: String,
    pub image_container: String,
    pub block_container: String,
    pub format: PixelFormat,
    pub flags: i32,
    /// Power-of-two size of the allocated texture.
    pub texture_size: (u32, u32),
    /// Size of the actual image inside it.
    pub image_size: (u32, u32),
    /// A word whose meaning has not been established.
    ///
    /// It was first read as an average colour, which decoding disproved: across
    /// 138 textures the pixel average drifted from it by 64/255 at the median,
    /// including for formats whose decoding is independently confirmed. In some
    /// masks it reads as `(0, 127, 254)`, closer to a parameter triple than a
    /// colour. Carried through unaltered until something identifies it.
    pub unknown_word: u32,
    pub free_image: FreeImage,
    pub images: Vec<Vec<Mipmap>>,
    pub sprite: Option<Sprite>,
}

impl Texture {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let mut r = Reader::new(bytes);

        let container = r.tag()?.to_owned();
        if !container.starts_with("TEXV") {
            return Err(Error::UnknownTag {
                tag: container,
                at: 0,
            });
        }
        let image_container = r.tag()?.to_owned();

        let format = PixelFormat::from_id(r.i32()?);
        let flags = r.i32()?;
        let texture_size = (dim(&mut r)?, dim(&mut r)?);
        let image_size = (dim(&mut r)?, dim(&mut r)?);
        let unknown_word = r.u32()?;

        let at = r.pos();
        let block_container = r.tag()?.to_owned();
        // The field count differs per version. Getting this wrong does not fail
        // here; it silently shifts every later read, so each version is explicit.
        let (has_free_image, extra_words) = match block_container.as_str() {
            // The oldest: no image format, and mipmaps that are never packed.
            "TEXB0001" | "TEXB0002" => (false, 0),
            "TEXB0003" => (true, 0),
            "TEXB0004" => (true, 1),
            _ => {
                return Err(Error::UnknownTag {
                    tag: block_container,
                    at,
                });
            }
        };

        let image_count = r.count("image count", MAX_IMAGES)?;
        let free_image = if has_free_image {
            FreeImage::from_id(r.i32()?)
        } else {
            FreeImage::None
        };
        for _ in 0..extra_words {
            r.i32()?;
        }

        let mut images = Vec::with_capacity(image_count.min(8));
        for _ in 0..image_count {
            let mip_count = r.count("mipmap count", MAX_MIPS)?;
            let mut mips = Vec::with_capacity(mip_count.min(16));
            for _ in 0..mip_count {
                mips.push(read_mipmap(&mut r, format, free_image, block_container != "TEXB0001")?);
            }
            images.push(mips);
        }

        let sprite = if r.is_empty() {
            None
        } else {
            Some(read_sprite(&mut r)?)
        };

        if !r.is_empty() {
            return Err(Error::Trailing {
                at: r.pos(),
                left: r.remaining(),
            });
        }

        Ok(Self {
            container,
            image_container,
            block_container,
            format,
            flags,
            texture_size,
            image_size,
            unknown_word,
            free_image,
            images,
            sprite,
        })
    }

    /// Whether this texture holds an encoded video rather than a surface.
    ///
    /// Scene wallpapers embed MP4 this way. Across the 384 textures this was
    /// derived from, flag bit 5 was set for exactly the two that begin with an
    /// ISO base media signature and for no others. That is a clean split but a
    /// small sample, so callers that must be certain should also check the
    /// bytes for `ftyp` at offset 4.
    pub fn is_video(&self) -> bool {
        self.flags & FLAG_VIDEO != 0
    }

    /// The first mipmap of the first image: the full-size surface.
    pub fn base(&self) -> Option<&Mipmap> {
        self.images.first()?.first()
    }
}

fn dim(r: &mut Reader<'_>) -> Result<u32> {
    let at = r.pos();
    let v = r.i32()?;
    if !(0..=MAX_DIM).contains(&v) {
        return Err(Error::Absurd {
            what: "dimension",
            value: v as i64,
            at,
        });
    }
    Ok(v as u32)
}

fn read_mipmap(
    r: &mut Reader<'_>,
    format: PixelFormat,
    free: FreeImage,
    packable: bool,
) -> Result<Mipmap> {
    let width = dim(r)?;
    let height = dim(r)?;
    let compressed = packable && r.i32()? != 0;

    let at = r.pos();
    let declared = if packable { r.i32()? } else { 0 };
    if declared < 0 || declared as usize > MAX_DECOMPRESSED {
        return Err(Error::Absurd {
            what: "decompressed size",
            value: declared as i64,
            at,
        });
    }
    let mut decompressed_size = declared as usize;

    let size = r.count("mipmap byte count", i32::MAX)?;
    let data = r.take(size)?.to_vec();

    if compressed {
        // Raw pixel data has a size the format and dimensions already fix, so a
        // mismatch means the header is lying about how much it will expand to.
        if free == FreeImage::None
            && let Some(expected) = format.surface_size(width, height)
            && decompressed_size != expected
        {
            return Err(Error::SizeMismatch {
                got: decompressed_size,
                want: expected,
            });
        }
    } else {
        decompressed_size = data.len();
    }

    Ok(Mipmap {
        width,
        height,
        compressed,
        decompressed_size,
        data,
    })
}

fn read_sprite(r: &mut Reader<'_>) -> Result<Sprite> {
    let at = r.pos();
    let version = r.tag()?.to_owned();
    let has_frame_size = match version.as_str() {
        "TEXS0002" => false,
        "TEXS0003" => true,
        _ => return Err(Error::UnknownTag { tag: version, at }),
    };

    let count = r.count("frame count", MAX_FRAMES)?;
    let frame_size = if has_frame_size {
        Some((dim(r)?, dim(r)?))
    } else {
        None
    };

    let mut frames = Vec::with_capacity(count.min(1024));
    for _ in 0..count {
        let image = r.i32()?;
        let mut words = [0f32; 7];
        for v in &mut words {
            *v = r.f32()?;
        }
        frames.push(Frame {
            image,
            duration: words[0],
            x: words[1],
            y: words[2],
            width: words[3],
            unknown: [words[4], words[5]],
            height: words[6],
        });
    }

    Ok(Sprite {
        version,
        frame_size,
        frames,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn put(v: &mut Vec<u8>, xs: &[i32]) {
        for x in xs {
            v.extend_from_slice(&x.to_le_bytes());
        }
    }

    /// One 4x4 ARGB8888 texture, stored uncompressed, in the given block version.
    fn texture(block: &str, pixels: &[u8]) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(b"TEXV0005\0TEXI0001\0");
        put(&mut v, &[0, 0, 4, 4, 4, 4, 0]);
        v.extend_from_slice(block.as_bytes());
        v.push(0);
        put(&mut v, &[1]); // one image
        match block {
            "TEXB0003" => put(&mut v, &[-1]),
            "TEXB0004" => put(&mut v, &[-1, 0]),
            _ => {}
        }
        put(&mut v, &[1]); // one mipmap
        put(&mut v, &[4, 4, 0, 0, pixels.len() as i32]);
        v.extend_from_slice(pixels);
        v
    }

    #[test]
    fn every_block_version_yields_the_same_pixels() {
        let pixels = [7u8; 64];
        for block in ["TEXB0002", "TEXB0003", "TEXB0004"] {
            let t = Texture::parse(&texture(block, &pixels)).expect(block);
            assert_eq!(t.format, PixelFormat::Argb8888);
            assert_eq!(t.image_size, (4, 4));
            assert_eq!(t.base().unwrap().decompress().unwrap().as_ref(), &pixels);
        }
    }

    /// Reading TEXB0002 as though it had a free_image field, or TEXB0004 as
    /// though it had none, shifts every later field. Guards the one mistake
    /// that cost the most to find.
    #[test]
    fn block_version_field_counts_are_not_interchangeable() {
        let mut wrong = texture("TEXB0003", &[0u8; 64]);
        let tag = wrong.windows(8).position(|w| w == b"TEXB0003").unwrap();
        wrong[tag..tag + 8].copy_from_slice(b"TEXB0002");
        assert!(Texture::parse(&wrong).is_err());
    }

    #[test]
    fn sprite_versions_differ_by_the_frame_size_field() {
        for (version, extra) in [("TEXS0002", &[][..]), ("TEXS0003", &[16, 16][..])] {
            let mut v = texture("TEXB0003", &[0u8; 64]);
            v.extend_from_slice(version.as_bytes());
            v.push(0);
            put(&mut v, &[1]);
            put(&mut v, extra);
            put(&mut v, &[0]);
            v.extend_from_slice(&[0u8; 28]);
            let t = Texture::parse(&v).expect(version);
            let s = t.sprite.unwrap();
            assert_eq!(s.frames.len(), 1);
            assert_eq!(s.frame_size, (version == "TEXS0003").then_some((16, 16)));
        }
    }

    #[test]
    fn rejects_declared_size_that_disagrees_with_the_format() {
        let mut v = texture("TEXB0003", &[0u8; 64]);
        let n = v.len();
        // Mark compressed and claim it expands to 999 bytes, not 4*4*4 = 64.
        v[n - 64 - 12..n - 64 - 8].copy_from_slice(&1i32.to_le_bytes());
        v[n - 64 - 8..n - 64 - 4].copy_from_slice(&999i32.to_le_bytes());
        assert!(matches!(
            Texture::parse(&v),
            Err(Error::SizeMismatch { .. })
        ));
    }

    #[test]
    fn rejects_trailing_bytes() {
        let mut v = texture("TEXB0003", &[0u8; 64]);
        v.push(0xff);
        assert!(Texture::parse(&v).is_err());
    }

    #[test]
    fn video_flag_is_read_from_the_header() {
        let mut v = texture("TEXB0003", &[0u8; 64]);
        assert!(!Texture::parse(&v).unwrap().is_video());
        // flags is the word right after the format id, at the end of the tags.
        let at = b"TEXV0005\0TEXI0001\0".len() + 4;
        v[at..at + 4].copy_from_slice(&34i32.to_le_bytes());
        assert!(Texture::parse(&v).unwrap().is_video());
    }

    #[test]
    fn surface_size_rounds_blocks_up() {
        assert_eq!(PixelFormat::Argb8888.surface_size(4, 4), Some(64));
        assert_eq!(PixelFormat::Dxt1.surface_size(4, 4), Some(8));
        assert_eq!(PixelFormat::Dxt5.surface_size(4, 4), Some(16));
        // A 5x5 DXT1 surface still occupies two full rows of blocks.
        assert_eq!(PixelFormat::Dxt1.surface_size(5, 5), Some(32));
        assert_eq!(PixelFormat::Unknown(42).surface_size(4, 4), None);
    }
}

#[cfg(test)]
mod sidecar_tests {
    use super::Sidecar;

    #[test]
    fn reads_the_sheet_a_sidecar_describes() {
        let json = br#"{"clampuvs":false,"format":"r8","nonpoweroftwo":true,
            "spritesheetsequences":[{"duration":1,"frames":64,"height":64,"width":128}]}"#;
        let sheet = Sidecar::parse(json).unwrap().sheet().expect("no sheet");
        assert_eq!((sheet.frames, sheet.width, sheet.height), (64, 128, 64));
    }

    #[test]
    fn a_sidecar_without_a_sequence_describes_no_sheet() {
        let json = br#"{"format":"dxt5","spritesheetsequences":[]}"#;
        assert!(Sidecar::parse(json).unwrap().sheet().is_none());
        assert!(Sidecar::parse(b"{}").unwrap().sheet().is_none());
    }

    #[test]
    fn finds_the_sheet_a_built_in_lightning_texture_hides_in_its_sidecar() {
        let Some(assets) = crate::Assets::locate() else {
            return;
        };
        let Some(bytes) = assets.get("materials/particle/lightning/lightning1.tex-json") else {
            return;
        };
        let sheet = Sidecar::parse(&bytes).unwrap().sheet().expect("no sheet");
        assert_eq!(sheet.frames, 64);
        // The point of the sidecar: the texture's own sprite block is the
        // older version, which states no frame size, so the sheet cannot be
        // laid out from the `.tex` alone.
        let tex = assets
            .get("materials/particle/lightning/lightning1.tex")
            .and_then(|b| super::Texture::parse(&b).ok())
            .expect("no texture");
        assert_eq!(tex.sprite.map(|s| s.frame_size), Some(None));
    }
}

#[cfg(test)]
mod frame_tests {
    use super::Texture;

    #[test]
    fn a_frame_states_where_it_sits_and_how_long_it_is_held() {
        let Some(assets) = crate::Assets::locate() else {
            return;
        };
        let Some(bytes) = assets.get("materials/particle/fog/fog1.tex") else {
            return;
        };
        let sprite = Texture::parse(&bytes).unwrap().sprite.unwrap();
        // A 1024 square holding 64 frames of 128 pixels, a second in all.
        assert_eq!(sprite.frames.len(), 64);
        assert_eq!(
            (sprite.frames[0].x, sprite.frames[0].y),
            (0.0, 0.0),
            "the first frame should start at the corner"
        );
        for frame in &sprite.frames {
            assert_eq!((frame.width, frame.height), (128.0, 128.0));
            assert_eq!(frame.unknown, [0.0, 0.0]);
            assert!((frame.duration - 1.0 / 64.0).abs() < 1e-6);
        }
        let last = sprite.frames.last().unwrap();
        assert_eq!((last.x, last.y), (896.0, 896.0), "the sheet is 8 by 8");
    }
}

#[cfg(test)]
mod wrapping_tests {
    use super::Sidecar;

    #[test]
    fn reads_whether_a_texture_repeats() {
        let repeats = br#"{"clampuvs":false,"format":"rgba8888"}"#;
        let clamps = br#"{"clampuvs":true}"#;
        assert_eq!(Sidecar::parse(repeats).unwrap().clamp, Some(false));
        assert_eq!(Sidecar::parse(clamps).unwrap().clamp, Some(true));
        // Said by neither, which is the wallpapers' own textures to a one.
        assert_eq!(Sidecar::parse(b"{}").unwrap().clamp, None);
    }

    #[test]
    fn the_cloud_layer_wallpapers_use_says_it_repeats() {
        let Some(assets) = crate::Assets::locate() else {
            return;
        };
        let Some(bytes) = assets.get("materials/util/clouds_256.tex-json") else {
            return;
        };
        assert_eq!(Sidecar::parse(&bytes).unwrap().clamp, Some(false));
    }
}
