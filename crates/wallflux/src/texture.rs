// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

//! Getting a wallpaper's textures onto the GPU.
//!
//! A `.tex` holds one of three things, and each takes a different road:
//!
//! - Block-compressed data, which is uploaded **as it is**. That is the point
//!   of the format: a quarter of the bytes of the uncompressed image, and the
//!   hardware samples it directly, so decoding it on the way would throw away
//!   the saving.
//! - Raw pixels, uploaded with the matching channel count.
//! - A whole JPEG or PNG, which has to be decoded first.
//!
//! Mipmaps come from the file rather than being generated: a wallpaper ships
//! them, and the ones it ships are the ones its author looked at.

use glow::HasContext;
use wpe_format::tex::{FreeImage, PixelFormat};

/// Uncompressed internal formats, as `(internal, format, type)`.
const RGBA8: (u32, u32, u32) = (glow::RGBA8, glow::RGBA, glow::UNSIGNED_BYTE);
const RG8: (u32, u32, u32) = (glow::RG8, glow::RG, glow::UNSIGNED_BYTE);
const R8: (u32, u32, u32) = (glow::R8, glow::RED, glow::UNSIGNED_BYTE);
const RGB8: (u32, u32, u32) = (glow::RGB8, glow::RGB, glow::UNSIGNED_BYTE);

/// From `GL_EXT_texture_compression_s3tc`.
const COMPRESSED_RGB_S3TC_DXT1: u32 = 0x83F0;
const COMPRESSED_RGBA_S3TC_DXT3: u32 = 0x83F2;
const COMPRESSED_RGBA_S3TC_DXT5: u32 = 0x83F3;

pub struct Texture {
    id: glow::Texture,
    pub width: u32,
    pub height: u32,
    /// Size of the image inside the texture, which is rounded up to a power of
    /// two. Anything sampling this has to scale by the ratio or it reads the
    /// empty margin.
    pub image_size: (u32, u32),
    /// The format as it ended up on the GPU, for the shader to be told.
    ///
    /// Not always the format the file names: a mipmap holding an encoded JPEG
    /// or PNG is decoded to RGBA here, whatever the header said.
    pub format: i32,
}

impl Texture {
    /// The GL name, for giving back a texture this program made itself.
    pub fn raw(&self) -> glow::Texture {
        self.id
    }

    /// Uploads one of a texture's images.
    ///
    /// A sprite sheet too big for one image is stored as several, and a frame
    /// says which of them it sits on, so they cannot all be the same texture.
    pub fn upload_page(
        gl: &glow::Context,
        source: &wpe_format::Texture,
        page: usize,
        repeat: bool,
    ) -> Result<Self, String> {
        if source.is_video() {
            return Err("this texture holds a video, not an image".into());
        }
        let mips = source.images.get(page).ok_or("texture has no such image")?;
        if mips.is_empty() {
            return Err("texture has no mipmaps".into());
        }

        let id = unsafe { gl.create_texture() }.map_err(|e| format!("create texture: {e}"))?;
        unsafe {
            gl.bind_texture(glow::TEXTURE_2D, Some(id));
            // Levels are uploaded exactly as the file has them, so the driver is
            // told how many to expect rather than being asked to invent any.
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_BASE_LEVEL, 0);
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAX_LEVEL,
                mips.len() as i32 - 1,
            );
        }

        for (level, mip) in mips.iter().enumerate() {
            let data = mip
                .decompress()
                .map_err(|e| format!("level {level}: {e}"))?;
            let result = match (source.free_image, source.format) {
                (FreeImage::None, format) if format.is_block_compressed() => unsafe {
                    upload_blocks(gl, level, format, mip.width, mip.height, &data)
                },
                (FreeImage::None, format) => unsafe {
                    upload_pixels(gl, level, format, mip.width, mip.height, &data)
                },
                (_, _) => unsafe { upload_encoded(gl, level, &data) },
            };
            if let Err(e) = result {
                unsafe { gl.delete_texture(id) };
                return Err(format!("level {level}: {e}"));
            }
        }

        unsafe {
            let min = if mips.len() > 1 {
                glow::LINEAR_MIPMAP_LINEAR
            } else {
                glow::LINEAR
            };
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, min as i32);
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                glow::LINEAR as i32,
            );
            // Said by the `.tex-json` beside a texture, where there is one.
            // Clamping is the safe default for a backdrop; a texture meant to
            // tile and clamped instead smears its edge across whatever samples
            // past it.
            let wrap = if repeat {
                glow::REPEAT
            } else {
                glow::CLAMP_TO_EDGE
            };
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, wrap as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, wrap as i32);
            gl.bind_texture(glow::TEXTURE_2D, None);
        }

        Ok(Self {
            id,
            width: mips[0].width,
            height: mips[0].height,
            // For a sheet the header's size is one frame rather than the whole
            // image, so the page's own size is the one to report.
            image_size: match source.sprite {
                Some(_) => (mips[0].width, mips[0].height),
                None => source.image_size,
            },
            format: match source.free_image {
                FreeImage::None => source.format.id(),
                _ => PixelFormat::Argb8888.id(),
            },
        })
    }

    /// Wraps a texture someone else made, so it can be bound like any other.
    ///
    /// Nothing here owns a texture -- none are ever deleted -- so this hands
    /// out a handle rather than taking charge of one.
    pub fn borrowed(id: glow::Texture, width: u32, height: u32) -> Self {
        Self {
            id,
            width,
            height,
            image_size: (width, height),
            format: PixelFormat::Argb8888.id(),
        }
    }

    /// A single opaque white pixel, for a slot a pass leaves empty.
    ///
    /// Sampling a unit nothing was bound to does not read white, it reads
    /// whatever texture was last left there -- which is how one object ends up
    /// drawing another's. The shaders name `util/white` as the default for an
    /// unset texture, so white is also what the format expects.
    pub fn white(gl: &glow::Context) -> Result<Self, String> {
        let id = unsafe { gl.create_texture() }.map_err(|e| format!("create texture: {e}"))?;
        unsafe {
            gl.bind_texture(glow::TEXTURE_2D, Some(id));
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_BASE_LEVEL, 0);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAX_LEVEL, 0);
            gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                RGBA8.0 as i32,
                1,
                1,
                0,
                RGBA8.1,
                RGBA8.2,
                glow::PixelUnpackData::Slice(Some(&[255, 255, 255, 255])),
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MIN_FILTER,
                glow::NEAREST as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                glow::NEAREST as i32,
            );
            gl.bind_texture(glow::TEXTURE_2D, None);
        }
        Ok(Self {
            id,
            width: 1,
            height: 1,
            image_size: (1, 1),
            format: PixelFormat::Argb8888.id(),
        })
    }

    pub fn bind(&self, gl: &glow::Context, unit: u32) {
        unsafe {
            gl.active_texture(glow::TEXTURE0 + unit);
            gl.bind_texture(glow::TEXTURE_2D, Some(self.id));
        }
    }
}

