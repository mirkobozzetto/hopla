//! macOS: secrets in the Keychain, reached only through `/usr/bin/security`.
//! hop stores every item with that tool, so it is on each item's access
//! list. Linking the Keychain API directly would make macOS ask for the
//! password after every rebuild, an ad hoc signed binary changing identity.

use std::env;
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const SECURITY: &str = "/usr/bin/security";
const TIMEOUT: Duration = Duration::from_secs(3);
// `security` exits with the low byte of the OSStatus: errSecItemNotFound.
const NOT_FOUND: i32 = 44;

fn service(account: &str, name: &str) -> String {
    let prefix = env::var("HOP_KEYCHAIN_PREFIX").unwrap_or_else(|_| "hop".into());
    format!("{prefix}:{account}:{name}")
}

/// The value, or only whether it exists when `value` is false.
pub fn find(account: &str, name: &str, value: bool) -> Result<Option<String>, String> {
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

/// Stores the value; fed through stdin so it never shows in the process list.
pub fn put(account: &str, name: &str, value: &str) -> Result<(), String> {
    let line = format!("add-generic-password -U -a \"{}\" -s \"{}\" -w \"{value}\"\n", user(), service(account, name));
    security(&["-i"], Some(&line))?;
    match find(account, name, false)? {
        Some(_) => Ok(()),
        None => Err("the Keychain did not keep the value".into()),
    }
}

pub fn delete(account: &str, name: &str) -> Result<(), String> {
    let (user, service) = (user(), service(account, name));
    match security(&["delete-generic-password", "-a", &user, "-s", &service], None)? {
        (Some(0 | NOT_FOUND), _) => Ok(()),
        (_, err) => Err(refused(&err)),
    }
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
