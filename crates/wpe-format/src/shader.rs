// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 invisigoth
// With the Steamworks SDK linking exception; see LICENSE-EXCEPTION.

//! Turning a wallpaper's shader into one a GLES 3 driver will accept.
//!
//! Wallpaper Engine's shaders are GLSL, which is why this renderer is built on
//! GL at all, but they are not GLSL a modern driver compiles as-is. Three
//! things are missing, and the engine supplies all three at compile time:
//!
//! - `#include "common.h"`, which is not a GLSL directive.
//! - A set of helpers the shaders call but no shipped file defines:
//!   `texSample2D`, `mul`, `saturate`, `frac` and the `CAST*` family. These
//!   come from the shaders' Direct3D ancestry.
//! - The `#define`s that select branches, which come from the material's
//!   combos rather than from the shader.
//!
//! On top of that the sources are written in the older dialect, with `varying`
//! and `gl_FragColor`, so the prelude bridges those to `in`/`out` rather than
//! rewriting hundreds of shaders.

use std::collections::BTreeMap;

/// What a shader asked for but could not be given.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Missing {
    /// An `#include` naming a file neither the wallpaper nor the installation has.
    Include { name: String, line: usize },
    /// Includes nested far enough to look like a cycle.
    IncludeTooDeep { name: String },
}

impl std::fmt::Display for Missing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Include { name, line } => write!(f, "line {line}: no such include {name:?}"),
            Self::IncludeTooDeep { name } => write!(f, "include {name:?} nests too deeply"),
        }
    }
}

/// Which pipeline stage a shader is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Vertex,
    Fragment,
}

const MAX_INCLUDE_DEPTH: usize = 16;

/// The compatibility layer every shader is compiled with.
///
/// `mul` is a plain multiply in both argument orders, because GLSL's `*`
/// carries the same two meanings Direct3D's `mul` does: a vector on the left is
/// a row vector, on the right a column vector. Shaders here use both.
const PRELUDE: &str = r#"
#define saturate(x) clamp((x), 0.0, 1.0)
#define mul(a, b) ((a) * (b))
#define lerp(a, b, t) mix((a), (b), (t))
#define frac(x) fract(x)
#define fmod(a, b) mod((a), (b))
#define atan2(y, x) atan((y), (x))
#define ddx(x) dFdx(x)
#define ddy(x) dFdy(x)
#define rsqrt(x) inversesqrt(x)
#define CASTF(x) float(x)
#define CASTI(x) int(x)
#define CASTU(x) uint(x)
#define CAST2(x) vec2(x)
#define CAST3(x) vec3(x)
#define CAST4(x) vec4(x)
#define CAST2X2(x) mat2(x)
#define CAST3X3(x) mat3(x)
#define CAST4X4(x) mat4(x)
#define texSample2D(s, uv) texture((s), vec2(uv))
#define texSample2DLod(s, uv, lod) textureLod((s), vec2(uv), (lod))
#define texSample2DCompare(s, uvz) (texture((s), (uvz).xy).r < (uvz).z ? 0.0 : 1.0)
#define texSample3D(s, uvw) texture((s), (uvw))
#define texSample3DLod(s, uvw, lod) textureLod((s), (uvw), (lod))
#define texSampleCube(s, dir) texture((s), (dir))
#define MAKE_SAMPLER2D_ARGUMENT(name) sampler2D name

// Reserved in GLSL ES 3 but used as ordinary names in these shaders. Renaming
// them with a define keeps every use consistent without editing the sources.
#define sample wf_sample
#define filter wf_filter
#define input wf_input
#define output wf_output
#define common wf_common
#define active wf_active
#define partition wf_partition
"#;

/// Bridges the older dialect the sources are written in.
const VERTEX_PRELUDE: &str = "#define varying out\n#define attribute in\n";
const FRAGMENT_PRELUDE: &str =
    "#define varying in\nout vec4 wf_FragColor;\n#define gl_FragColor wf_FragColor\n";

/// Builds the source to hand to the driver.
///
/// `resolve` is given an include's name and returns its contents, so the caller
/// decides whether a wallpaper's own copy or the installation's wins.
pub fn preprocess(
    source: &str,
    stage: Stage,
    combos: &BTreeMap<String, String>,
    mut resolve: impl FnMut(&str) -> Option<String>,
) -> (String, Vec<Missing>) {
    let mut out = String::with_capacity(source.len() * 2);
    let mut missing = Vec::new();

    out.push_str("#version 330 core\n");
    out.push_str(match stage {
        Stage::Vertex => VERTEX_PRELUDE,
        Stage::Fragment => FRAGMENT_PRELUDE,
    });
    out.push_str(PRELUDE);

    // Combos select branches inside the shader. Shaders only ever test these
    // uppercase, so they are emitted that way whatever case the material used.
    // The body is built first: a shader declares defaults for its own options,
    // and those declarations can come from an include.
    let mut body = String::with_capacity(source.len() * 2);
    expand(source, 0, &mut body, &mut missing, &mut resolve);

    // What the material asked for wins; anything it left alone falls back to
    // the default the shader declares for itself.
    let mut settings = combo_defaults(&body);
    for (name, value) in combos {
        settings.insert(name.to_ascii_uppercase(), value.clone());
    }
    for (name, value) in &settings {
        if !defines(&body, name) {
            out.push_str(&format!("#define {name} {value}\n"));
        }
    }

    // Shaders branch on which language they are being compiled for. Some set
    // this themselves, so it is only supplied when one does not.
    if !defines(&body, "GLSL") {
        out.push_str("#define GLSL 1\n");
    }

    out.push_str(&hoist_uniforms(&body));
    (out, missing)
}

