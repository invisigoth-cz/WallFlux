// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

//! A scene's text, laid out and drawn into a picture.
//!
//! The rules are read off what the editor wrote into scene.json, which is the
//! size each text came to with its font. Over four fonts the width less any
//! padding is the font's advance times `pointsize` times 4.16, and 300/72 is
//! 4.167: **a point size is points at 300 dots an inch**. Padding is in the
//! stored size only of the objects with effects -- the editor calls it *effect
//! padding*, room for them to draw into -- and is not there on any without.
//!
//! **An alignment is about the origin, not about the box.** `right` ends the
//! text on the origin, `left` starts it there, and only `center` straddles it:
//! Future Clock's four digits, each a whole clock behind a mask, land within
//! five pixels of their four tiles that way and hundreds off centred in the
//! box, and Dark Core's date stops running under the panel beside it. The box
//! is what the object's effects are stretched over, so a mask has to find the
//! picture exactly that size whenever the text fits.
//!
//! The picture is RGBA and carries its colours, which is what the engine's own
//! `materials/fonts/basefontrgba.json` draws.

use ab_glyph::{Font, FontVec, GlyphId, PxScale, ScaleFont, point};

/// Dots an inch over points an inch.
const PIXELS_PER_POINT: f32 = 300.0 / 72.0;
/// A script may write anything: a stop for one writing a book.
const MOST_CHARACTERS: usize = 4096;
const MOST_POINTS: f32 = 512.0;
/// The widest and tallest picture, at four bytes a pixel.
const MOST_PIXELS: f32 = 4096.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Align {
    Start,
    Middle,
    End,
}

impl Align {
    /// `left`/`right` or `top`/`bottom`; anything else, as the editor's
    /// default, is the middle.
    pub fn parse(word: &str) -> Self {
        match word {
            "left" | "top" => Self::Start,
            "right" | "bottom" => Self::End,
            _ => Self::Middle,
        }
    }

    /// Which end of the space the origin is on, from its middle.
    fn edge(self) -> f32 {
        match self {
            Self::Start => -1.0,
            Self::Middle => 0.0,
            Self::End => 1.0,
        }
    }

    /// Where something `length` long starts, in a space `room` either side
    /// of nought.
    fn place(self, length: f32, room: f32) -> f32 {
        match self {
            Self::Start => -room,
            Self::Middle => -length / 2.0,
            Self::End => room - length,
        }
    }
}

/// How one object wants its text.
#[derive(Debug, Clone)]
pub struct Style {
    pub points: f32,
    /// The box the editor measured, if the file says.
    pub size: Option<[f32; 2]>,
    /// Room kept clear inside the box for the object's effects.
    pub padding: f32,
    pub across: Align,
    pub down: Align,
    pub fill: [f32; 3],
    /// Drawn behind the text where the object asks for an opaque background.
    pub background: Option<[f32; 3]>,
    /// The width a line is broken at, in the scene's pixels.
    pub max_width: Option<f32>,
    pub max_rows: Option<usize>,
    /// Whether a text cut off by `max_rows` says so.
    pub ellipsis: bool,
}

pub struct Picture {
    pub width: u32,
    pub height: u32,
    /// Rows from the top, four bytes a pixel.
    pub rgba: Vec<u8>,
    /// Where on the picture the object's origin is, from the middle, in its
    /// own sizes, with y running up as a scene's does.
    pub anchor: [f32; 2],
}

/// The font an object names and the one standing in for what it lacks: a
/// `systemfont_` name, a font that is not there, a character it does not have.
pub struct Face {
    fonts: Vec<FontVec>,
}

impl Face {
    /// `None` when neither can be read, which is a text that cannot be drawn.
    pub fn new(own: Option<Vec<u8>>, spare: Option<Vec<u8>>) -> Option<Self> {
        let fonts: Vec<_> = [own, spare]
            .into_iter()
            .flatten()
            .filter_map(|bytes| FontVec::try_from_vec(bytes).ok())
            .collect();
        (!fonts.is_empty()).then_some(Self { fonts })
    }

