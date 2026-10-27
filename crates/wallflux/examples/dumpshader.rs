//! Prints a preprocessed shader with line numbers, for reading an error log
//! against the source the driver actually saw.
//!
//! ```text
//! cargo run -p wallflux --example dumpshader -- <wallpaper-dir/id> <shader> <vert|frag> [line]
//! ```
use std::collections::BTreeMap;
use std::path::PathBuf;
use wpe_format::shader::{Stage, preprocess};
use wpe_format::{Assets, Material, Package, Resolver};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [dir, want, stage_name, rest @ ..] = args.as_slice() else {
        eprintln!("usage: dumpshader <wallpaper-dir> <shader> <vert|frag> [line]");
        std::process::exit(2);
    };
    let around: Option<usize> = rest.first().and_then(|s| s.parse().ok());
    let stage = if stage_name == "vert" {
        Stage::Vertex
    } else {
        Stage::Fragment
    };
    let assets = Assets::locate();

    let mut pkgs = Vec::new();
    let mut stack = vec![PathBuf::from(dir)];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p)
            } else if p.extension().is_some_and(|x| x == "pkg") {
                pkgs.push(p)
            }
        }
    }
    pkgs.sort();

    for path in pkgs {
        let Ok(package) = Package::read(&path) else {
            continue;
        };
        let resolver = Resolver::new(&package, assets.as_ref());
        for (name, bytes) in package.iter() {
            if !(name.starts_with("materials/") && name.ends_with(".json")) {
                continue;
            }
            let Ok(material) = Material::parse(bytes) else {
                continue;
            };
            for pass in &material.passes {
                if pass.shader != *want {
                    continue;
                }
                let combos: BTreeMap<String, String> = pass
                    .defines()
                    .map(|(k, v)| (k, v.to_string().trim_matches('"').to_owned()))
                    .collect();
                let Some(src) = resolver.shader(&pass.shader, stage_name) else {
                    continue;
                };
                let (full, _) = preprocess(&String::from_utf8_lossy(&src), stage, &combos, |i| {
                    resolver
                        .get(&format!("shaders/{i}"))
                        .map(|b| String::from_utf8_lossy(&b).into_owned())
                });
                println!("=== {} combos={:?} ===", pass.shader, combos);
                for (i, line) in full.lines().enumerate() {
                    let n = i + 1;
                    if around.is_none_or(|a| n.abs_diff(a) <= 4) {
                        println!("{n:>4} | {line}");
                    }
                }
                return;
            }
        }
    }
    eprintln!("shader {want} not found");
}