/// Lifts every `uniform` declaration above the body it was written in.
///
/// Wallpaper Engine rewrites these into constant buffers and texture registers
/// at the top of the file, so a shader may declare `g_Texture0` *after* the
/// include whose functions sample it -- which is what `common_blur.h` does,
/// and what every blur built on it does. GLSL wants a name declared before it
/// is used, so the same file will not compile until the declarations are moved
/// up. The guards they were written under come with them, since whether a
/// uniform exists at all is often decided by an option.
fn hoist_uniforms(body: &str) -> String {
    // Each level of nesting remembers the branches already taken, so an
    // `#else` reads as "none of those", which is the condition the moved line
    // has to be put back under.
    struct Level {
        taken: Vec<String>,
        current: Option<String>,
    }
    let mut stack: Vec<Level> = Vec::new();
    let mut lifted = String::new();
    let mut rest = String::with_capacity(body.len());

    for line in body.lines() {
        let trimmed = line.trim_start();
        let directive = |word: &str| trimmed.strip_prefix(word).map(str::trim);
        if let Some(rest) = directive("#ifdef ") {
            stack.push(Level {
                taken: Vec::new(),
                current: Some(format!("defined({rest})")),
            });
        } else if let Some(rest) = directive("#ifndef ") {
            stack.push(Level {
                taken: Vec::new(),
                current: Some(format!("!defined({rest})")),
            });
        } else if let Some(rest) = directive("#if ") {
            stack.push(Level {
                taken: Vec::new(),
                current: Some(rest.to_owned()),
            });
        } else if let Some(rest) = directive("#elif ") {
            if let Some(level) = stack.last_mut() {
                level.taken.extend(level.current.take());
                level.current = Some(rest.to_owned());
            }
        } else if trimmed.starts_with("#else") {
            if let Some(level) = stack.last_mut() {
                level.taken.extend(level.current.take());
            }
        } else if trimmed.starts_with("#endif") {
            stack.pop();
        } else if trimmed.starts_with("uniform ") && trimmed.contains(';') {
            for level in &stack {
                for gone in &level.taken {
                    lifted.push_str(&format!("#if !({gone})\n"));
                }
                if let Some(now) = &level.current {
                    lifted.push_str(&format!("#if ({now})\n"));
                }
            }
            lifted.push_str(trimmed);
            lifted.push('\n');
            for level in &stack {
                for _ in level.taken.iter().chain(level.current.iter()) {
                    lifted.push_str("#endif\n");
                }
            }
            continue;
        }
        rest.push_str(line);
        rest.push('\n');
    }
    lifted + &rest
}

/// The options a shader declares for itself, with the values to use when a
/// material does not set them.
///
/// These ride in comments next to the code they control:
///
/// ```text
/// // [COMBO] {"combo":"BLENDMODE","type":"imageblending","default":9}
/// ```
///
/// Without them an option used as a value rather than in an `#if` is simply an
/// undeclared identifier, which is where a blend mode goes missing.
pub fn combo_defaults(source: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for line in source.lines() {
        let line = line.trim_start();
        if !line.starts_with("//") {
            continue;
        }
        let upper = line.to_ascii_uppercase();
        let Some(at) = upper.find("[COMBO]") else {
            continue;
        };
        let Some(start) = line[at..].find('{').map(|i| at + i) else {
            continue;
        };
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&line[start..]) else {
            continue;
        };
        let (Some(name), Some(default)) = (value.get("combo"), value.get("default")) else {
            continue;
        };
        let Some(name) = name.as_str() else { continue };
        let default = match default {
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        };
        out.insert(name.to_ascii_uppercase(), default);
    }
    out
}

/// A uniform a shader declares, with whatever rides in the comment beside it.
#[derive(Debug, Clone, PartialEq)]
pub struct Uniform {
    pub name: String,
    /// What to use when nothing sets it. A uniform nobody sets reads as zero
    /// on the GPU, so a shader that multiplies by one of these draws black
    /// until its declared default is fed back in.
    pub default: Option<serde_json::Value>,
    /// The name a *material* sets it by, which is not the uniform's own name.
    /// Materials write `ui_editor_properties_overbright`, never `g_Overbright`
    /// -- of the 210 constants in the library, 201 are written this way.
    pub label: Option<String>,
    /// Whether this is a texture rather than a value. A sampler's default
    /// names a texture to load, not a number to set.
    pub sampler: bool,
    /// A switch the shader keeps the use of this uniform behind, which turns
    /// on when something is bound to it. Masks and normal maps are written
    /// this way, and the code that reads them is compiled out until then.
    pub combo: Option<String>,
}

/// The uniforms a shader declares, read from the comments beside them:
///
/// ```text
/// uniform float g_Overbright; // {"material":"ui_editor_properties_overbright","default":1.0}
/// ```
///
/// Samplers are included, flagged, because their default names a texture the
/// pass still needs: a shader that samples clouds and is given none draws a
/// sky of flat white.
pub fn uniforms(source: &str) -> Vec<Uniform> {
    let mut out = Vec::new();
    for line in source.lines() {
        let line = line.trim_start();
        let Some(rest) = line.strip_prefix("uniform") else {
            continue;
        };
        let Some((declaration, comment)) = rest.split_once("//") else {
            continue;
        };
        let mut words = declaration.split_whitespace();
        let (Some(kind), Some(name)) = (words.next(), words.next()) else {
            continue;
        };
        let Some(start) = comment.find('{') else {
            continue;
        };
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&comment[start..]) else {
            continue;
        };
        out.push(Uniform {
            name: name.trim_end_matches([';', '[']).trim().to_owned(),
            default: value.get("default").cloned(),
            label: value
                .get("material")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned),
            sampler: kind.starts_with("sampler"),
            combo: value
                .get("combo")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned),
        });
    }
    out
}

/// Whether the source defines `name` itself.
fn defines(source: &str, name: &str) -> bool {
    source.lines().any(|line| {
        line.trim_start()
            .strip_prefix("#define")
            .map(str::trim_start)
            .and_then(|rest| {
                rest.split(|c: char| !c.is_alphanumeric() && c != '_')
                    .next()
            })
            == Some(name)
    })
}

/// Names a shader tests in an `#if` but never defines.
///
/// Desktop GLSL treats an undefined name in such an expression as zero; GLSL ES
/// rejects it outright, and these shaders lean on the desktop behaviour for
/// every option a material did not set. Giving those a zero restores it.
///
fn expand(
    source: &str,
    depth: usize,
    out: &mut String,
    missing: &mut Vec<Missing>,
    resolve: &mut impl FnMut(&str) -> Option<String>,
) {
    for (number, line) in source.lines().enumerate() {
        match include_name(line) {
            Some(name) if depth >= MAX_INCLUDE_DEPTH => {
                missing.push(Missing::IncludeTooDeep {
                    name: name.to_owned(),
                });
            }
            Some(name) => match resolve(name) {
                Some(text) => expand(&text, depth + 1, out, missing, resolve),
                None => {
                    missing.push(Missing::Include {
                        name: name.to_owned(),
                        line: number + 1,
                    });
                }
            },
            // A `#version` inside a wallpaper's own shader would land after the
            // one above and be rejected, so it is dropped rather than passed on.
            None if line.trim_start().starts_with("#version") => {}
            // `#require` is the engine's own directive, not GLSL's. The driver
            // rejects it outright, and what it asks for is already in scope.
            None if line.trim_start().starts_with("#require") => {}
            None => {
                out.push_str(line);
                out.push('\n');
            }
        }
    }
}

