//! Loading a wallpaper far enough to put something on screen.
//!
//! Follows the chain an image object describes -- model, material, pass -- and
//! uploads the texture the first pass samples. The shaders a pass names are not
//! run yet; this is the step that proves the right bytes reach the GPU.

use crate::texture::Texture;
use wpe_format::scene::ObjectKind;
use wpe_format::{Assets, Material, Model, Package, Resolver, Scene};

pub struct Wallpaper {
    pub name: String,
    pub texture: Texture,
    /// The scene's reference size, which the image was authored against.
    pub scene_size: (u32, u32),
}

impl Wallpaper {
    /// Loads the first image object of the wallpaper in `dir`.
    pub fn load(gl: &glow::Context, dir: &std::path::Path) -> Result<Self, String> {
        let package = find_package(dir)?;
        let package = Package::read(&package).map_err(|e| format!("reading package: {e}"))?;
        let assets = Assets::locate();
        let resolve = Resolver::new(&package, assets.as_ref());

        let scene_bytes = package
            .get("scene.json")
            .ok_or("no scene.json in this wallpaper")?;
        let scene = Scene::parse(scene_bytes).map_err(|e| format!("scene.json: {e}"))?;

        for object in &scene.objects {
            let ObjectKind::Image(model_path) = object.kind() else {
                continue;
            };
            if !object.visible.value {
                continue;
            }
            let Some(model_bytes) = resolve.get(&model_path) else {
                continue;
            };
            let model = Model::parse(&model_bytes).map_err(|e| format!("{model_path}: {e}"))?;
            let Some(material_bytes) = resolve.get(&model.material) else {
                continue;
            };
            let material =
                Material::parse(&material_bytes).map_err(|e| format!("{}: {e}", model.material))?;

            let Some(pass) = material.passes.first() else {
                continue;
            };
            let Some(Some(name)) = pass.textures.first() else {
                continue;
            };
            if wpe_format::effect::is_render_target(name) {
                continue;
            }
            let Some(tex_bytes) = resolve.get(&format!("materials/{name}.tex")) else {
                continue;
            };
            let source = wpe_format::Texture::parse(&tex_bytes)
                .map_err(|e| format!("materials/{name}.tex: {e}"))?;
            if source.is_video() {
                continue; // video wallpapers are their own road
            }
            let texture = Texture::upload(gl, &source)?;

            return Ok(Self {
                name: object.name.clone(),
                texture,
                scene_size: (
                    scene.general.orthogonalprojection.width,
                    scene.general.orthogonalprojection.height,
                ),
            });
        }
        Err("no image object with a texture in this wallpaper".into())
    }
}

fn find_package(dir: &std::path::Path) -> Result<std::path::PathBuf, String> {
    let mut found: Vec<_> = std::fs::read_dir(dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "pkg"))
        .collect();
    found.sort();
    found
        .into_iter()
        .next()
        .ok_or_else(|| format!("no .pkg in {}", dir.display()))
}
