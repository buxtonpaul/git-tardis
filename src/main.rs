use std::io::{stdout, IsTerminal};

use crossterm::{
    event::{self, Event, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use git_tardis::app::AppState;
use git_tardis::cli::CliArgs;
use git_tardis::config::Config;
use git_tardis::ui::{render, KeyDispatcher, KeymapRegistry};
use ratatui::{backend::CrosstermBackend, Terminal};

fn load_repo_data(app: &mut AppState) {
    app.reload_repo_data();
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = CliArgs::parse_args();
    let config = Config::load_or_default(args.config.as_deref());

    if let Some(target_hash) = args.mark_edit {
        let todo_path = args.path;
        if let Err(e) =
            git_tardis::rebase::handle_sequence_editor_mark_edit(&target_hash, &todo_path)
        {
            eprintln!("Sequence editor error: {}", e);
            std::process::exit(1);
        }
        return Ok(());
    }

    let mut app = AppState::new(args.path);
    load_repo_data(&mut app);

    let mut registry = KeymapRegistry::new();
    if let Some(keymaps) = &config.keymaps {
        registry.apply_config(keymaps);
    }
    let mut dispatcher = KeyDispatcher::new(registry);

    if !stdout().is_terminal() {
        println!("Initialized Git-tardis for repository: {:?}", app.repo_path);
        println!("State status: {}", app.status_message);
        println!("Non-interactive environment detected. Application loop ready.");
        return Ok(());
    }

    // Interactive TUI Execution
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    while app.running {
        terminal.draw(|f| render(f, &mut app))?;

        if event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    if app.input_prompt.is_some() {
                        app.handle_input_key(key);
                    } else if let Some(action) = dispatcher.handle_event(key, app.active_scope()) {
                        app.dispatch_action(action);
                    }
                }
            }
        } else if app.input_prompt.is_none() {
            if let Some(action) = dispatcher.check_timeout(app.active_scope()) {
                app.dispatch_action(action);
            }
        }
    }

    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;

    Ok(())
}