/// Makes the vertex stage write a varying as wide as the fragment stage reads
/// it.
///
/// A wallpaper can ship a vertex shader declaring `varying vec2 v_TexCoord`
/// against a fragment shader declaring `vec3` -- the component nobody reads is
/// simply missing. Direct3D links that and leaves the odd component undefined;
/// GLSL refuses the program outright, so an effect written that way draws
/// nothing at all. Widening the declaration is the same bargain: the extra
/// component is undefined, and the fragment shader never looks at it.
///
/// The vertex shader has to fill the varying through a swizzle for this to
/// compile, which is what the ones in the wild do. Where it assigns the whole
/// name, nothing is touched and the program fails to link as before.
pub fn widen_varyings(vertex: &mut String, fragment: &str) {
    let wanted: BTreeMap<&str, &str> = fragment
        .lines()
        .filter_map(varying_decl)
        .map(|(ty, name)| (name, ty))
        .collect();
    if wanted.is_empty() {
        return;
    }
    let mut out = String::with_capacity(vertex.len());
    for line in vertex.lines() {
        match varying_decl(line) {
            Some((ty, name))
                if wanted
                    .get(name)
                    .is_some_and(|wider| width(wider) > width(ty))
                    && !assigns_whole(vertex, name) =>
            {
                out.push_str(&line.replacen(ty, wanted[name], 1));
            }
            _ => out.push_str(line),
        }
        out.push('\n');
    }
    *vertex = out;
}

/// The type and name a `varying vec2 v_TexCoord;` line declares.
fn varying_decl(line: &str) -> Option<(&str, &str)> {
    let rest = line.trim_start().strip_prefix("varying")?;
    let (decl, _) = rest.split_once(';')?;
    let mut words = decl.split_whitespace();
    let (ty, name) = (words.next()?, words.next()?);
    words.next().is_none().then_some((ty, name))
}

/// How many components a type carries, or nought for one this does not widen.
fn width(ty: &str) -> usize {
    match ty {
        "float" => 1,
        "vec2" => 2,
        "vec3" => 3,
        "vec4" => 4,
        _ => 0,
    }
}

/// Whether the shader ever writes the whole varying rather than a swizzle of
/// it, which is what a wider declaration would stop compiling.
fn assigns_whole(source: &str, name: &str) -> bool {
    source.match_indices(name).any(|(at, _)| {
        let before = source[..at].chars().next_back();
        let after = source[at + name.len()..].trim_start();
        before.is_none_or(|c| !c.is_alphanumeric() && c != '_' && c != '.')
            && after.starts_with('=')
            && !after.starts_with("==")
    })
}

/// The file named by an `#include "..."` line, if the line is one.
fn include_name(line: &str) -> Option<&str> {
    let rest = line.trim_start().strip_prefix("#include")?;
    let rest = rest.trim_start();
    let rest = rest.strip_prefix('"')?;
    rest.split('"').next().filter(|n| !n.is_empty())
}

/// Writes out the conversions HLSL makes by itself and GLSL will not: a `vec4`
/// put into a `vec3`, a `vec2` into a `float`. Every initializer, assignment
/// and `return` of a plain type gets a constructor of that type, which is a
/// no-op where the types already agree and shortens or spreads where not --
/// just what HLSL does.
///
/// A pass over statements rather than a parser: what it cannot read as one of
/// those three it leaves alone, and the compiler says so.
///
/// The value itself is followed through too, as far as its names' types are
/// known: where HLSL takes a `vec4` from a `vec2` it shortens the `vec4`
/// first, so the longer side gets a swizzle down to the shorter one. A value
/// with a name or a call of unknown type is left as it was.
///
/// And `max(0, v)` becomes `max(v, 0.0)`, `min` likewise: HLSL spreads the
/// number over the vector, GLSL has only the vector-and-number way round.
/// Lightning Thunderstorm's `pulse` wrote `max(0, albedo.rgb)`.
pub fn spell_out_conversions(source: &str) -> String {
    let turned = number_last(source);
    let source = turned.as_str();
    let mut types: BTreeMap<String, &'static str> = BTreeMap::new();
    let mut returns: Option<&'static str> = None;
    let mut functions: BTreeMap<String, &'static str> = BTreeMap::new();
    let mut out = String::with_capacity(source.len() + source.len() / 8);
    let mut rest = source;
    while let Some(end) = next_mark(rest) {
        let (statement, tail) = rest.split_at(end);
        let mark = &tail[..1];
        rest = &tail[1..];
        // What comes first may be comments and directives, kept as they are.
        let head = code_starts_at(statement);
        out.push_str(&statement[..head]);
        let code = &statement[head..];
        match mark {
            "{" => {
                if let Some((kind, parameters)) = function_header(code) {
                    returns = kind;
                    if let (Some(kind), Some(name)) =
                        (kind, code.split('(').next().and_then(|b| b.split_whitespace().last()))
                    {
                        functions.insert(name.to_owned(), kind);
                    }
                    for (name, kind) in parameters {
                        types.insert(name, kind);
                    }
                }
                out.push_str(code);
            }
            ";" => out.push_str(&convert_statement(code, &mut types, &functions, returns)),
            _ => out.push_str(code),
        }
        out.push_str(mark);
    }
    out.push_str(rest);
    out
}

/// `max(<number>, x)` and `min(<number>, x)` turned round, the number a float.
fn number_last(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut rest = source;
    while let Some(at) = rest.find(['m']) {
        let (before, here) = rest.split_at(at);
        out.push_str(before);
        let named = ["max(", "min("].iter().find(|f| here.starts_with(**f));
        let starts_a_word = !out.ends_with(|c: char| c.is_alphanumeric() || c == '_');
        if let (Some(function), true) = (named, starts_a_word)
            && let Some(turned) = turned_round(&here[4..])
        {
            out.push_str(&function[..3]);
            out.push('(');
            out.push_str(&turned.0);
            rest = &here[4 + turned.1..];
            continue;
        }
        out.push('m');
        rest = &here[1..];
    }
    out.push_str(rest);
    out
}

