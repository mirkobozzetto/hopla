//! Touch ID, or the session password when Touch ID is not available.
//!
//! A guard for the screen, not a lock on the data: any process of the user
//! can read hop items through `security` without a prompt.

use std::sync::mpsc::{self, Receiver};

use robius_authentication::{AndroidText, BiometricStrength, Context, PolicyBuilder, Text, WindowsText};

/// Asks once; the outcome arrives on the returned channel.
pub fn unlock() -> Receiver<Result<(), String>> {
    let (tx, rx) = mpsc::channel();
    // Biometrics plus password is LAPolicy::DeviceOwnerAuthentication.
    let Some(policy) = PolicyBuilder::new().biometrics(Some(BiometricStrength::Strong)).password(true).build() else {
        let _ = tx.send(Err("no authentication policy on this Mac".into()));
        return rx;
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
    rx
}
