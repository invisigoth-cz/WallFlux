// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

//! A puppet: an image cut into a mesh, with bones to bend it and animations
//! that move the bones. What the editor's puppet warp makes, as an `.mdl`.
//!
//! The layout was read off the five in the library, all `MDLV0017`, `MDLS0002`
//! and `MDLA0004`, and each of them is consumed to its last byte by this.
//! Anything else is refused, and the image is then drawn flat.

/// One corner of the mesh, in the picture's pixels from its middle, y up.
#[derive(Debug, Clone, Copy)]
pub struct Corner {
    pub position: [f32; 3],
    pub bones: [u32; 4],
    pub weights: [f32; 4],
    pub uv: [f32; 2],
}

/// A bone's place against its parent. Only a turn in the picture's plane:
/// the puppets here have no scale, and their largest other turn is 0.05.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Pose {
    pub x: f32,
    pub y: f32,
    pub angle: f32,
}

#[derive(Debug, Clone)]
pub struct Bone {
    pub parent: Option<usize>,
    pub rest: Pose,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Playback {
    Loop,
    Mirror,
    Once,
}

#[derive(Debug, Clone)]
pub struct Animation {
    pub id: i64,
    pub fps: f32,
    pub frames: f32,
    pub playback: Playback,
    /// Per bone, its poses frame by frame.
    pub tracks: Vec<Vec<Pose>>,
    /// The frame its first pass starts from. See [`first_settled_frame`].
    pub opening: f32,
}

impl Animation {
    /// The pose of `bone` at `seconds` of this animation.
    fn pose(&self, bone: usize, seconds: f32) -> Option<Pose> {
        let track = self.tracks.get(bone).filter(|t| !t.is_empty())?;
        let frame = seconds.max(0.0) * self.fps;
        let last = self.frames.max(f32::EPSILON);
        let at = match self.playback {
            Playback::Loop => frame.rem_euclid(last),
            Playback::Mirror if frame < last => frame.max(self.opening),
            Playback::Mirror => last - (frame.rem_euclid(2.0 * last) - last).abs(),
            Playback::Once => frame.min(last),
        };
        let first = (at as usize).min(track.len() - 1);
        let (a, b) = (track[first], track[(first + 1).min(track.len() - 1)]);
        let t = at - first as f32;
        Some(Pose {
            x: a.x + (b.x - a.x) * t,
            y: a.y + (b.y - a.y) * t,
            angle: a.angle + (b.angle - a.angle) * t,
        })
    }
}

/// Where a first pass begins when an animation starts away from rest: the frame
/// the bone furthest off has come furthest back. Alien Isolation's second jaw starts
/// out of the mouth, and Wallpaper Engine leaves it in the mouth until it comes
/// out at the far end of the mirror, forty seconds in. Seen from Windows, not
/// explained; three animations here start that way and the rest start at rest.
fn first_settled_frame(tracks: &[Vec<Pose>], bones: &[Bone]) -> f32 {
    let off = |i: usize| {
        let (start, rest) = (tracks[i].first().copied().unwrap_or(bones[i].rest), bones[i].rest);
        (rest.x - start.x, rest.y - start.y)
    };
    let size = |(x, y): (f32, f32)| x * x + y * y;
    let Some(furthest) = (0..tracks.len()).max_by(|a, b| size(off(*a)).total_cmp(&size(off(*b)))) else {
        return 0.0;
    };
    let back = off(furthest);
    if size(back) < 1.0 {
        return 0.0;
    }
    // How far along the way back: the first frame to get furthest, since a curve
    // may stand still at either end of the journey.
    let track = &tracks[furthest];
    let along = |p: &Pose| (p.x - track[0].x) * back.0 + (p.y - track[0].y) * back.1;
    let mut opening = 0;
    for (frame, pose) in track.iter().enumerate() {
        if along(pose) > along(&track[opening]) + 1e-3 {
            opening = frame;
        }
    }
    opening as f32
}

/// An animation as the scene plays it.
#[derive(Debug, Clone, Copy)]
pub struct Playing {
    pub animation: i64,
    pub rate: f32,
    pub blend: f32,
}

#[derive(Debug, Clone)]
pub struct Puppet {
    pub corners: Vec<Corner>,
    pub triangles: Vec<u16>,
    pub bones: Vec<Bone>,
    pub animations: Vec<Animation>,
}

/// A rotation and a shift, which is all a bone does here.
#[derive(Debug, Clone, Copy)]
struct Placed {
    cos: f32,
    sin: f32,
    x: f32,
    y: f32,
}

impl Placed {
    fn of(pose: Pose) -> Self {
        let (sin, cos) = pose.angle.sin_cos();
        Self { cos, sin, x: pose.x, y: pose.y }
    }