unsafe fn upload_blocks(
    gl: &glow::Context,
    level: usize,
    format: PixelFormat,
    width: u32,
    height: u32,
    data: &[u8],
) -> Result<(), String> {
    let internal = match format {
        PixelFormat::Dxt1 => COMPRESSED_RGB_S3TC_DXT1,
        PixelFormat::Dxt3 => COMPRESSED_RGBA_S3TC_DXT3,
        PixelFormat::Dxt5 => COMPRESSED_RGBA_S3TC_DXT5,
        other => return Err(format!("{other:?} is not block compressed")),
    };
    let expected = format
        .surface_size(width, height)
        .ok_or_else(|| format!("no size known for {format:?}"))?;
    if data.len() != expected {
        return Err(format!(
            "{} bytes of blocks, expected {expected}",
            data.len()
        ));
    }
    unsafe {
        gl.compressed_tex_image_2d(
            glow::TEXTURE_2D,
            level as i32,
            internal as i32,
            width as i32,
            height as i32,
            0,
            data.len() as i32,
            data,
        );
    }
    Ok(())
}

unsafe fn upload_pixels(
    gl: &glow::Context,
    level: usize,
    format: PixelFormat,
    width: u32,
    height: u32,
    data: &[u8],
) -> Result<(), String> {
    let (internal, layout, kind) = match format {
        // Despite the name, these bytes are in RGBA order.
        PixelFormat::Argb8888 => RGBA8,
        PixelFormat::Rgb888 => RGB8,
        PixelFormat::Rg88 => RG8,
        PixelFormat::R8 => R8,
        other => return Err(format!("no uploader for {other:?}")),
    };
    let expected = format
        .surface_size(width, height)
        .ok_or_else(|| format!("no size known for {format:?}"))?;
    if data.len() != expected {
        return Err(format!(
            "{} bytes of pixels, expected {expected}",
            data.len()
        ));
    }
    unsafe {
        // Rows of one- and two-channel data are not a multiple of four bytes
        // wide, which the default unpack alignment assumes.
        gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
        gl.tex_image_2d(
            glow::TEXTURE_2D,
            level as i32,
            internal as i32,
            width as i32,
            height as i32,
            0,
            layout,
            kind,
            glow::PixelUnpackData::Slice(Some(data)),
        );
    }
    Ok(())
}

unsafe fn upload_encoded(gl: &glow::Context, level: usize, data: &[u8]) -> Result<(), String> {
    let decoded = image::load_from_memory(data)
        .map_err(|e| e.to_string())?
        .to_rgba8();
    let (width, height) = decoded.dimensions();
    unsafe {
        gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
        gl.tex_image_2d(
            glow::TEXTURE_2D,
            level as i32,
            RGBA8.0 as i32,
            width as i32,
            height as i32,
            0,
            RGBA8.1,
            RGBA8.2,
            glow::PixelUnpackData::Slice(Some(decoded.as_raw())),
        );
    }
    Ok(())
}
