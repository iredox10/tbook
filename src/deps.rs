use anyhow::{Context, Result};
use std::env;
use std::path::PathBuf;

fn env_override(name: &str) -> Option<PathBuf> {
    let var = match name {
        "pdftotext" => "TBOOK_PDFTOTEXT",
        "pdftoppm" => "TBOOK_PDFTOPPM",
        _ => return None,
    };

    env::var_os(var).map(PathBuf::from).filter(|p| p.exists())
}

fn sibling_binary(name: &str) -> Option<PathBuf> {
    let exe = env::current_exe().ok()?;
    let dir = exe.parent()?;
    let direct = dir.join(name);
    if direct.exists() {
        return Some(direct);
    }
    let bin = dir.join("bin").join(name);
    if bin.exists() {
        return Some(bin);
    }
    None
}

fn data_dir_binary(name: &str) -> Option<PathBuf> {
    let data = dirs::data_dir()?;
    let candidate = data.join("tbook").join("bin").join(name);
    if candidate.exists() {
        return Some(candidate);
    }
    None
}

fn path_binary(name: &str) -> Option<PathBuf> {
    let path = env::var_os("PATH")?;
    for dir in env::split_paths(&path) {
        let candidate = dir.join(name);
        if candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

pub fn resolve_poppler_binary(name: &str) -> Result<PathBuf> {
    if let Some(path) = env_override(name) {
        return Ok(path);
    }
    if let Some(path) = sibling_binary(name) {
        return Ok(path);
    }
    if let Some(path) = data_dir_binary(name) {
        return Ok(path);
    }
    if let Some(path) = path_binary(name) {
        return Ok(path);
    }

    Err(anyhow::anyhow!(
        "Missing {}. Install poppler-utils or use the bundled Linux release (includes poppler binaries).",
        name
    ))
}

pub fn resolve_poppler_command(name: &str) -> Result<std::process::Command> {
    let path = resolve_poppler_binary(name)
        .with_context(|| format!("Unable to locate bundled or system {}", name))?;
    Ok(std::process::Command::new(path))
}
