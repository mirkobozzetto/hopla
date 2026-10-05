#[cfg(not(target_os = "macos"))]
compile_error!("hopla is macOS only");

mod auth;
mod vault;

fn main() {
    if let Err(e) = auth::unlock().recv().unwrap_or_else(|_| Err("no answer".into())) {
        eprintln!("locked: {e}");
        std::process::exit(1);
    }
    match vault::load() {
        Ok(accounts) => {
            let active = vault::active();
            for a in &accounts {
                let mark = if active.as_deref() == Some(&a.name) { " (active)" } else { "" };
                for s in &a.secrets {
                    let state = match vault::read(&a.name, &s.name) {
                        Ok(Some(v)) => format!("set, {} chars read", v.chars().count()),
                        Ok(None) => "missing".into(),
                        Err(e) => e,
                    };
                    println!("{}{mark}: {} {state}", a.name, s.name);
                }
            }
        }
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}
