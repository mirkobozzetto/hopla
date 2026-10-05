//! The hop accounts and their secrets, read the way `bin/hop` reads them:
//! names from the profiles, values from the Keychain on macOS and from
//! `<account>.secrets` files elsewhere.

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::secrets;

pub struct Secret {
    pub name: String,
    /// `None` until the Keychain has been asked.
    pub set: Option<bool>,
}

pub struct Account {
    pub name: String,
    pub secrets: Vec<Secret>,
}

fn home() -> PathBuf {
    PathBuf::from(env::var_os("HOME").unwrap_or_default())
}

pub fn profiles_dir() -> PathBuf {
    env::var_os("HOP_PROFILES").map_or_else(|| home().join(".config/hop"), PathBuf::from)
}

fn git_config(file: &Path, args: &[&str]) -> Vec<String> {
    Command::new("git")
        .arg("config")
        .arg("-f")
        .arg(file)
        .args(args)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).lines().map(str::to_owned).collect())
        .unwrap_or_default()
}

/// The account `hop` currently points at, if any.
pub fn active() -> Option<String> {
    let file = env::var_os("HOP_ACTIVE").map_or_else(|| home().join(".gitconfig-active"), PathBuf::from);
    let path = git_config(&file, &["include.path"]).into_iter().next()?;
    Some(Path::new(&path).file_stem()?.to_string_lossy().into_owned())
}

fn profile(account: &str) -> PathBuf {
    profiles_dir().join(format!("{account}.gitconfig"))
}

/// Every account with the names of its secrets, from the profiles only:
/// fast, the Keychain is asked afterwards with `exists`.
pub fn load() -> Result<Vec<Account>, String> {
    let dir = profiles_dir();
    let entries = std::fs::read_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let mut files: Vec<PathBuf> = entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "gitconfig"))
        .collect();
    files.sort();
    files
        .iter()
        .map(|file| {
            let name = file.file_stem().unwrap_or_default().to_string_lossy().into_owned();
            let secrets = git_config(file, &["--get-all", "env.var"]).into_iter().map(|var| Secret { name: var, set: None }).collect();
            Ok(Account { name, secrets })
        })
        .collect()
}

/// The same rules as `hop secret set`.
pub fn valid_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|c| c.is_ascii_uppercase() || c == '_')
        && chars.all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}

/// Non empty, without a quote, backslash or space, like `hop secret set`:
/// the value travels inside a quoted `security -i` command.
pub fn valid_value(value: &str) -> bool {
    !value.is_empty() && !value.chars().any(|c| c == '"' || c == '\\' || c.is_whitespace())
}

/// Set or missing, like `hop secret list`, without reading the value.
pub fn exists(account: &str, name: &str) -> Result<bool, String> {
    secrets::find(account, name, false).map(|found| found.is_some())
}

/// The value of one secret, `None` when it is missing.
pub fn read(account: &str, name: &str) -> Result<Option<String>, String> {
    secrets::find(account, name, true)
}

/// Stores a value like `hop secret set NAME account`, and declares the name
/// in the account's profile when it is new.
pub fn store(account: &str, name: &str, value: &str) -> Result<(), String> {
    secrets::put(account, name, value)?;
    let file = profile(account);
    if !git_config(&file, &["--get-all", "env.var"]).iter().any(|n| n == name) {
        git_config(&file, &["--add", "env.var", name]);
    }
    Ok(())
}

/// Deletes the value and the name from the account's profile.
pub fn remove(account: &str, name: &str) -> Result<(), String> {
    secrets::delete(account, name)?;
    // Names are [A-Z0-9_], so the anchored pattern matches only this one.
    git_config(&profile(account), &["--unset", "env.var", &format!("^{name}$")]);
    Ok(())
}

