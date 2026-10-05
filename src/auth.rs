//! Unlocking: Touch ID or the password on macOS, Windows Hello on Windows,
//! the session password through `su` elsewhere.
//!
//! A guard for the screen, not a lock on the data: any process of the user
//! can read hop's secrets.

use std::process::Command;
use std::sync::mpsc::{self, Receiver, Sender};

pub type Outcome = Result<(), String>;

#[cfg(target_os = "macos")]
pub const NAME: &str = "Touch ID";
#[cfg(windows)]
pub const NAME: &str = "Windows Hello";
#[cfg(not(any(target_os = "macos", windows)))]
pub const NAME: &str = "your password";

/// Asks once; the outcome arrives on the receiver. When a command comes
/// back, it needs the whole terminal: run it, then send its outcome.
pub fn unlock() -> (Receiver<Outcome>, Option<(Command, Sender<Outcome>)>) {
    let (tx, rx) = mpsc::channel();
    let prompt = ask(tx);
    (rx, prompt)
}

#[cfg(any(target_os = "macos", windows))]
fn ask(tx: Sender<Outcome>) -> Option<(Command, Sender<Outcome>)> {
    use robius_authentication::{AndroidText, BiometricStrength, Context, PolicyBuilder, Text, WindowsText};

    // Biometrics plus password is LAPolicy::DeviceOwnerAuthentication on
    // macOS, and Hello with its PIN fallback on Windows.
    let Some(policy) = PolicyBuilder::new().biometrics(Some(BiometricStrength::Strong)).password(true).build() else {
        let _ = tx.send(Err("no authentication policy on this machine".into()));
        return None;
    };
    let text = Text {
        android: AndroidText { title: "hopla", subtitle: None, description: None },
        apple: "unlock your hop secrets",
        windows: WindowsText::new_truncated("hopla", "unlock your hop secrets"),
    };
    let reply = tx.clone();
    if let Err(e) = Context::new(()).authenticate(text, &policy, move |r| {
        let _ = reply.send(r.map_err(|e| e.to_string()));
    }) {
        let _ = tx.send(Err(e.to_string()));
    }
    None
}

#[cfg(not(any(target_os = "macos", windows)))]
fn ask(tx: Sender<Outcome>) -> Option<(Command, Sender<Outcome>)> {
    // su to yourself checks your own password through PAM, no admin right
    // or polkit rule needed.
    let mut su = Command::new("su");
    su.args([&std::env::var("USER").unwrap_or_default(), "-c", "true"]);
    Some((su, tx))
}