/// After `max(`: a number, a comma and one more argument up to the closing
/// bracket, handed back the other way round with how much of `after` it took.
fn turned_round(after: &str) -> Option<(String, usize)> {
    let comma = after.find(',')?;
    let number = after[..comma].trim();
    let digits = number.trim_end_matches(['f', 'F']);
    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit() || c == '.') {
        return None;
    }
    let mut depth = 0;
    for (i, c) in after[comma + 1..].char_indices() {
        match c {
            '(' => depth += 1,
            ',' if depth == 0 => return None,
            ')' if depth == 0 => {
                let other = after[comma + 1..comma + 1 + i].trim();
                let float = if digits.contains('.') { digits.to_owned() } else { format!("{digits}.0") };
                return Some((format!("{other}, {float})"), comma + 1 + i + 1));
            }
            ')' => depth -= 1,
            _ => {}
        }
    }
    None
}

/// The next `;`, `{` or `}` that is not in a `//` comment.
fn next_mark(text: &str) -> Option<usize> {
    let mut at = 0;
    while let Some(found) = text[at..].find([';', '{', '}', '/']) {
        let mark = at + found;
        if !text[mark..].starts_with('/') {
            return Some(mark);
        }
        at = match text[mark..].starts_with("//") {
            true => mark + text[mark..].find('\n')?,
            false => mark + 1,
        };
    }
    None
}

/// Where the code in a statement starts, past comments and `#` lines.
fn code_starts_at(statement: &str) -> usize {
    let mut at = 0;
    loop {
        let rest = &statement[at..];
        let trimmed = rest.trim_start();
        let skipped = rest.len() - trimmed.len();
        if trimmed.starts_with('#') || trimmed.starts_with("//") {
            match trimmed.find('\n') {
                Some(line) => at += skipped + line + 1,
                None => return statement.len(),
            }
        } else {
            return at + skipped;
        }
    }
}

fn plain_type(word: &str) -> Option<&'static str> {
    ["float", "vec2", "vec3", "vec4"].into_iter().find(|kind| *kind == word)
}

fn is_name(word: &str) -> bool {
    !word.is_empty()
        && word.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !word.starts_with(|c: char| c.is_ascii_digit())
}

/// `vec3 name(float a, in vec2 b)`: what it returns, and its parameters.
#[allow(clippy::type_complexity)]
fn function_header(code: &str) -> Option<(Option<&'static str>, Vec<(String, &'static str)>)> {
    let (before, after) = code.split_once('(')?;
    let mut words = before.split_whitespace();
    let kind = words.next()?;
    if !is_name(words.next()?) || matches!(kind, "if" | "for" | "while" | "else" | "switch") {
        return None;
    }
    let parameters = after
        .rsplit_once(')')?
        .0
        .split(',')
        .filter_map(|parameter| {
            let mut words = parameter.split_whitespace().rev();
            let name = words.next()?;
            Some((name.to_owned(), plain_type(words.next()?)?))
        })
        .collect();
    Some((plain_type(kind), parameters))
}

/// Where the assignment is at the top of an expression, and how long its
/// operator is.
fn assignment(code: &str) -> Option<(usize, usize)> {
    let bytes = code.as_bytes();
    let mut depth = 0;
    for (at, byte) in bytes.iter().enumerate() {
        match byte {
            b'(' | b'[' => depth += 1,
            b')' | b']' => depth -= 1,
            b'=' if depth == 0 => {
                if bytes.get(at + 1) == Some(&b'=') {
                    return None;
                }
                return match at.checked_sub(1).map(|before| bytes[before]) {
                    Some(b'+' | b'-' | b'*' | b'/') => Some((at - 1, 2)),
                    Some(b'<' | b'>' | b'!' | b'=') => None,
                    _ => Some((at, 1)),
                };
            }
            _ => {}
        }
    }
    None
}

fn convert_statement(
    code: &str,
    types: &mut BTreeMap<String, &'static str>,
    functions: &BTreeMap<String, &'static str>,
    returns: Option<&'static str>,
) -> String {
    let known = types.clone();
    let fit = |value: &str| Fit::new(value, &known, functions).map_or_else(|| value.to_owned(), |t| t);
    let wrap = |kind: &str, left: &str, value: &str| format!("{left} {kind}({})", fit(value.trim()));
    if let Some(value) = code.strip_prefix("return").filter(|v| v.starts_with(char::is_whitespace))
        && let Some(kind) = returns
    {
        return format!("return {kind}({})", fit(value.trim()));
    }
    let assigned = assignment(code);
    let left = assigned.map_or(code, |(at, _)| &code[..at]);
    let mut words: Vec<&str> = left.split_whitespace().collect();
    while let Some(word) = words.first()
        && matches!(*word, "const" | "uniform" | "varying" | "attribute" | "in" | "out" | "inout")
    {
        words.remove(0);
    }
    // A declaration, which is also how a name's type comes to be known.
    if let [kind, name] = words.as_slice()
        && let Some(kind) = plain_type(kind)
        && is_name(name)
    {
        types.insert((*name).to_owned(), kind);
        if let Some((at, width)) = assigned {
            return wrap(kind, &code[..at + width], &code[at + width..]);
        }
        return code.to_owned();
    }
    // `float a, b;`, or one this does not follow: the names are still noted.
    if let Some(first) = words.first().and_then(|word| plain_type(word)) {
        for name in left.split(',').map(|part| part.split_whitespace().last().unwrap_or("")) {
            if is_name(name) {
                types.insert(name.to_owned(), first);
            }
        }
        return code.to_owned();
    }
    let (Some((at, width)), [target]) = (assigned, words.as_slice()) else {
        return code.to_owned();
    };
    let (name, swizzle) = target.split_once('.').unwrap_or((target, ""));
    let Some(kind) = types.get(name) else {
        return code.to_owned();
    };
    let kind = match swizzle.len() {
        0 => kind,
        _ if !swizzle.chars().all(|c| "xyzwrgba".contains(c)) => return code.to_owned(),
        1 => "float",
        2 => "vec2",
        3 => "vec3",
        4 => "vec4",
        _ => return code.to_owned(),
    };
    wrap(kind, &code[..at + width], &code[at + width..])
}

