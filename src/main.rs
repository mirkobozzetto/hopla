#[cfg(not(target_os = "macos"))]
compile_error!("hopla is macOS only");

mod app;
mod auth;
mod hop;
mod palette;
mod pixel;
mod scene;
mod screen;
mod sprites;
mod state;
mod view;
mod surprise;
mod vault;

use std::time::Duration;

use ratatui::crossterm::event::{self, Event, KeyEventKind};

const FRAME: Duration = Duration::from_millis(33);

fn main() -> std::io::Result<()> {
    let mut app = app::App::new();
    // run() restores the terminal on return; its panic hook does it on a crash.
    ratatui::run(|terminal| {
        while !app.quit {
            app.tick();
            terminal.draw(|f| screen::draw(f, &mut app))?;
            if event::poll(FRAME)?
                && let Event::Key(key) = event::read()?
                && key.kind == KeyEventKind::Press
            {
                app.key(key);
            }
        }
        Ok(())
    })
}
