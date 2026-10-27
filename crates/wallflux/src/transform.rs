// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

//! Placing an object in the scene.
//!
//! A scene is flat and measured in pixels: an object's origin and size are
//! given in the coordinates of the scene's own resolution, with the origin in
//! the top-left corner and y increasing downwards, which is how an image at
//! `1280 720` lands in the middle of a 2560x1440 scene.
//!
//! Matrices are plain arrays in column-major order, as GL wants them. The
//! shaders multiply the other way round -- `mul(position, matrix)`, a row
//! vector -- so they are uploaded transposed; see [`Transform::upload`].

/// A 4x4 matrix in column-major order.
pub type Matrix = [f32; 16];

/// The matrix that changes nothing.
pub const IDENTITY: Matrix = [
    1.0, 0.0, 0.0, 0.0, //
    0.0, 1.0, 0.0, 0.0, //
    0.0, 0.0, 1.0, 0.0, //
    0.0, 0.0, 0.0, 1.0,
];

/// Maps a scene onto an output of another shape without distorting it.
///
/// A scene is authored at one resolution and shown at another, and the two
/// rarely agree -- this library holds 16:9, 16:10 and two ultrawide shapes. The
/// scene keeps its proportions and whatever does not fit is cut evenly from the
/// edges, so the middle of the scene stays the middle of the screen.
///
/// Projecting the scene straight onto the output instead squeezes it, which is
/// invisible whenever the two shapes happen to match and obvious the moment
/// they do not.
pub fn orthographic_cover(scene: (f32, f32), output: (f32, f32)) -> Matrix {
    let (sw, sh) = (scene.0.max(1.0), scene.1.max(1.0));
    let scene_aspect = sw / sh;
    let output_aspect = output.0.max(1.0) / output.1.max(1.0);

    // The part of the scene the output can show, centred on the scene's middle.
    let (visible_w, visible_h) = if scene_aspect > output_aspect {
        (sh * output_aspect, sh)
    } else {
        (sw, sw / output_aspect)
    };
    let (left, top) = ((sw - visible_w) / 2.0, (sh - visible_h) / 2.0);

    // Clip space only runs from -1 to 1, so depth has to be scaled into it or
    // anything that leaves that span is thrown away. A scene is flat but the
    // things in it are not: particles drift hundreds of units in z, and at a
    // scale of one they vanish the moment they do. Nothing is depth tested, so
    // the span only has to be wide enough to hold a scene.
    let depth = sw.max(sh);

    [
        2.0 / visible_w,
        0.0,
        0.0,
        0.0,
        0.0,
        -2.0 / visible_h,
        0.0,
        0.0,
        0.0,
        0.0,
        -1.0 / depth,
        0.0,
        -1.0 - 2.0 * left / visible_w,
        1.0 + 2.0 * top / visible_h,
        0.0,
        1.0,
    ]
}

/// Places a quad of `size` at `origin`, scaled and spun about its own centre.
///
/// `angles` are degrees about x, y and z. Only z turns a flat image, but all
/// three are carried because scenes set them.
pub fn model(origin: [f32; 3], size: [f32; 2], scale: [f32; 3], angles: [f32; 3]) -> Matrix {
    let (sx, sy) = (size[0] * scale[0], size[1] * scale[1]);
    let z = angles[2].to_radians();
    let (sin, cos) = (z.sin(), z.cos());

    // Column-major: the quad is built around its own centre, turned, then moved.
    [
        cos * sx,
        sin * sx,
        0.0,
        0.0,
        -sin * sy,
        cos * sy,
        0.0,
        0.0,
        0.0,
        0.0,
        scale[2],
        0.0,
        origin[0],
        origin[1],
        origin[2],
        1.0,
    ]
}