/// A fragment shader's inputs are read-only in GLSL, and HLSL hands its
/// function a copy that may be written: chromatic aberration does
/// `v_TexCoord += ...` before sampling. Each input the shader writes gets a
/// global of its own, filled from the input at the top of `main` and used
/// everywhere else. The copy is a `#define` beside the declaration, so an
/// input declared under an `#if` that is off copies nothing.
pub fn copy_written_inputs(fragment: &str) -> String {
    let written: Vec<(&str, &str)> = fragment
        .lines()
        .filter_map(varying_decl)
        .filter(|(_, name)| writes(fragment, name))
        .collect();
    if written.is_empty() {
        return fragment.to_owned();
    }
    let mut out = String::with_capacity(fragment.len() + 256);
    let mut in_main = false;
    for line in fragment.lines() {
        if let Some((kind, name)) = varying_decl(line).filter(|d| written.contains(d)) {
            out.push_str(&format!("{line}\n{kind} wf_{name};\n#define WF_COPY_{name} wf_{name} = {name}\n"));
            continue;
        }
        let mut line = line.to_owned();
        for (_, name) in &written {
            line = rename(&line, name, &format!("wf_{name}"));
        }
        if !in_main && let Some(at) = line.find("void main").and_then(|m| line[m..].find('{').map(|b| m + b + 1)) {
            in_main = true;
            out.push_str(&line[..at]);
            out.push('\n');
            for (_, name) in &written {
                out.push_str(&format!("#ifdef WF_COPY_{name}\nWF_COPY_{name};\n#endif\n"));
            }
            out.push_str(&line[at..]);
        } else {
            out.push_str(&line);
        }
        out.push('\n');
    }
    out
}

/// Whether `name`, whole or through a swizzle, is ever assigned to -- not
/// declared afresh, `vec4 timer = ...`, which in HLSL only hides the input.
fn writes(source: &str, name: &str) -> bool {
    source.match_indices(name).any(|(at, _)| {
        let before = source[..at].chars().next_back();
        let word_before = source[..at]
            .trim_end()
            .rsplit(|c: char| !c.is_ascii_alphanumeric() && c != '_')
            .next()
            .unwrap_or("");
        let declared = !word_before.is_empty() && !matches!(word_before, "else" | "do" | "return");
        let after = &source[at + name.len()..];
        let after = after.trim_start_matches(|c: char| c == '.' || c.is_ascii_alphabetic());
        let after = after.trim_start();
        before.is_none_or(|c| !c.is_alphanumeric() && c != '_' && c != '.')
            && !declared
            && !source[at + name.len()..].starts_with(|c: char| c.is_alphanumeric() || c == '_')
            && (["+=", "-=", "*=", "/=", "++", "--"].iter().any(|op| after.starts_with(op))
                || (after.starts_with('=') && !after.starts_with("==")))
    })
}

/// Every whole-word `name` in `line` turned into `to`.
fn rename(line: &str, name: &str, to: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut rest = line;
    while let Some(at) = rest.find(name) {
        let before = rest[..at].chars().next_back().or_else(|| out.chars().next_back());
        let after = rest[at + name.len()..].chars().next();
        let word = |c: Option<char>| c.is_some_and(|c| c.is_alphanumeric() || c == '_');
        out.push_str(&rest[..at]);
        out.push_str(if word(before) || word(after) { name } else { to });
        rest = &rest[at + name.len()..];
    }
    out.push_str(rest);
    out
}

/// A value read through once for the widths of its vectors, with a swizzle
/// put on the longer side wherever two different widths meet in arithmetic.
/// `None` where it could not be read, which leaves it as it was.
struct Fit<'a> {
    tokens: Vec<(&'a str, &'a str)>,
    at: usize,
    types: &'a BTreeMap<String, &'static str>,
    functions: &'a BTreeMap<String, &'static str>,
}

/// A width of nought is a type not followed: a matrix, a bool, a name not known.
type Typed = (String, usize);

impl<'a> Fit<'a> {
    fn new(
        value: &'a str,
        types: &'a BTreeMap<String, &'static str>,
        functions: &'a BTreeMap<String, &'static str>,
    ) -> Option<String> {
        let mut fit = Fit { tokens: lex(value)?, at: 0, types, functions };
        let (text, _) = fit.ternary()?;
        (fit.at == fit.tokens.len()).then_some(text)
    }

