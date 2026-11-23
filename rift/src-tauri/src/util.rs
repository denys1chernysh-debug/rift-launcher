use std::path::PathBuf;
use std::process::Command;

pub fn e<T: ToString>(x: T) -> String {
    x.to_string()
}

pub fn now() -> i64 {
    chrono::Utc::now().timestamp()
}

/// Command that never flashes a console window on Windows.
pub fn cmd(program: &str) -> Command {
    let mut c = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x0800_0000);
    }
    c
}

pub fn run_out(program: &str, args: &[&str]) -> String {
    cmd(program)
        .args(args)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default()
}

pub fn env_path(key: &str) -> Option<PathBuf> {
    std::env::var_os(key).map(PathBuf::from)
}

pub fn slug(s: &str) -> String {
    let mut out = String::new();
    let mut dash = false;
    for c in s.to_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
            dash = false;
        } else if !dash && !out.is_empty() {
            out.push('-');
            dash = true;
        }
    }
    out.trim_end_matches('-').to_string()
}

/// Open a URI (steam://, com.epicgames.launcher://, shell:AppsFolder\...) with the OS handler.
pub fn open_uri(uri: &str) -> Result<(), String> {
    if uri.starts_with("shell:") {
        cmd("explorer.exe").arg(uri).spawn().map_err(e)?;
    } else {
        cmd("rundll32.exe")
            .arg("url.dll,FileProtocolHandler")
            .arg(uri)
            .spawn()
            .map_err(e)?;
    }
    Ok(())
}

/// Split a launch-arguments string, honouring double quotes (so paths with spaces survive).
pub fn split_args(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    for c in s.chars() {
        match c {
            '"' => quoted = !quoted,
            ' ' | '\t' if !quoted => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            _ => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

pub fn dir_size(p: &std::path::Path) -> u64 {
    walkdir::WalkDir::new(p)
        .into_iter()
        .filter_map(|x| x.ok())
        .filter_map(|x| x.metadata().ok())
        .filter(|m| m.is_file())
        .map(|m| m.len())
        .sum()
}
