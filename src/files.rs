//! Linux and Windows: secrets in `<account>.secrets` next to the profiles,
//! one `NAME=value` per line, readable by you only, as hop does there.

use std::fs;
use std::io::Write;
use std::path::PathBuf;

use crate::vault::profiles_dir;

fn path(account: &str) -> PathBuf {
    profiles_dir().join(format!("{account}.secrets"))
}

fn lines(account: &str) -> Vec<String> {
    fs::read_to_string(path(account)).unwrap_or_default().lines().map(str::to_owned).collect()
}

/// The value, `None` when missing or empty. `_value` only matters to the
/// Keychain, where checking does not read the secret.
pub fn find(account: &str, name: &str, _value: bool) -> Result<Option<String>, String> {
    let key = format!("{name}=");
    Ok(lines(account).iter().find_map(|l| l.strip_prefix(&key)).filter(|v| !v.is_empty()).map(str::to_owned))
}

fn write(account: &str, lines: &[String]) -> Result<(), String> {
    let path = path(account);
    let tmp = path.with_extension("secrets.tmp");
    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    let mut file = options.open(&tmp).map_err(|e| format!("{}: {e}", tmp.display()))?;
    file.write_all((lines.join("\n") + "\n").as_bytes()).map_err(|e| e.to_string())?;
    fs::rename(&tmp, &path).map_err(|e| e.to_string())
}

pub fn put(account: &str, name: &str, value: &str) -> Result<(), String> {
    let key = format!("{name}=");
    let mut all: Vec<String> = lines(account).into_iter().filter(|l| !l.starts_with(&key)).collect();
    all.push(format!("{name}={value}"));
    write(account, &all)
}

pub fn delete(account: &str, name: &str) -> Result<(), String> {
    let key = format!("{name}=");
    let all: Vec<String> = lines(account).into_iter().filter(|l| !l.starts_with(&key)).collect();
    write(account, &all)
}
