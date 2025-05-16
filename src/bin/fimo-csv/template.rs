// src/template.rs
use minijinja::Environment;
use anyhow::Result;
use std::fs;
use std::path::Path;

pub fn load_templates<P: AsRef<Path>>(dir: P) -> Result<Environment<'static>> {
    let mut env = Environment::new();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("j2") {
            let name = path.file_stem().unwrap().to_string_lossy().to_string().into_boxed_str();
            let content = fs::read_to_string(&path)?;
            let name_static: &'static str = Box::leak(name);
            let content_static: &'static str = Box::leak(content.into_boxed_str());
            env.add_template(name_static, content_static)?;
        }
    }

    Ok(env)
}