//! The hop accounts themselves. Switching and status run the hop command,
//! so both tools behave the same; creating and removing edit hop's .env,
//! the source install.sh builds the profiles from.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const SSH_KEYS_PAGE: &str = "https://github.com/settings/ssh/new";

fn home() -> PathBuf {
    PathBuf::from(env::var_os("HOME").unwrap_or_default())
}

/// `hop ui` passes its own path and repo; run directly, hop is on the PATH.
fn bin() -> String {
    env::var("HOP_BIN").unwrap_or_else(|_| "hop".into())
}

fn root() -> PathBuf {
    env::var_os("HOP_ROOT").map_or_else(|| home().join(".hop"), PathBuf::from)
}

fn profiles() -> PathBuf {
    env::var_os("HOP_PROFILES").map_or_else(|| home().join(".config/hop"), PathBuf::from)
}

fn run(cmd: &mut Command) -> Result<String, String> {
    let out = cmd.stdin(Stdio::null()).output().map_err(|e| e.to_string())?;
    let (stdout, stderr) = (String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    if out.status.success() {
        Ok(stdout.into_owned())
    } else {
        Err(stderr.lines().chain(stdout.lines()).find(|l| !l.trim().is_empty()).unwrap_or("failed").trim().to_owned())
    }
}

/// `hop <account>`: git identity, SSH key and gh. Returns hop's verdict.
pub fn switch(account: &str) -> Result<String, String> {
    let out = run(Command::new(bin()).arg(account))?;
    Ok(out.lines().rev().find(|l| l.starts_with("OK") || l.starts_with("WARNING")).unwrap_or("switched").to_owned())
}

/// `hop s`, line by line.
pub fn status() -> Result<Vec<String>, String> {
    Ok(run(Command::new(bin()).arg("s"))?.lines().filter(|l| !l.trim().is_empty()).map(str::to_owned).collect())
}

pub struct NewAccount {
    pub name: String,
    pub email: String,
    pub key: String,
    pub login: String,
}

/// .env is sourced by bash: only plain characters may go in it.
fn plain(value: &str, extra: &str) -> bool {
    !value.is_empty() && value.chars().all(|c| c.is_ascii_alphanumeric() || extra.contains(c))
}

pub fn valid(a: &NewAccount) -> Result<(), &'static str> {
    let mut name = a.name.chars();
    if !name.next().is_some_and(|c| c.is_ascii_lowercase()) || !name.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()) {
        return Err("account names are lowercase letters and digits");
    }
    if profiles().join(format!("{}.gitconfig", a.name)).exists() {
        return Err("this account already exists");
    }
    if !plain(&a.email, "@._%+-") || !a.email.contains('@') {
        return Err("that email does not look right");
    }
    if !plain(&a.key, "/._~-") || !plain(&a.login, "-") {
        return Err("SSH key path or GitHub login has odd characters");
    }
    Ok(())
}

fn expand(path: &str) -> PathBuf {
    path.strip_prefix("~/").map_or_else(|| PathBuf::from(path), |rest| home().join(rest))
}

/// Rewrites hop's .env line by line.
fn edit_env(f: impl FnOnce(Vec<String>) -> Result<Vec<String>, String>) -> Result<(), String> {
    let path = root().join(".env");
    let text = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut lines = f(text.lines().map(str::to_owned).collect())?;
    lines.dedup_by(|a, b| a.is_empty() && b.is_empty());
    fs::write(&path, lines.join("\n") + "\n").map_err(|e| e.to_string())
}

fn value(line: &str) -> &str {
    line.split_once('=').map_or("", |(_, v)| v.trim_matches('"'))
}

fn set_accounts(lines: &mut [String], accounts: &[&str]) {
    if let Some(l) = lines.iter_mut().find(|l| l.starts_with("HOP_ACCOUNTS=")) {
        *l = format!("HOP_ACCOUNTS=\"{}\"", accounts.join(" "));
    }
}

/// Adds the account to .env, brings back its secrets list if it was
/// removed, makes the SSH key if missing, then reruns install.sh. Returns
/// the public key, to paste on GitHub.
pub fn create(a: &NewAccount) -> Result<String, String> {
    valid(a)?;
    let prefix = a.name.to_uppercase();
    edit_env(|mut lines| {
        let accounts = lines.iter().find(|l| l.starts_with("HOP_ACCOUNTS=")).map(|l| value(l).to_owned()).ok_or("no HOP_ACCOUNTS in .env")?;
        let mut names: Vec<&str> = accounts.split_whitespace().collect();
        names.push(&a.name);
        set_accounts(&mut lines, &names);
        lines.extend(["".into(), format!("{prefix}_EMAIL={}", a.email), format!("{prefix}_SSH_KEY=\"{}\"", a.key), format!("{prefix}_GITHUB={}", a.login)]);
        Ok(lines)
    })?;
    let removed = profiles().join("removed").join(format!("{}.gitconfig", a.name));
    if removed.exists() {
        fs::rename(&removed, profiles().join(format!("{}.gitconfig", a.name))).map_err(|e| e.to_string())?;
    }
    let key = expand(&a.key);
    if !key.exists() {
        run(Command::new("ssh-keygen").args(["-q", "-t", "ed25519", "-N", "", "-C", &a.email, "-f"]).arg(&key))?;
    }
    run(Command::new("bash").arg(root().join("install.sh")).current_dir(root()))?;
    fs::read_to_string(key.with_extension("pub")).map_err(|e| e.to_string())
}

/// Opens GitHub's page for a new SSH key in the default browser.
pub fn open_keys_page() {
    let _ = Command::new("open").arg(SSH_KEYS_PAGE).stdout(Stdio::null()).stderr(Stdio::null()).status();
}

/// Takes the account out of .env and parks its profile, with the names of
/// its secrets, in `removed/`. The Keychain items stay.
pub fn remove(name: &str, active: Option<&str>) -> Result<(), String> {
    if active == Some(name) {
        return Err("switch to another account first".into());
    }
    let prefix = format!("{}_", name.to_uppercase());
    edit_env(|mut lines| {
        if lines.iter().any(|l| l.starts_with("HOP_WORK_ACCOUNT=") && value(l) == name) {
            return Err("HOP_WORK_DIR uses this account: change it in .env first".into());
        }
        let accounts = lines.iter().find(|l| l.starts_with("HOP_ACCOUNTS=")).map(|l| value(l).to_owned()).unwrap_or_default();
        let left: Vec<&str> = accounts.split_whitespace().filter(|n| *n != name).collect();
        set_accounts(&mut lines, &left);
        for l in lines.iter_mut().filter(|l| l.starts_with("HOP_DEFAULT=") && value(l) == name) {
            *l = format!("HOP_DEFAULT={}", left.first().unwrap_or(&""));
        }
        for l in lines.iter_mut().filter(|l| l.starts_with("HOP_REPO_ACCOUNT=") && value(l) == name) {
            *l = "HOP_REPO_ACCOUNT=".into();
        }
        lines.retain(|l| !(l.starts_with(&prefix) && ["EMAIL=", "SSH_KEY=", "GITHUB="].iter().any(|k| l[prefix.len()..].starts_with(k))));
        Ok(lines)
    })?;
    let parked = profiles().join("removed");
    fs::create_dir_all(&parked).map_err(|e| e.to_string())?;
    let profile = profiles().join(format!("{name}.gitconfig"));
    fs::rename(&profile, parked.join(format!("{name}.gitconfig"))).map_err(|e| format!("{}: {e}", Path::new(&profile).display()))
}