/// `a` then `b`, as `b * a`.
pub fn multiply(b: &Matrix, a: &Matrix) -> Matrix {
    let mut out = [0.0; 16];
    for column in 0..4 {
        for row in 0..4 {
            out[column * 4 + row] = (0..4).map(|k| b[k * 4 + row] * a[column * 4 + k]).sum();
        }
    }
    out
}

/// The point a transform sends to `to`, in the plane it draws in -- which is
/// all a mouse on a flat screen can say. None where the transform flattens
/// that plane to a line.
pub fn unapply_flat(m: &Matrix, to: [f32; 2]) -> Option<[f32; 2]> {
    let det = m[0] * m[5] - m[4] * m[1];
    if det.abs() < f32::EPSILON {
        return None;
    }
    let (x, y) = (to[0] - m[12], to[1] - m[13]);
    Some([(x * m[5] - y * m[4]) / det, (y * m[0] - x * m[1]) / det])
}

/// Where a point lands after a transform: a child particle system's place in
/// the scene, and a check of one without a GPU.
pub fn apply(m: &Matrix, p: [f32; 3]) -> [f32; 3] {
    let mut out = [0.0; 3];
    for (row, slot) in out.iter_mut().enumerate() {
        *slot = m[row] * p[0] + m[4 + row] * p[1] + m[8 + row] * p[2] + m[12 + row];
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_point_comes_back_from_where_a_transform_sent_it() {
        let m = multiply(
            &orthographic_cover((1920.0, 1080.0), (640.0, 480.0)),
            &model([300.0, 200.0, 0.0], [10.0, 10.0], [2.0, 3.0, 1.0], [0.0, 0.0, 30.0]),
        );
        let out = apply(&m, [0.25, -0.4, 0.0]);
        let back = unapply_flat(&m, [out[0], out[1]]).unwrap();
        assert!(close(back[0], 0.25) && close(back[1], -0.4), "{back:?}");
    }

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    fn at(m: &Matrix, p: [f32; 3]) -> [f32; 3] {
        apply(m, p)
    }

    /// y grows downwards in a scene and upwards in clip space. Getting this
    /// backwards gives a wallpaper that is right in every way but upside down.
    #[test]
    fn y_runs_the_other_way_in_clip_space() {
        let p = orthographic_cover((100.0, 100.0), (100.0, 100.0));
        let higher_on_screen = at(&p, [50.0, 10.0, 0.0]);
        let lower_on_screen = at(&p, [50.0, 90.0, 0.0]);
        assert!(higher_on_screen[1] > lower_on_screen[1]);
    }

    #[test]
    fn a_full_scene_image_covers_the_whole_screen() {
        let scene = (2560.0, 1440.0);
        let mvp = multiply(
            &orthographic_cover(scene, (scene.0, scene.1)),
            &model([1280.0, 720.0, 0.0], [2560.0, 1440.0], [1.0; 3], [0.0; 3]),
        );
        // The quad is built on -0.5..0.5, so its corners are its own extremes.
        let tl = at(&mvp, [-0.5, -0.5, 0.0]);
        let br = at(&mvp, [0.5, 0.5, 0.0]);
        assert!(close(tl[0], -1.0) && close(tl[1], 1.0), "{tl:?}");
        assert!(close(br[0], 1.0) && close(br[1], -1.0), "{br:?}");
    }

    #[test]
    fn an_object_off_centre_lands_off_centre() {
        let mvp = multiply(
            &orthographic_cover((1000.0, 1000.0), (1000.0, 1000.0)),
            &model([250.0, 500.0, 0.0], [100.0, 100.0], [1.0; 3], [0.0; 3]),
        );
        let middle = at(&mvp, [0.0, 0.0, 0.0]);
        assert!(close(middle[0], -0.5), "{middle:?}");
        assert!(close(middle[1], 0.0), "{middle:?}");
    }

    #[test]
    fn scale_grows_the_quad_about_its_own_centre() {
        let big = model(
            [500.0, 500.0, 0.0],
            [100.0, 100.0],
            [2.0, 2.0, 1.0],
            [0.0; 3],
        );
        assert_eq!(at(&big, [0.0, 0.0, 0.0]), [500.0, 500.0, 0.0]);
        let corner = at(&big, [0.5, 0.5, 0.0]);
        assert!(
            close(corner[0], 600.0) && close(corner[1], 600.0),
            "{corner:?}"
        );
    }

    #[test]
    fn a_quarter_turn_swaps_the_axes() {
        let turned = model([0.0, 0.0, 0.0], [100.0, 100.0], [1.0; 3], [0.0, 0.0, 90.0]);
        let moved = at(&turned, [0.5, 0.0, 0.0]);
        assert!(close(moved[0], 0.0) && close(moved[1], 50.0), "{moved:?}");
    }

    #[test]
    fn a_matching_shape_is_projected_whole() {
        let m = orthographic_cover((2560.0, 1440.0), (1280.0, 720.0));
        let tl = at(&m, [0.0, 0.0, 0.0]);
        let br = at(&m, [2560.0, 1440.0, 0.0]);
        assert!(close(tl[0], -1.0) && close(tl[1], 1.0), "{tl:?}");
        assert!(close(br[0], 1.0) && close(br[1], -1.0), "{br:?}");
    }

    /// A 16:9 scene on a square output shows a centred square of it, not the
    /// whole scene squeezed sideways.
    #[test]
    fn a_wider_scene_is_cropped_not_squeezed() {
        let m = orthographic_cover((3840.0, 2160.0), (1024.0, 1024.0));
        // The visible width is as many pixels as the height, centred.
        let left_edge = at(&m, [(3840.0 - 2160.0) / 2.0, 0.0, 0.0]);
        let right_edge = at(&m, [(3840.0 + 2160.0) / 2.0, 0.0, 0.0]);
        assert!(close(left_edge[0], -1.0), "{left_edge:?}");
        assert!(close(right_edge[0], 1.0), "{right_edge:?}");
        // The full width runs off the screen rather than fitting into it.
        assert!(at(&m, [0.0, 0.0, 0.0])[0] < -1.0);
    }

    #[test]
    fn a_taller_scene_is_cropped_top_and_bottom() {
        let m = orthographic_cover((1920.0, 1200.0), (1920.0, 1080.0));
        assert!(!close(at(&m, [0.0, 0.0, 0.0])[1], 1.0));
        let top = at(&m, [0.0, (1200.0 - 1920.0 / (1920.0 / 1080.0)) / 2.0, 0.0]);
        assert!(close(top[1], 1.0), "{top:?}");
    }

    /// Whatever the two shapes, the scene is never stretched: a square in the
    /// scene is still a square on the screen.
    #[test]
    fn a_square_in_the_scene_stays_square_on_screen() {
        for scene in [(3840.0, 2160.0), (3440.0, 1440.0), (1920.0, 1200.0)] {
            for output in [(2560.0, 1440.0), (1024.0, 1024.0), (800.0, 600.0)] {
                let m = orthographic_cover(scene, output);
                let a = at(&m, [100.0, 100.0, 0.0]);
                let b = at(&m, [200.0, 200.0, 0.0]);
                // Equal distances in the scene become equal distances on the
                // screen once the output's own shape is divided back out.
                let dx = (b[0] - a[0]) * output.0;
                let dy = (b[1] - a[1]).abs() * output.1;
                assert!(
                    (dx - dy).abs() / dx.abs().max(1.0) < 1e-3,
                    "{scene:?} on {output:?}: {dx} by {dy}"
                );
            }
        }
    }

    #[test]
    fn multiplying_by_the_identity_changes_nothing() {
        let m = model([3.0, 4.0, 5.0], [6.0, 7.0], [1.0; 3], [0.0; 3]);
        assert_eq!(multiply(&IDENTITY, &m), m);
        assert_eq!(multiply(&m, &IDENTITY), m);
    }
}