    fn apply(&self, p: [f32; 2]) -> [f32; 2] {
        [p[0] * self.cos - p[1] * self.sin + self.x, p[0] * self.sin + p[1] * self.cos + self.y]
    }

    /// `inner` first, then this.
    fn after(&self, inner: &Placed) -> Placed {
        let [x, y] = self.apply([inner.x, inner.y]);
        Placed {
            cos: self.cos * inner.cos - self.sin * inner.sin,
            sin: self.sin * inner.cos + self.cos * inner.sin,
            x,
            y,
        }
    }

    fn inverse(&self) -> Placed {
        Placed {
            cos: self.cos,
            sin: -self.sin,
            x: -(self.x * self.cos + self.y * self.sin),
            y: self.x * self.sin - self.y * self.cos,
        }
    }
}

impl Puppet {
    pub fn parse(bytes: &[u8]) -> Option<Self> {
        let mut r = Reader { bytes, at: 0 };
        if r.take(9)? != b"MDLV0017\0" {
            return None;
        }
        r.take(12)?;
        r.string()?; // The material, which the model names as well.
        r.take(32)?;

        let size = r.u32()? as usize;
        if size % 80 != 0 {
            return None;
        }
        let mut corners = Vec::with_capacity(size / 80);
        for _ in 0..size / 80 {
            let position = [r.f32()?, r.f32()?, r.f32()?];
            r.take(12 + 16)?; // Normal and tangent: a flat picture's, always.
            let bones = [r.u32()?, r.u32()?, r.u32()?, r.u32()?];
            let weights = [r.f32()?, r.f32()?, r.f32()?, r.f32()?];
            corners.push(Corner { position, bones, weights, uv: [r.f32()?, r.f32()?] });
        }
        let size = r.u32()? as usize;
        if size % 6 != 0 {
            return None;
        }
        let mut triangles = Vec::with_capacity(size / 2);
        for _ in 0..size / 2 {
            let corner = r.u16()?;
            if corner as usize >= corners.len() {
                return None;
            }
            triangles.push(corner);
        }

        if r.take(9)? != b"MDLS0002\0" {
            return None;
        }
        let skeleton_ends = r.u32()? as usize;
        let count = r.u16()? as usize;
        r.take(2)?;
        let mut bones: Vec<Bone> = Vec::with_capacity(count);
        for index in 0..count {
            r.string()?;
            r.take(4)?;
            let parent = r.u32()?;
            let parent = match parent {
                u32::MAX => None,
                // Parents come first, which is what lets a pose be worked out
                // in one pass, and what stops a bone being its own ancestor.
                p if (p as usize) < index => Some(p as usize),
                _ => return None,
            };
            if r.u32()? != 64 {
                return None;
            }
            let matrix: Vec<f32> = (0..16).map(|_| r.f32()).collect::<Option<_>>()?;
            r.string()?; // The simulation's settings, which nothing here runs.
            bones.push(Bone {
                parent,
                rest: Pose { x: matrix[12], y: matrix[13], angle: matrix[1].atan2(matrix[0]) },
            });
        }
        if corners.iter().any(|c| c.bones.iter().any(|b| *b as usize >= count)) {
            return None;
        }
        r.at = skeleton_ends;
        // Attachments, where it has any, which nothing here hangs anything on.
        if bytes.get(r.at..)?.starts_with(b"MDAT0001\0") {
            r.take(9)?;
            r.at = r.u32()? as usize;
        }

        if r.take(9)? != b"MDLA0004\0" {
            return None;
        }
        r.u32()?;
        let count = r.u32()?;
        let mut animations = Vec::new();
        for _ in 0..count {
            let id = r.u32()? as i64;
            r.take(4)?;
            r.string()?;
            let playback = match r.string()? {
                "mirror" => Playback::Mirror,
                "single" => Playback::Once,
                _ => Playback::Loop,
            };
            let fps = r.f32()?;
            let frames = r.u32()? as f32;
            r.take(4)?;
            let tracked = r.u32()? as usize;
            if tracked != bones.len() {
                return None;
            }
            let mut tracks = Vec::with_capacity(tracked);
            for _ in 0..tracked {
                r.take(4)?;
                let size = r.u32()? as usize;
                let mut track = Vec::with_capacity(size / 36);
                for _ in 0..size / 36 {
                    let values: Vec<f32> = (0..9).map(|_| r.f32()).collect::<Option<_>>()?;
                    track.push(Pose { x: values[0], y: values[1], angle: values[5] });
                }
                tracks.push(track);
            }
            r.take(10)?;
            let opening = first_settled_frame(&tracks, &bones);
            animations.push(Animation { id, fps, frames, playback, tracks, opening });
        }
        Some(Self { corners, triangles, bones, animations })
    }

