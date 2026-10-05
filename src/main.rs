#[cfg(not(target_os = "macos"))]
compile_error!("hopla is macOS only");

mod auth;
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
            // A command that needs the whole terminal, like gh auth login.
            if let Some(mut command) = app.suspend.take() {
                ratatui::restore();
                let _ = command.status();
                *terminal = ratatui::init();
            }
        }
        Ok(())
    })
}
