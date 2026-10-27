//! Reads every scene in a wallpaper directory and reports what is in them.
//!
//! The counts are the check: they are compared against the same library
//! surveyed independently, so a parser that quietly drops or miscounts objects
//! shows up as a number that does not line up.

use std::collections::BTreeMap;
use std::path::PathBuf;
use wpe_format::scene::ObjectKind;
use wpe_format::{Assets, Effect, Material, Model, Package, Resolver, Scene};

fn main() {
    let Some(root) = std::env::args_os().nth(1).map(PathBuf::from) else {
        eprintln!("usage: scenes <wallpaper-dir>");
        std::process::exit(2);
    };

    let mut scenes = 0usize;
    let mut objects = 0usize;
    let mut kinds: BTreeMap<&str, usize> = BTreeMap::new();
    let mut bound = BTreeMap::<&str, usize>::new();
    let mut scripted_wallpapers = Vec::new();
    let mut failures = Vec::new();
    // Following the chain is the real check: a name that resolves to nothing
    // would be a hole the renderer falls into later, not here.
    let mut chains = 0usize;
    let mut unresolved: Vec<String> = Vec::new();
    let mut shaders_builtin: BTreeMap<String, usize> = BTreeMap::new();
    let mut shaders_bundled = 0usize;
    let mut audio_passes = 0usize;
    let mut effects_followed = 0usize;

    let assets = Assets::locate();
    match &assets {
        Some(a) => println!("Wallpaper Engine assets: {}\n", a.root().display()),
        None => println!("no Wallpaper Engine installation found; built-ins will not resolve\n"),
    }

    let mut stack = vec![root];
    let mut packages = Vec::new();
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "pkg") {
                packages.push(path);
            }
        }
    }
    packages.sort();

    for path in packages {
        let Ok(package) = Package::read(&path) else {
            continue;
        };
        let Some(bytes) = package.get("scene.json") else {
            continue;
        };
        let name = path
            .parent()
            .and_then(|p| p.file_name())
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();

        let resolve = Resolver::new(&package, assets.as_ref());
        let scene = match Scene::parse(bytes) {
            Ok(s) => s,
            Err(e) => {
                failures.push((name, e.to_string()));
                continue;
            }
        };
        scenes += 1;
        let scripts = wpe_format::scene::count_scripts(bytes).unwrap_or(0);

        for object in &scene.objects {
            objects += 1;
            let key = match object.kind() {
                ObjectKind::Image(_) => "image",
                ObjectKind::Particle(_) => "particle",
                ObjectKind::Sound(_) => "sound",
                ObjectKind::Text => "text",
                ObjectKind::Empty => "empty",
            };
            *kinds.entry(key).or_default() += 1;
            if object.visible.is_bound() {
                *bound.entry("visible").or_default() += 1;
            }
            if object.color.is_bound() {
                *bound.entry("color").or_default() += 1;
            }
            if object.scale.is_bound() {
                *bound.entry("scale").or_default() += 1;
            }
            // Rarer than it looks: the same depth is written several ways.
            if object.parallax_depth.0 != [1.0, 1.0] {
                *bound.entry("parallax depth set").or_default() += 1;
            }

            // Effects hang off any object, and are where audio reactivity lives.
            for effect_ref in &object.effects {
                let Some(bytes) = resolve.get(&effect_ref.file) else {
                    unresolved.push(format!("{name}: effect {}", effect_ref.file));
                    continue;
                };
                let effect = match Effect::parse(&bytes) {
                    Ok(e) => e,
                    Err(e) => {
                        unresolved.push(format!("{name}: effect {}: {e}", effect_ref.file));
                        continue;
                    }
                };
                effects_followed += 1;
                // The scene's own overrides decide what an effect actually does.
                for over in &effect_ref.passes {
                    if over.uses_audio() {
                        audio_passes += 1;
                    }
                }
                for pass in &effect.passes {
                    if pass.material.is_empty() {
                        continue; // a command, not a draw
                    }
                    let Some(bytes) = resolve.get(&pass.material) else {
                        unresolved.push(format!("{name}: effect material {}", pass.material));
                        continue;
                    };
                    let Ok(material) = Material::parse(&bytes) else {
                        unresolved.push(format!("{name}: effect material {}", pass.material));
                        continue;
                    };
                    for p in &material.passes {
                        if p.uses_audio() {
                            audio_passes += 1;
                        }
                    }
                }
            }

            let ObjectKind::Image(model_path) = object.kind() else {
                continue;
            };
            let Some(model_bytes) = resolve.get(&model_path) else {
                unresolved.push(format!("{name}: model {model_path}"));
                continue;
            };
            let model = match Model::parse(&model_bytes) {
                Ok(m) => m,
                Err(e) => {
                    unresolved.push(format!("{name}: model {model_path}: {e}"));
                    continue;
                }
            };
            let Some(material_bytes) = resolve.get(&model.material) else {
                unresolved.push(format!("{name}: material {}", model.material));
                continue;
            };
            let material = match Material::parse(&material_bytes) {
                Ok(m) => m,
                Err(e) => {
                    unresolved.push(format!("{name}: material {}: {e}", model.material));
                    continue;
                }
            };
            chains += 1;
            for pass in &material.passes {
                if pass.uses_audio() {
                    audio_passes += 1;
                }
                if pass.shader.is_empty() {
                } else if package
                    .get(&format!("shaders/{}.frag", pass.shader))
                    .is_some()
                {
                    shaders_bundled += 1;
                } else if resolve.shader(&pass.shader, "frag").is_some() {
                    *shaders_builtin.entry(pass.shader.clone()).or_default() += 1;
                } else {
                    unresolved.push(format!("{name}: shader {}", pass.shader));
                }
                for texture in pass.textures.iter().flatten() {
                    if texture.is_empty() {
                        continue;
                    }
                    if wpe_format::effect::is_render_target(texture) {
                        continue;
                    }
                    if !resolve.has(&format!("materials/{texture}.tex")) {
                        unresolved.push(format!("{name}: texture {texture}"));
                    }
                }
            }
        }
        if scene.general.camerashake.is_bound() || scene.general.cameraparallax.is_bound() {
            *bound.entry("camera bound to a setting").or_default() += 1;
        }
        if scripts > 0 {
            scripted_wallpapers.push((name, scripts));
        }
    }

    println!("scenes parsed   {scenes}");
    println!("objects         {objects}");
    println!("object kinds    {kinds:?}");
    println!("bound/notable   {bound:?}");
    println!(
        "wallpapers with scripts  {} {:?}",
        scripted_wallpapers.len(),
        scripted_wallpapers
    );
    println!("\nimage chains followed  {chains}");
    println!("shaders in wallpaper   {shaders_bundled}");
    println!("shaders from WPE       {shaders_builtin:?}");
    println!("effects followed       {effects_followed}");
    println!("passes wanting audio   {audio_passes}");
    if unresolved.is_empty() {
        println!("every model, material, shader and texture reference resolves");
    } else {
        println!("{} unresolved references:", unresolved.len());
        for u in unresolved.iter().take(10) {
            println!("  {u}");
        }
    }
    if failures.is_empty() {
        println!("\nno failures");
    } else {
        println!("\n{} failures:", failures.len());
        for (what, why) in failures.iter().take(10) {
            println!("  {what}: {why}");
        }
        std::process::exit(1);
    }
}