    fn peek(&self) -> Option<&'a str> {
        self.tokens.get(self.at).map(|t| t.1)
    }

    /// The next token with the space before it.
    fn take(&mut self) -> Option<String> {
        let (space, token) = *self.tokens.get(self.at)?;
        self.at += 1;
        Some(format!("{space}{token}"))
    }

    fn ternary(&mut self) -> Option<Typed> {
        let (mut text, width) = self.binary(0)?;
        if self.peek() != Some("?") {
            return Some((text, width));
        }
        text += &self.take()?;
        let (yes, a) = self.ternary()?;
        text += &yes;
        (self.peek() == Some(":")).then_some(())?;
        text += &self.take()?;
        let (no, b) = self.ternary()?;
        Some((text + &no, if a == b { a } else { 0 }))
    }

    fn binary(&mut self, level: usize) -> Option<Typed> {
        const LEVELS: [&[&str]; 6] =
            [&["||"], &["&&"], &["==", "!="], &["<", ">", "<=", ">="], &["+", "-"], &["*", "/", "%"]];
        let Some(ops) = LEVELS.get(level) else {
            return self.unary();
        };
        let (mut text, mut width) = self.binary(level + 1)?;
        while self.peek().is_some_and(|t| ops.contains(&t)) {
            let op_text = self.take()?;
            let (right, other) = self.binary(level + 1)?;
            if level < 4 {
                (text, width) = (text + &op_text + &right, 0);
                continue;
            }
            let shorter = width.min(other);
            (text, width) = match (width, other) {
                (a, b) if a > 1 && b > 1 && a != b => {
                    let cut = |t: String, w: usize| match w > shorter {
                        true => cut_to(&t, shorter),
                        false => t,
                    };
                    (cut(text, width) + &op_text + &cut(right, other), shorter)
                }
                (0, _) | (_, 0) => (text + &op_text + &right, 0),
                (a, b) => (text + &op_text + &right, a.max(b)),
            };
        }
        Some((text, width))
    }

    fn unary(&mut self) -> Option<Typed> {
        if matches!(self.peek(), Some("-" | "+" | "!")) {
            let sign = self.take()?;
            let (text, width) = self.unary()?;
            return Some((sign + &text, width));
        }
        self.postfix()
    }

    fn postfix(&mut self) -> Option<Typed> {
        let (mut text, mut width) = self.primary()?;
        loop {
            match self.peek() {
                Some(".") => {
                    text += &self.take()?;
                    let member = self.peek()?;
                    text += &self.take()?;
                    width = match member.chars().all(|c| "xyzwrgba".contains(c)) && member.len() <= 4 {
                        true => member.len(),
                        false => 0,
                    };
                }
                Some("[") => {
                    text += &self.take()?;
                    text += &self.ternary()?.0;
                    (self.peek() == Some("]")).then_some(())?;
                    text += &self.take()?;
                    width = 0;
                }
                _ => return Some((text, width)),
            }
        }
    }

    fn primary(&mut self) -> Option<Typed> {
        let token = self.peek()?;
        if token == "(" {
            let mut text = self.take()?;
            let (inner, width) = self.ternary()?;
            (self.peek() == Some(")")).then_some(())?;
            text += &inner;
            return Some((text + &self.take()?, width));
        }
        let first = token.chars().next()?;
        if first.is_ascii_digit() || first == '.' {
            return Some((self.take()?, 1));
        }
        if !is_name(token) {
            return None;
        }
        let mut text = self.take()?;
        if self.peek() != Some("(") {
            let width = self.types.get(token).map_or(0, |kind| width(kind));
            return Some((text, width));
        }
        text += &self.take()?;
        let mut first_width = None;
        while self.peek() != Some(")") {
            let (argument, width) = self.ternary()?;
            first_width.get_or_insert(width);
            text += &argument;
            match self.peek()? {
                "," => text += &self.take()?,
                ")" => {}
                _ => return None,
            }
        }
        text += &self.take()?;
        let width = match token {
            "float" | "int" | "dot" | "length" | "distance" => 1,
            "vec2" => 2,
            "vec3" | "cross" => 3,
            "vec4" | "texture" | "textureLod" | "texSample2D" | "texSample2DLod" => 4,
            "abs" | "sign" | "floor" | "ceil" | "fract" | "frac" | "mod" | "fmod" | "min" | "max"
            | "clamp" | "saturate" | "mix" | "lerp" | "step" | "smoothstep" | "pow" | "exp" | "exp2"
            | "log" | "log2" | "sqrt" | "inversesqrt" | "rsqrt" | "sin" | "cos" | "tan" | "asin"
            | "acos" | "atan" | "atan2" | "normalize" | "reflect" => first_width.unwrap_or(0),
            _ => self.functions.get(token).map_or(0, |kind| width(kind)),
        };
        Some((text, width))
    }
}

/// `text` swizzled down to `width` components, in brackets unless it is
/// already one name or one bracket.
fn cut_to(text: &str, width: usize) -> String {
    let body = text.trim_start();
    let space = &text[..text.len() - body.len()];
    let swizzle = &"xyzw"[..width];
    if is_name(body) || (body.starts_with('(') && body.ends_with(')') && closes_at_end(body)) {
        format!("{space}{body}.{swizzle}")
    } else {
        format!("{space}({body}).{swizzle}")
    }
}

/// Whether the bracket a text opens with is the one it ends with.
fn closes_at_end(text: &str) -> bool {
    let mut depth = 0;
    for (at, c) in text.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return at == text.len() - 1;
                }
            }
            _ => {}
        }
    }
    false
}

