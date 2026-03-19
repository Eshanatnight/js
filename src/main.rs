mod app;
mod picker;
mod tree;
mod ui;

use std::io::{self, IsTerminal, Read};
use std::path::PathBuf;
use std::{env, fs};

use anyhow::{Context, Result};
use crossterm::ExecutableCommand;
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() -> Result<()> {
    if let Some((json_str, filename)) = read_input()? {
        reattach_tty();
        let tree = tree::JsonTree::from_str(&json_str).context("Failed to parse JSON")?;
        let mut app = app::App::new(tree, filename);

        let mut terminal = ratatui::init();
        io::stdout().execute(EnableMouseCapture)?;
        let result = app.run(&mut terminal);
        io::stdout().execute(DisableMouseCapture).ok();
        ratatui::restore();
        result
    } else {
        let mut terminal = ratatui::init();
        io::stdout().execute(EnableMouseCapture)?;
        let result = run_with_picker(&mut terminal);
        io::stdout().execute(DisableMouseCapture).ok();
        ratatui::restore();
        result
    }
}

fn run_with_picker(terminal: &mut ratatui::DefaultTerminal) -> Result<()> {
    let mut fp = picker::FilePicker::new();
    let Some(path) = fp.run(terminal)? else {
        return Ok(());
    };

    let (json_str, filename) = load_json_file(&path)?;
    let tree = tree::JsonTree::from_str(&json_str).context("Failed to parse JSON")?;
    let mut app = app::App::new(tree, filename);
    app.run(terminal)
}

fn load_json_file(path: &PathBuf) -> Result<(String, String)> {
    let display = path.display().to_string();
    let content =
        fs::read_to_string(path).with_context(|| format!("Failed to read file: {display}"))?;
    Ok((content, display))
}

/// When stdin was a pipe (e.g. `cat file.json | json-tui`), crossterm can't
/// enable raw mode on it. Re-open `/dev/tty` on fd 0 so the TUI works.
fn reattach_tty() {
    #[cfg(unix)]
    if !io::stdin().is_terminal() {
        use std::os::unix::io::AsRawFd;
        if let Ok(tty) = fs::File::open("/dev/tty") {
            unsafe {
                libc::dup2(tty.as_raw_fd(), libc::STDIN_FILENO);
            }
        }
    }
}

fn read_input() -> Result<Option<(String, String)>> {
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 {
        let path = &args[1];
        let content =
            fs::read_to_string(path).with_context(|| format!("Failed to read file: {path}"))?;
        Ok(Some((content, path.clone())))
    } else if !io::stdin().is_terminal() {
        let mut buf = String::new();
        io::stdin()
            .read_to_string(&mut buf)
            .context("Failed to read from stdin")?;
        Ok(Some((buf, "stdin".to_string())))
    } else {
        Ok(None)
    }
}