    /// Where every bone's corners have gone from their rest: each animation
    /// adds what it moves the bone by, against the bone at rest.
    fn bent(&self, playing: &[Playing], seconds: f32) -> Vec<Placed> {
        let mut at_rest: Vec<Placed> = Vec::with_capacity(self.bones.len());
        let mut now: Vec<Placed> = Vec::with_capacity(self.bones.len());
        for (index, bone) in self.bones.iter().enumerate() {
            let mut pose = bone.rest;
            for layer in playing {
                let Some(animation) = self.animations.iter().find(|a| a.id == layer.animation) else {
                    continue;
                };
                let Some(moved) = animation.pose(index, seconds * layer.rate) else {
                    continue;
                };
                pose.x += (moved.x - bone.rest.x) * layer.blend;
                pose.y += (moved.y - bone.rest.y) * layer.blend;
                pose.angle += (moved.angle - bone.rest.angle) * layer.blend;
            }
            let (rest, moved) = (Placed::of(bone.rest), Placed::of(pose));
            at_rest.push(match bone.parent {
                Some(parent) => at_rest[parent].after(&rest),
                None => rest,
            });
            now.push(match bone.parent {
                Some(parent) => now[parent].after(&moved),
                None => moved,
            });
        }
        now.iter().zip(&at_rest).map(|(n, r)| n.after(&r.inverse())).collect()
    }