/// A value cut into tokens, each with the space in front of it. `None` for
/// anything this does not read, such as a string or a `#`.
fn lex(text: &str) -> Option<Vec<(&str, &str)>> {
    let mut tokens = Vec::new();
    let mut rest = text;
    loop {
        let trimmed = rest.trim_start();
        let space = &rest[..rest.len() - trimmed.len()];
        let Some(c) = trimmed.chars().next() else {
            return Some(tokens);
        };
        let len = if c.is_ascii_alphabetic() || c == '_' {
            trimmed.find(|c: char| !c.is_ascii_alphanumeric() && c != '_').unwrap_or(trimmed.len())
        } else if c.is_ascii_digit() || (c == '.' && trimmed[1..].starts_with(|c: char| c.is_ascii_digit())) {
            let bytes = trimmed.as_bytes();
            let mut end = 0;
            while end < bytes.len() {
                let b = bytes[end];
                let exponent_sign = matches!(b, b'+' | b'-') && end > 0 && matches!(bytes[end - 1], b'e' | b'E');
                if b.is_ascii_alphanumeric() || b == b'.' || exponent_sign {
                    end += 1;
                } else {
                    break;
                }
            }
            end
        } else if ["<=", ">=", "==", "!=", "&&", "||"].iter().any(|op| trimmed.starts_with(op)) {
            2
        } else if "+-*/%<>!?:()[],.".contains(c) {
            1
        } else {
            return None;
        };
        tokens.push((space, &trimmed[..len]));
        rest = &trimmed[len..];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_hlsl_shortens_by_itself_is_written_out() {
        let out = spell_out_conversions(
            "// a; comment\nuniform vec4 g_A;\nvec3 f(vec2 p) {\n  float x = p;\n  vec3 y;\n  y = g_A;\n  y.xy = g_A;\n  x *= p;\n  for (int i = 0; i < 2; ++i) { y += g_A; }\n  return g_A;\n}\n",
        );
        assert!(out.contains("float x = float(p);"), "{out}");
        assert!(out.contains("y = vec3(g_A);"), "{out}");
        assert!(out.contains("y.xy = vec2(g_A);"), "{out}");
        assert!(out.contains("x *= float(p);"), "{out}");
        assert!(out.contains("y += vec3(g_A);"), "{out}");
        assert!(out.contains("return vec3(g_A);"), "{out}");
        assert!(out.contains("// a; comment"), "{out}");
    }

    #[test]
    fn a_longer_vector_is_shortened_where_two_widths_meet() {
        let out = spell_out_conversions(
            "uniform float u_r;\nvarying vec4 v_T;\nvoid main() {\n  vec4 t = v_T;\n  float p = 1.0;\n  vec2 a = v_T.xy - (u_r * t + p);\n  vec4 c = texSample2D(g_S, v_T.xy - u_r * t);\n  vec3 d = t * 2.0 + v_T.xyz;\n  vec2 e = unknown - t;\n}\n",
        );
        assert!(out.contains("vec2 a = vec2(v_T.xy - (u_r * t + p).xy);"), "{out}");
        assert!(out.contains("texSample2D(g_S, v_T.xy - (u_r * t).xy)"), "{out}");
        assert!(out.contains("vec3 d = vec3((t * 2.0).xyz + v_T.xyz);"), "{out}");
        assert!(out.contains("vec2 e = vec2(unknown - t);"), "{out}");
    }

    #[test]
    fn an_input_the_fragment_shader_writes_is_copied_first() {
        let out = copy_written_inputs(
            "varying vec4 v_TexCoord;\nvarying vec2 v_Kept;\nvec4 f() { return v_TexCoord; }\nvoid main() {\n  v_TexCoord.xy += v_Kept;\n  gl_FragColor = v_TexCoord + vec4(v_TexCoord2);\n}\n",
        );
        assert!(out.contains("varying vec4 v_TexCoord;\nvec4 wf_v_TexCoord;\n#define WF_COPY_v_TexCoord wf_v_TexCoord = v_TexCoord\n"), "{out}");
        assert!(out.contains("return wf_v_TexCoord;"), "{out}");
        assert!(out.contains("void main() {\n#ifdef WF_COPY_v_TexCoord\nWF_COPY_v_TexCoord;\n#endif\n"), "{out}");
        assert!(out.contains("wf_v_TexCoord.xy += v_Kept;"), "{out}");
        assert!(out.contains("wf_v_TexCoord + vec4(v_TexCoord2)"), "{out}");
        assert!(!out.contains("wf_v_Kept"), "{out}");
        let untouched = "varying vec2 v_A;\nvoid main() { vec2 v_A = vec2(1.0); gl_FragColor = vec4(v_A, 0.0, 1.0); }\n";
        assert_eq!(copy_written_inputs(untouched), untouched);
    }

    #[test]
    fn what_it_cannot_follow_it_leaves_alone() {
        let source = "mat3 m = mat3(1);\nint i = 2;\nbool b = i == 2;\nvoid main() { foo(1); }\n";
        assert_eq!(spell_out_conversions(source), source);
    }

    fn none(_: &str) -> Option<String> {
        None
    }

    #[test]
    fn puts_the_version_first_and_only_once() {
        let (out, missing) = preprocess(
            "#version 100\nvoid main() {}",
            Stage::Fragment,
            &BTreeMap::new(),
            none,
        );
        assert!(out.starts_with("#version 330 core\n"));
        assert_eq!(out.matches("#version").count(), 1);
        assert!(missing.is_empty());
    }

    #[test]
    fn pulls_in_includes_recursively() {
        let (out, missing) = preprocess(
            "#include \"a.h\"\nvoid main() {}",
            Stage::Fragment,
            &BTreeMap::new(),
            |name| match name {
                "a.h" => Some("#include \"b.h\"\nfloat a() { return 1.0; }".into()),
                "b.h" => Some("float b() { return 2.0; }".into()),
                _ => None,
            },
        );
        assert!(out.contains("float a()") && out.contains("float b()"));
        assert!(missing.is_empty());
    }

    #[test]
    fn reports_a_missing_include_with_its_line() {
        let (_, missing) = preprocess(
            "\n\n#include \"gone.h\"",
            Stage::Vertex,
            &BTreeMap::new(),
            none,
        );
        assert_eq!(
            missing,
            vec![Missing::Include {
                name: "gone.h".into(),
                line: 3
            }]
        );
    }

    #[test]
    fn stops_an_include_cycle() {
        let (_, missing) = preprocess(
            "#include \"loop.h\"",
            Stage::Vertex,
            &BTreeMap::new(),
            |_| Some("#include \"loop.h\"".into()),
        );
        assert!(matches!(
            missing.first(),
            Some(Missing::IncludeTooDeep { .. })
        ));
    }

    /// Materials sometimes spell combos lowercase; shaders only test uppercase.
    #[test]
    fn a_shader_supplies_defaults_for_its_own_options() {
        let source = r#"// [COMBO] {"combo":"BLENDMODE","type":"imageblending","default":9}
                        void main() { ApplyBlending(BLENDMODE); }"#;
        let (out, _) = preprocess(source, Stage::Fragment, &BTreeMap::new(), none);
        assert!(out.contains("#define BLENDMODE 9"));
    }

    #[test]
    fn the_material_overrides_the_shaders_default() {
        let source = r#"// [COMBO] {"combo":"BLENDMODE","default":9}"#;
        let combos = BTreeMap::from([("blendmode".into(), "2".into())]);
        let (out, _) = preprocess(source, Stage::Fragment, &combos, none);
        assert!(out.contains("#define BLENDMODE 2"));
        assert!(!out.contains("#define BLENDMODE 9"));
    }

    #[test]
    fn a_number_first_in_max_or_min_goes_last() {
        assert_eq!(number_last("c = vec4(max(0, a.rgb), a.a);"), "c = vec4(max(a.rgb, 0.0), a.a);");
        assert_eq!(number_last("x = min(1.5f, f(y, z));"), "x = min(f(y, z), 1.5);");
        // Left alone: already GLSL's way round, or not a number first.
        assert_eq!(number_last("x = max(a, 0.0);"), "x = max(a, 0.0);");
        assert_eq!(number_last("x = fmax(0, a);"), "x = fmax(0, a);");
        assert_eq!(number_last("x = max(b, c);"), "x = max(b, c);");
    }

    #[test]
    fn combos_are_defined_uppercase() {
        let combos = BTreeMap::from([("audioprocessing".into(), "3".into())]);
        let (out, _) = preprocess("", Stage::Fragment, &combos, none);
        assert!(out.contains("#define AUDIOPROCESSING 3"));
    }

    /// A shader that sets this itself must not be given a second definition.
    #[test]
    fn the_language_macro_yields_to_the_shader() {
        let (mine, _) = preprocess("void main() {}", Stage::Fragment, &BTreeMap::new(), none);
        assert!(mine.contains("#define GLSL 1"));
        let (theirs, _) = preprocess(
            "#define GLSL 1
void main() {}",
            Stage::Fragment,
            &BTreeMap::new(),
            none,
        );
        assert_eq!(theirs.matches("#define GLSL").count(), 1);
    }

    #[test]
    fn each_stage_gets_its_own_bridge() {
        let (vert, _) = preprocess("", Stage::Vertex, &BTreeMap::new(), none);
        let (frag, _) = preprocess("", Stage::Fragment, &BTreeMap::new(), none);
        assert!(vert.contains("#define varying out"));
        assert!(frag.contains("#define varying in"));
        assert!(frag.contains("#define gl_FragColor"));
        assert!(!vert.contains("gl_FragColor"));
    }

    #[test]
    fn drops_directives_only_the_engine_understands() {
        let (out, _) = preprocess(
            "#require LightingV1\nvoid main() {}",
            Stage::Fragment,
            &BTreeMap::new(),
            none,
        );
        assert!(!out.contains("#require"));
        assert!(out.contains("void main()"));
    }

    /// Three effects in the library ship a vertex shader half a component
    /// narrower than the fragment shader that reads it. Direct3D links that
    /// and GLSL does not, so the effect drew nothing until the declaration
    /// was widened to match.
    #[test]
    fn the_vertex_stage_is_widened_to_what_the_fragment_stage_reads() {
        let mut vert = "varying vec2 v_TexCoord;\nvoid main() { v_TexCoord.xy = a_TexCoord; }\n".to_owned();
        widen_varyings(&mut vert, "varying vec3 v_TexCoord;\n");
        assert!(vert.starts_with("varying vec3 v_TexCoord;"));
    }

    /// Widening a varying the vertex shader assigns whole would turn a link
    /// error into a compile error, which says less about what is wrong.
    #[test]
    fn a_varying_written_whole_is_left_as_it_is() {
        let mut vert = "varying vec2 v_TexCoord;\nvoid main() { v_TexCoord = a_TexCoord; }\n".to_owned();
        widen_varyings(&mut vert, "varying vec3 v_TexCoord;\n");
        assert!(vert.starts_with("varying vec2 v_TexCoord;"));
    }

    /// Every blur in the library samples `g_Texture0` from a function in
    /// `common_blur.h`, which the file includes before it declares the
    /// sampler. Left where it was written, that is an undeclared identifier
    /// and the effect draws nothing.
    #[test]
    fn a_uniform_is_declared_before_the_include_that_samples_it() {
        let source = "#include \"blur.h\"\nuniform sampler2D g_Texture0;\n";
        let (out, _) = preprocess(source, Stage::Fragment, &BTreeMap::new(), |_| {
            Some("vec4 blur() { return texSample2D(g_Texture0, vec2(0.0)); }\n".to_owned())
        });
        let declared = out.find("uniform sampler2D g_Texture0").unwrap();
        assert!(declared < out.find("texSample2D(g_Texture0").unwrap());
    }

    /// A uniform that only exists under an option has to be put back under it,
    /// or a shader compiled without that option declares something it never
    /// had and reads a texture nobody bound.
    #[test]
    fn a_hoisted_uniform_keeps_the_option_it_was_written_under() {
        let out = hoist_uniforms("#if COPYBG\nuniform sampler2D g_Texture2;\n#endif\nvoid main() {}\n");
        assert!(out.starts_with("#if (COPYBG)\nuniform sampler2D g_Texture2;\n#endif\n"));
    }

    #[test]
    fn recognises_include_lines_and_leaves_other_lines_alone() {
        assert_eq!(include_name("#include \"common.h\""), Some("common.h"));
        assert_eq!(include_name("  #include   \"a/b.h\"  "), Some("a/b.h"));
        assert_eq!(include_name("// #include is mentioned here"), None);
        assert_eq!(include_name("#define x 1"), None);
        assert_eq!(include_name("#include <angle.h>"), None);
    }
}

#[cfg(test)]
mod uniform_tests {
    use super::uniforms;

    #[test]
    fn reads_the_name_the_default_and_the_label() {
        let source = r#"
uniform float g_Overbright; // {"material":"ui_editor_properties_overbright","default":1.0,"range":[0,5]}
uniform vec3 g_Tint; // {"default":"1 0.5 0"}
uniform float g_Plain;
uniform float g_Undocumented; // not json
"#;
        let u = uniforms(source);
        assert_eq!(u.len(), 2);
        assert_eq!(u[0].name, "g_Overbright");
        assert_eq!(u[0].default, Some(serde_json::json!(1.0)));
        assert_eq!(
            u[0].label.as_deref(),
            Some("ui_editor_properties_overbright")
        );
        assert_eq!(u[1].name, "g_Tint");
        assert_eq!(u[1].label, None);
    }

    #[test]
    fn a_samplers_annotation_names_a_texture_and_is_flagged_as_one() {
        let source = r#"uniform sampler2D g_Texture1; // {"default":"util/clouds_256"}"#;
        let found = uniforms(source);
        let [found] = found.as_slice() else {
            panic!("expected one uniform, got {found:?}");
        };
        assert!(found.sampler);
        assert_eq!(found.default, Some(serde_json::json!("util/clouds_256")));
        assert_eq!(found.combo, None);
    }

    #[test]
    fn finds_what_the_particle_shaders_rely_on() {
        let Some(assets) = crate::Assets::locate() else {
            return;
        };
        let Some(source) = assets.get("shaders/genericparticle.frag") else {
            return;
        };
        let u = uniforms(&String::from_utf8_lossy(&source));
        let overbright = u
            .iter()
            .find(|u| u.name == "g_Overbright")
            .expect("no g_Overbright");
        // Without the default every particle in the library draws black, and
        // without the label the materials that set it to 5 are ignored.
        assert_eq!(overbright.default, Some(serde_json::json!(1.0)));
        assert_eq!(
            overbright.label.as_deref(),
            Some("ui_editor_properties_overbright")
        );
    }
}

#[cfg(test)]
mod sampler_switch_tests {
    use super::uniforms;

    #[test]
    fn reads_the_switch_a_sampler_hides_behind() {
        let source = r#"uniform sampler2D g_Texture2; // {"mode":"opacitymask","combo":"MASK","default":"util/white"}"#;
        let found = uniforms(source);
        let [mask] = found.as_slice() else {
            panic!("expected one uniform, got {found:?}");
        };
        assert_eq!(mask.combo.as_deref(), Some("MASK"));
        assert_eq!(mask.default, Some(serde_json::json!("util/white")));
    }
}
