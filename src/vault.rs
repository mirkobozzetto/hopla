//! The hop accounts and their secrets, read the way `bin/hop` reads them.
//!
//! The Keychain is only reached through `/usr/bin/security`: hop stores every
//! item with that tool, so it is on each item's access list. Linking the
//! Keychain API directly would make macOS ask for the password after every
//! rebuild, because an ad hoc signed binary changes identity each time.

use std::env;
use std::io::Read;
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

fn service(account: &str, name: &str) -> String {
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
                .map(|var| Ok(Secret { set: security(&service(&name, &var), false)?.is_some(), name: var }))
                .collect::<Result<_, String>>()?;
            Ok(Account { name, secrets })
        })
        .collect()
}

/// The value of one secret, `None` when it is missing.
pub fn read(account: &str, name: &str) -> Result<Option<String>, String> {
    security(&service(account, name), true)
}

/// Runs `security find-generic-password` for one item. Without `value` it
/// only checks the item exists, which never touches the secret data.
fn security(service: &str, value: bool) -> Result<Option<String>, String> {
    let user = env::var("USER").unwrap_or_default();
    let mut cmd = Command::new(SECURITY);
    cmd.args(["find-generic-password", "-a", &user, "-s", service]);
    if value {
        cmd.arg("-w");
    }
    let mut child = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("{SECURITY}: {e}"))?;
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
    match status.code() {
        Some(0) => {
            if value {
                child.stdout.take().map(|mut s| s.read_to_string(&mut out));
                out.truncate(out.trim_end_matches('\n').len());
            }
            Ok(Some(out))
        }
        Some(NOT_FOUND) => Ok(None),
        _ => {
            child.stderr.take().map(|mut s| s.read_to_string(&mut out));
            let reason = out.rsplit(": ").next().unwrap_or("").trim();
            Err(format!("Keychain: {}", if reason.is_empty() { "access refused" } else { reason }))
        }
    }
}