    /// Every corner at `seconds`, in the picture's pixels: x, y, z, u, v.
    pub fn bend_into(&self, playing: &[Playing], seconds: f32, out: &mut Vec<f32>) {
        let bent = self.bent(playing, seconds);
        out.clear();
        for corner in &self.corners {
            let p = [corner.position[0], corner.position[1]];
            let (mut x, mut y) = (0.0, 0.0);
            for (bone, weight) in corner.bones.iter().zip(corner.weights) {
                if weight != 0.0 {
                    let moved = bent[*bone as usize].apply(p);
                    x += moved[0] * weight;
                    y += moved[1] * weight;
                }
            }
            out.extend_from_slice(&[x, y, corner.position[2], corner.uv[0], corner.uv[1]]);
        }
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        let piece = self.bytes.get(self.at..self.at.checked_add(n)?)?;
        self.at += n;
        Some(piece)
    }
    fn u16(&mut self) -> Option<u16> {
        Some(u16::from_le_bytes(self.take(2)?.try_into().ok()?))
    }
    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }
    fn f32(&mut self) -> Option<f32> {
        Some(f32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }
    fn string(&mut self) -> Option<&'a str> {
        let rest = self.bytes.get(self.at..)?;
        let end = rest.iter().position(|b| *b == 0)?;
        self.at += end + 1;
        std::str::from_utf8(&rest[..end]).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn two_bones(track: Vec<Pose>, playback: Playback) -> Puppet {
        let bones = vec![
            Bone { parent: None, rest: Pose::default() },
            Bone { parent: Some(0), rest: Pose { x: 10.0, y: 0.0, angle: 0.0 } },
        ];
        Puppet {
            corners: vec![Corner {
                position: [20.0, 0.0, 0.0],
                bones: [1, 0, 0, 0],
                weights: [1.0, 0.0, 0.0, 0.0],
                uv: [0.25, 0.75],
            }],
            triangles: vec![],
            animations: vec![Animation {
                id: 7,
                fps: 1.0,
                frames: 2.0,
                playback,
                tracks: vec![
                    vec![Pose::default(); 3],
                    track,
                ],
                opening: 0.0,
            }],
            bones,
        }
    }

    const PLAY: Playing = Playing { animation: 7, rate: 1.0, blend: 1.0 };

    fn bent_x_y(puppet: &Puppet, playing: &[Playing], seconds: f32) -> (f32, f32) {
        let mut out = Vec::new();
        puppet.bend_into(playing, seconds, &mut out);
        (out[0], out[1])
    }

    #[test]
    fn a_first_pass_starts_where_a_bone_off_rest_settles() {
        let bones = vec![Bone { parent: None, rest: Pose::default() }];
        let at = |x: f32| Pose { x, y: 0.0, angle: 0.0 };
        let track = vec![at(9.0), at(9.0), at(-1.0), at(2.0), at(0.0)];
        assert_eq!(first_settled_frame(std::slice::from_ref(&track), &bones), 2.0);
        let animation = Animation {
            id: 1,
            fps: 1.0,
            frames: 4.0,
            playback: Playback::Mirror,
            tracks: vec![track.clone()],
            opening: 2.0,
        };
        assert_eq!(animation.pose(0, 0.0).unwrap().x, -1.0, "held at the start");
        assert_eq!(animation.pose(0, 3.0).unwrap().x, 2.0, "then as written");
        assert_eq!(animation.pose(0, 8.0).unwrap().x, 9.0, "and out at the far end of the mirror");

        let from_rest = vec![at(0.0), at(5.0), at(0.0)];
        assert_eq!(first_settled_frame(&[from_rest], &bones), 0.0);
    }

    #[test]
    fn a_corner_at_rest_stays_where_it_is() {
        let p = two_bones(vec![Pose { x: 10.0, y: 0.0, angle: 1.0 }; 3], Playback::Loop);
        assert_eq!(bent_x_y(&p, &[], 1.0), (20.0, 0.0));
    }

    /// A corner ten pixels past its bone, turned a quarter about that bone.
    #[test]
    fn a_bone_turns_its_corners_about_itself() {
        let turn = std::f32::consts::FRAC_PI_2;
        let p = two_bones(vec![Pose { x: 10.0, y: 0.0, angle: turn }; 3], Playback::Loop);
        let (x, y) = bent_x_y(&p, &[PLAY], 0.0);
        assert!((x - 10.0).abs() < 1e-3 && (y - 10.0).abs() < 1e-3, "{x} {y}");
    }

    #[test]
    fn a_mirror_comes_back_and_a_blend_scales_what_it_adds() {
        let track = vec![
            Pose { x: 10.0, y: 0.0, angle: 0.0 },
            Pose { x: 20.0, y: 0.0, angle: 0.0 },
            Pose { x: 30.0, y: 0.0, angle: 0.0 },
        ];
        let p = two_bones(track, Playback::Mirror);
        assert_eq!(bent_x_y(&p, &[PLAY], 1.0).0, 30.0);
        assert_eq!(bent_x_y(&p, &[PLAY], 2.0).0, 40.0);
        assert_eq!(bent_x_y(&p, &[PLAY], 3.0).0, 30.0);
        assert_eq!(bent_x_y(&p, &[Playing { blend: 0.5, ..PLAY }], 2.0).0, 30.0);
        assert_eq!(bent_x_y(&p, &[Playing { rate: 2.0, ..PLAY }], 1.0).0, 40.0);
    }

    #[test]
    fn nonsense_is_refused_rather_than_read() {
        assert!(Puppet::parse(b"MDLV0017\0").is_none());
        assert!(Puppet::parse(b"not a puppet").is_none());
    }
}
