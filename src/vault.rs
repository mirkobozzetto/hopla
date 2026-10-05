//! The hop accounts and their secrets, read the way `bin/hop` reads them.
//!
//! The Keychain is only reached through `/usr/bin/security`: hop stores every
//! item with that tool, so it is on each item's access list. Linking the
//! Keychain API directly would make macOS ask for the password after every
//! rebuild, because an ad hoc signed binary changes identity each time.

use std::env;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const SECURITY: &str = "/usr/bin/security";
const TIMEOUT: Duration = Duration::from_secs(3);
// `security` exits with the low byte of the OSStatus: errSecItemNotFound.
const NOT_FOUND: i32 = 44;

pub struct Secret {
    pub name: String,
    pub set: bool,
}

pub struct Account {
    pub name: String,
    pub secrets: Vec<Secret>,
}

fn home() -> PathBuf {
    PathBuf::from(env::var_os("HOME").unwrap_or_default())
}

fn profiles_dir() -> PathBuf {
    env::var_os("HOP_PROFILES").map_or_else(|| home().join(".config/hop"), PathBuf::from)
}

pub fn service(account: &str, name: &str) -> String {
    let prefix = env::var("HOP_KEYCHAIN_PREFIX").unwrap_or_else(|_| "hop".into());
    format!("{prefix}:{account}:{name}")
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

/// Every account with its declared secrets, each marked set or missing like
/// `hop secret list <account>`. Values are not read here.
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
            let secrets = git_config(file, &["--get-all", "env.var"])
                .into_iter()
                .map(|var| Ok(Secret { set: exists(&name, &var)?, name: var }))
                .collect::<Result<_, String>>()?;
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

fn find(account: &str, name: &str, value: bool) -> Result<Option<String>, String> {
    let (user, service) = (user(), service(account, name));
    let mut args = vec!["find-generic-password", "-a", &user, "-s", &service];
    if value {
        args.push("-w");
    }
    match security(&args, None)? {
        (Some(0), out) => Ok(Some(out.trim_end_matches('\n').to_owned())),
        (Some(NOT_FOUND), _) => Ok(None),
        (_, err) => Err(refused(&err)),
    }
}

fn exists(account: &str, name: &str) -> Result<bool, String> {
    find(account, name, false).map(|found| found.is_some())
}

/// The value of one secret, `None` when it is missing.
pub fn read(account: &str, name: &str) -> Result<Option<String>, String> {
    find(account, name, true)
}

/// Stores a value like `hop secret set NAME account`, and declares the name
/// in the account's profile when it is new.
pub fn store(account: &str, name: &str, value: &str) -> Result<(), String> {
    // Fed through stdin so the value never shows up in the process list.
    let line = format!("add-generic-password -U -a \"{}\" -s \"{}\" -w \"{value}\"\n", user(), service(account, name));
    security(&["-i"], Some(&line))?;
    if !exists(account, name)? {
        return Err("the Keychain did not keep the value".into());
    }
    let file = profile(account);
    if !git_config(&file, &["--get-all", "env.var"]).iter().any(|n| n == name) {
        git_config(&file, &["--add", "env.var", name]);
    }
    Ok(())
}

/// Deletes the Keychain item and the name from the account's profile.
pub fn remove(account: &str, name: &str) -> Result<(), String> {
    let (user, service) = (user(), service(account, name));
    match security(&["delete-generic-password", "-a", &user, "-s", &service], None)? {
        (Some(0 | NOT_FOUND), _) => {}
        (_, err) => return Err(refused(&err)),
    }
    // Names are [A-Z0-9_], so the anchored pattern matches only this one.
    git_config(&profile(account), &["--unset", "env.var", &format!("^{name}$")]);
    Ok(())
}

fn user() -> String {
    env::var("USER").unwrap_or_default()
}

fn refused(err: &str) -> String {
    let reason = err.rsplit(": ").next().unwrap_or("").trim();
    format!("Keychain: {}", if reason.is_empty() { "access refused" } else { reason })
}

/// Runs `security` with a 3 s limit and returns its exit code with its
/// output, stdout on success and stderr otherwise.
fn security(args: &[&str], input: Option<&str>) -> Result<(Option<i32>, String), String> {
    let mut child = Command::new(SECURITY)
        .args(args)
        .stdin(if input.is_some() { Stdio::piped() } else { Stdio::null() })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("{SECURITY}: {e}"))?;
    if let (Some(text), Some(mut stdin)) = (input, child.stdin.take()) {
        stdin.write_all(text.as_bytes()).map_err(|e| e.to_string())?;
    }
    let deadline = Instant::now() + TIMEOUT;
    // ponytail: polls every 20 ms, fine for one item at a time.
    let status = loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            break status;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err("the Keychain did not answer in 3 s: is it locked?".into());
        }
        thread::sleep(Duration::from_millis(20));
    };
    let mut out = String::new();
    if status.success() {
        child.stdout.take().map(|mut s| s.read_to_string(&mut out));
    } else {
        child.stderr.take().map(|mut s| s.read_to_string(&mut out));
    }
    Ok((status.code(), out))
}
