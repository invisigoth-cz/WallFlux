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
    pub levels: usize,
}

impl Texture {
    pub fn upload(gl: &glow::Context, source: &wpe_format::Texture) -> Result<Self, String> {
        if source.is_video() {
            return Err("this texture holds a video, not an image".into());
        }
        let mips = source.images.first().ok_or("texture has no images")?;
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
            // ponytail: wrapping is per-material in the format and not plumbed
            // through yet; clamping is the safe default for a backdrop, where
            // repeating would mirror an edge across the screen.
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_S,
                glow::CLAMP_TO_EDGE as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_T,
                glow::CLAMP_TO_EDGE as i32,
            );
            gl.bind_texture(glow::TEXTURE_2D, None);
        }

        Ok(Self {
            id,
            width: mips[0].width,
            height: mips[0].height,
            image_size: source.image_size,
            levels: mips.len(),
        })
    }

    pub fn bind(&self, gl: &glow::Context, unit: u32) {
        unsafe {
            gl.active_texture(glow::TEXTURE0 + unit);
            gl.bind_texture(glow::TEXTURE_2D, Some(self.id));
        }
    }

    /// How much of the texture the image occupies, for scaling texture
    /// coordinates past the power-of-two margin.
    pub fn image_scale(&self) -> (f32, f32) {
        (
            self.image_size.0 as f32 / self.width.max(1) as f32,
            self.image_size.1 as f32 / self.height.max(1) as f32,
        )
    }

    /// Texture coordinates that show this image on an output of a different
    /// shape, as `(scale, offset)` to apply to a 0..1 quad.
    pub fn cover(&self, output: (u32, u32)) -> ((f32, f32), (f32, f32)) {
        let ((sx, sy), (ox, oy)) = cover(self.image_size, output);
        let (mx, my) = self.image_scale();
        ((sx * mx, sy * my), (ox * mx, oy * my))
    }
}

/// Fits `image` onto `output` without distorting it, by cropping the overflow
/// evenly from both sides.
///
/// Wallpapers are not authored at the size they are shown. This library alone
/// holds 16:9, 16:10 and two ultrawide shapes, and no monitor matches all of
/// them -- not even one that matches none, as 2560x1413 does. Stretching to fit
/// is the one option that is always wrong, so the picture keeps its proportions
/// and the parts that cannot be shown are cut from the edges rather than
/// squeezed into view.
///
/// Returns `(scale, offset)` in texture coordinates. Nothing is resampled here:
/// the full-resolution texture stays on the GPU and the hardware samples it,
/// so a wallpaper larger than the screen loses only what the screen cannot
/// show anyway.
pub fn cover(image: (u32, u32), output: (u32, u32)) -> ((f32, f32), (f32, f32)) {
    let image_aspect = image.0 as f32 / image.1.max(1) as f32;
    let output_aspect = output.0 as f32 / output.1.max(1) as f32;

    if image_aspect > output_aspect {
        // Wider than the screen: show a centred vertical slice of it.
        let visible = output_aspect / image_aspect;
        ((visible, 1.0), ((1.0 - visible) / 2.0, 0.0))
    } else {
        let visible = image_aspect / output_aspect;
        ((1.0, visible), (0.0, (1.0 - visible) / 2.0))
    }
}

#[cfg(test)]
mod tests {
    use super::cover;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-5
    }

    #[test]
    fn an_exact_match_is_left_alone() {
        let ((sx, sy), (ox, oy)) = cover((1920, 1080), (1920, 1080));
        assert!(close(sx, 1.0) && close(sy, 1.0) && close(ox, 0.0) && close(oy, 0.0));
    }

    /// The same shape at a different size needs no cropping either, which is
    /// what keeps a 4K wallpaper whole on a 1080p screen.
    #[test]
    fn the_same_shape_at_another_size_is_left_alone() {
        let ((sx, sy), (ox, oy)) = cover((3840, 2160), (1280, 720));
        assert!(close(sx, 1.0) && close(sy, 1.0) && close(ox, 0.0) && close(oy, 0.0));
    }

    #[test]
    fn an_ultrawide_wallpaper_is_cropped_sideways_not_squashed() {
        let ((sx, sy), (ox, oy)) = cover((3440, 1440), (2560, 1440));
        assert!(sx < 1.0, "a wider picture shows less of its width");
        assert!(close(sy, 1.0), "its height is untouched");
        assert!(close(ox, (1.0 - sx) / 2.0), "and the crop is centred");
        assert!(close(oy, 0.0));
    }

    #[test]
    fn a_taller_wallpaper_is_cropped_vertically() {
        let ((sx, sy), (ox, oy)) = cover((1920, 1200), (2560, 1413));
        assert!(close(sx, 1.0));
        assert!(sy < 1.0);
        assert!(close(oy, (1.0 - sy) / 2.0));
        assert!(close(ox, 0.0));
    }

    /// Whatever the shapes, the visible area is never stretched: the ratio of
    /// the two scales always undoes the difference between the two aspects.
    #[test]
    fn proportions_survive_every_combination() {
        for image in [
            (1920, 1080),
            (3440, 1440),
            (1920, 1200),
            (5120, 2880),
            (1024, 1024),
        ] {
            for output in [(2560, 1413), (1920, 1080), (3440, 1440), (800, 600)] {
                let ((sx, sy), _) = cover(image, output);
                let shown = (image.0 as f32 * sx) / (image.1 as f32 * sy);
                let screen = output.0 as f32 / output.1 as f32;
                assert!(
                    (shown - screen).abs() < 1e-3,
                    "{image:?} on {output:?} came out at {shown} not {screen}"
                );
            }
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