    fn glyph(&self, c: char) -> (usize, GlyphId) {
        self.fonts
            .iter()
            .enumerate()
            .map(|(which, font)| (which, font.glyph_id(c)))
            .find(|(_, id)| id.0 != 0)
            .unwrap_or((0, GlyphId(0)))
    }

    /// The scale that makes an em `pixels` across. `PxScale` is the height
    /// from ascent to descent, and `pt_to_px_scale` assumes 96 dots an inch.
    fn scale(&self, which: usize, pixels: f32) -> PxScale {
        let font = &self.fonts[which];
        let per_em = font.units_per_em().unwrap_or_else(|| font.height_unscaled());
        PxScale::from(pixels * font.height_unscaled() / per_em)
    }

    fn line_width(&self, line: &str, pixels: f32) -> f32 {
        self.walk(line, pixels, |_, _, _| {})
    }

    /// Goes along a line, saying where each glyph sits, and answers with how
    /// far it went.
    fn walk(&self, line: &str, pixels: f32, mut each: impl FnMut(usize, GlyphId, f32)) -> f32 {
        let mut pen = 0.0;
        let mut last: Option<(usize, GlyphId)> = None;
        for c in line.chars() {
            let (which, id) = self.glyph(c);
            let font = self.fonts[which].as_scaled(self.scale(which, pixels));
            if let Some((before, previous)) = last
                && before == which
            {
                pen += font.kern(previous, id);
            }
            each(which, id, pen);
            pen += font.h_advance(id);
            last = Some((which, id));
        }
        pen
    }

    /// The text broken into the lines it is drawn as.
    fn lines(&self, text: &str, style: &Style, pixels: f32) -> Vec<String> {
        let mut lines = Vec::new();
        for paragraph in text.split('\n') {
            let Some(most) = style.max_width else {
                lines.push(paragraph.to_owned());
                continue;
            };
            let mut line = String::new();
            for word in paragraph.split(' ') {
                let longer = if line.is_empty() { word.to_owned() } else { format!("{line} {word}") };
                if !line.is_empty() && self.line_width(&longer, pixels) > most {
                    lines.push(std::mem::replace(&mut line, word.to_owned()));
                } else {
                    line = longer;
                }
            }
            lines.push(line);
        }
        if let Some(most) = style.max_rows.filter(|most| lines.len() > (*most).max(1)) {
            lines.truncate(most.max(1));
            if style.ellipsis
                && let Some(last) = lines.last_mut()
            {
                last.push('…');
            }
        }
        lines
    }

