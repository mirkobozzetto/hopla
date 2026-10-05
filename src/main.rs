mod auth;
#[cfg(not(target_os = "macos"))]
#[path = "files.rs"]
mod secrets;
#[cfg(target_os = "macos")]
#[path = "keychain.rs"]
mod secrets;
mod hop;
mod palette;
mod pixel;
mod sprites;
mod state;
mod vault;
mod view;

use std::time::Duration;

use ratatui::crossterm::event::{self, Event, KeyEventKind};

const FRAME: Duration = Duration::from_millis(33);

fn main() -> std::io::Result<()> {
    let mut app = state::App::new();
    // run() restores the terminal on return; its panic hook does it on a crash.
    ratatui::run(|terminal| {
        while !app.quit {
            app.tick();
            terminal.draw(|f| view::draw(f, &mut app))?;
            if event::poll(FRAME)?
                && let Event::Key(key) = event::read()?
                && key.kind == KeyEventKind::Press
            {
                app.key(key);
            }
            // A command that needs the whole terminal: su, gh auth login.
            if let Some(mut job) = app.takeover.take() {
                ratatui::restore();
                let status = job.command.status();
                if let Some(reply) = job.reply {
                    let _ = reply.send(match status {
                        Ok(s) if s.success() => Ok(()),
                        Ok(_) => Err("wrong password".into()),
                        Err(e) => Err(e.to_string()),
                    });
                }
                *terminal = ratatui::init();
            }
        }
        Ok(())
    })
}
