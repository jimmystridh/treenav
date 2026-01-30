mod app;
mod config;
mod icons;
mod size;
mod state;
mod tree;
mod ui;

use app::App;
use clap::Parser;
use color_eyre::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "treenav")]
#[command(about = "A terminal-based directory tree navigator with persistent state")]
struct Args {
    #[arg(default_value = ".")]
    path: PathBuf,
}

fn main() -> Result<()> {
    color_eyre::install()?;

    let args = Args::parse();
    let path = args.path.canonicalize()?;

    // Open /dev/tty directly for terminal I/O (allows stdout to be captured)
    let tty = File::options().read(true).write(true).open("/dev/tty")?;
    let mut tty_writer = BufWriter::new(tty);

    enable_raw_mode()?;
    execute!(tty_writer, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(tty_writer);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(path)?;
    let result = app.run(&mut terminal);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;

    // Print selected directory to stdout (can be captured by shell)
    if let Some(selected_dir) = app.selected_dir {
        println!("{}", selected_dir.display());
    }

    result
}