    /// Draws `text`, whose picture has its centre on the middle of the box.
    pub fn draw(&self, text: &str, style: &Style) -> Picture {
        let text: String = text.chars().take(MOST_CHARACTERS).collect();
        let pixels = style.points.clamp(1.0, MOST_POINTS) * PIXELS_PER_POINT;
        let metrics = self.fonts[0].as_scaled(self.scale(0, pixels));
        let line_height = metrics.height() + metrics.line_gap();
        let lines = self.lines(&text, style, pixels);
        let widths: Vec<f32> = lines.iter().map(|line| self.line_width(line, pixels)).collect();
        let block = line_height * lines.len() as f32;
        let measured = [widths.iter().copied().fold(0.0, f32::max), block];

        // Half the box, grown for a text longer than the one the editor
        // measured, and half of what is left of it inside the padding.
        let half: [f32; 2] = std::array::from_fn(|i| {
            let box_half = style.size.map_or(0.0, |size| size[i].abs() / 2.0);
            box_half.max(measured[i] / 2.0 + style.padding).min(MOST_PIXELS / 2.0)
        });
        let room = half.map(|h| (h - style.padding).max(0.0));

        let top = style.down.place(block, room[1]);
        let starts: Vec<f32> = widths.iter().map(|w| style.across.place(*w, room[0])).collect();
        let width = (half[0] * 2.0).ceil().max(1.0) as u32;
        let height = (half[1] * 2.0).ceil().max(1.0) as u32;
        let anchor = [
            style.across.edge() * room[0] / width as f32,
            -style.down.edge() * room[1] / height as f32,
        ];

        let mut coverage = vec![0u8; width as usize * height as usize];
        let middle = [width as f32 / 2.0, height as f32 / 2.0];
        for (row, line) in lines.iter().enumerate() {
            // The gap between lines goes above a line rather than under it:
            // under it, every text sat that much too high against Wallpaper
            // Engine on Windows -- Future Clock's digits off their tile's lamps.
            // ponytail: and a twentieth of an em lower still, measured the
            // same on three fonts whose metrics have nothing in common. Where
            // it comes from was not found; a fourth font disagreeing would say.
            let baseline = middle[1]
                + top
                + line_height * row as f32
                + metrics.line_gap()
                + metrics.ascent()
                + pixels / 20.0;
            let start = middle[0] + starts[row];
            self.walk(line, pixels, |which, id, pen| {
                // Past the edge of the most there can be: outlining what
                // nobody sees is how a book would stop the render thread.
                if start + pen >= width as f32 {
                    return;
                }
                let font = &self.fonts[which];
                let glyph = id.with_scale_and_position(self.scale(which, pixels), point(start + pen, baseline));
                let Some(outlined) = font.outline_glyph(glyph) else {
                    return;
                };
                let bounds = outlined.px_bounds();
                outlined.draw(|x, y, amount| {
                    let x = x as i64 + bounds.min.x as i64;
                    let y = y as i64 + bounds.min.y as i64;
                    if (0..width as i64).contains(&x) && (0..height as i64).contains(&y) {
                        let at = y as usize * width as usize + x as usize;
                        coverage[at] = coverage[at].max((amount.clamp(0.0, 1.0) * 255.0) as u8);
                    }
                });
            });
        }

        let byte = |channel: f32| (channel.clamp(0.0, 1.0) * 255.0).round() as u8;
        let mut rgba = Vec::with_capacity(coverage.len() * 4);
        for amount in coverage {
            let share = amount as f32 / 255.0;
            match style.background {
                // The fill's colour under a transparent pixel too, so the
                // edge of a letter does not fade through black when the
                // picture is filtered.
                None => rgba.extend([byte(style.fill[0]), byte(style.fill[1]), byte(style.fill[2]), amount]),
                Some(behind) => {
                    let mixed: [f32; 3] =
                        std::array::from_fn(|i| behind[i] + (style.fill[i] - behind[i]) * share);
                    rgba.extend([byte(mixed[0]), byte(mixed[1]), byte(mixed[2]), 255]);
                }
            }
        }
        Picture { width, height, rgba, anchor }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wpe_format::assets::Assets;

    fn face() -> Option<Face> {
        let assets = Assets::locate()?;
        Face::new(assets.get("fonts/RobotoMono-Regular.ttf"), None)
    }

    fn style() -> Style {
        Style {
            points: 18.0,
            size: None,
            padding: 0.0,
            across: Align::Middle,
            down: Align::Middle,
            fill: [1.0; 3],
            background: None,
            max_width: None,
            max_rows: None,
            ellipsis: false,
        }
    }

    /// The columns with any ink in them.
    fn inked(picture: &Picture) -> (u32, u32) {
        let mut columns = (u32::MAX, 0);
        for (i, pixel) in picture.rgba.chunks(4).enumerate() {
            if pixel[3] > 128 {
                let x = i as u32 % picture.width;
                columns = (columns.0.min(x), columns.1.max(x));
            }
        }
        columns
    }

    /// RobotoMono's advance is 0.6 of its em, and an em is the points at 300
    /// dots an inch: five characters at 18 points are 225 pixels wide.
    #[test]
    fn a_point_is_a_point_at_three_hundred_dots_an_inch() {
        let Some(face) = face() else { return };
        let picture = face.draw("12:34", &style());
        assert!((picture.width as i32 - 225).abs() <= 1, "{}", picture.width);
    }

    #[test]
    fn a_text_sits_where_its_box_says() {
        let Some(face) = face() else { return };
        let boxed = Style { size: Some([1000.0, 200.0]), ..style() };
        let left = face.draw("00", &Style { across: Align::Start, ..boxed.clone() });
        let right = face.draw("00", &Style { across: Align::End, ..boxed.clone() });
        let middle = face.draw("00", &boxed);
        assert_eq!((left.width, left.height), (1000, 200));
        assert!(inked(&left).1 < 150, "{:?}", inked(&left));
        assert!(inked(&right).0 > 850, "{:?}", inked(&right));
        let (from, to) = inked(&middle);
        assert!((from as i32 + to as i32 - 1000).abs() < 12, "{from} {to}");
    }

    /// The text's edge is on the origin whatever the box's size; padding sits
    /// beyond it, for an effect to draw into.
    #[test]
    fn an_aligned_text_has_its_edge_on_the_origin() {
        let Some(face) = face() else { return };
        let boxed = Style { size: Some([1000.0, 200.0]), padding: 100.0, ..style() };
        let right = face.draw("0", &Style { across: Align::End, ..boxed.clone() });
        assert!((right.anchor[0] - 0.4).abs() < 1e-3, "{:?}", right.anchor);
        let left = face.draw("0", &Style { across: Align::Start, ..boxed.clone() });
        assert!((left.anchor[0] + 0.4).abs() < 1e-3, "{:?}", left.anchor);
        assert_eq!(face.draw("0", &boxed).anchor, [0.0, 0.0]);
        let top = face.draw("0", &Style { down: Align::Start, ..boxed });
        assert!(top.anchor[1] > 0.0, "{:?}", top.anchor);
    }

    #[test]
    fn padding_keeps_an_aligned_text_off_the_edge() {
        let Some(face) = face() else { return };
        let padded = Style {
            size: Some([1000.0, 200.0]),
            padding: 101.0,
            across: Align::Start,
            ..style()
        };
        assert!(inked(&face.draw("0", &padded)).0 >= 101);
    }

    /// A date written longer than the one the editor measured grows the
    /// picture, cutting nothing off.
    #[test]
    fn a_text_longer_than_its_box_is_not_cut_off() {
        let Some(face) = face() else { return };
        let boxed = Style { size: Some([100.0, 100.0]), ..style() };
        let picture = face.draw("WEDNESDAY", &boxed);
        assert!(picture.width > 400, "{}", picture.width);
        let (from, to) = inked(&picture);
        assert!(from < 12 && to > picture.width - 12, "{from} {to}");
    }

    #[test]
    fn a_line_breaks_at_the_width_and_stops_at_the_rows() {
        let Some(face) = face() else { return };
        let one = face.draw("00 00 00", &style()).height;
        let broken = Style { max_width: Some(200.0), ..style() };
        let three = face.draw("00 00 00", &broken).height;
        assert!(three >= one * 3 - 3, "{one} {three}");
        let cut = Style { max_rows: Some(2), ellipsis: true, ..broken };
        let two = face.draw("00 00 00", &cut).height;
        assert!(two < three, "{two} {three}");
    }

    #[test]
    fn a_script_writing_a_book_does_not_take_the_machine_with_it() {
        let Some(face) = face() else { return };
        let picture = face.draw(&"0".repeat(1 << 20), &Style { points: 1e9, ..style() });
        assert!(picture.width <= MOST_PIXELS as u32);
        assert_eq!(picture.rgba.len(), picture.width as usize * picture.height as usize * 4);
    }

    #[test]
    fn an_opaque_background_fills_the_picture() {
        let Some(face) = face() else { return };
        let picture = face.draw("0", &Style { background: Some([1.0, 0.0, 0.0]), ..style() });
        assert_eq!(&picture.rgba[..4], &[255, 0, 0, 255]);
    }

    #[test]
    fn no_font_at_all_is_said_rather_than_drawn() {
        assert!(Face::new(None, Some(vec![0; 8])).is_none());
    }
}
