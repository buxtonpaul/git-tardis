use std::io::{stdout, IsTerminal, Write};

use crossterm::{
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use git_tardis::app::AppState;
use git_tardis::cli::CliArgs;
use git_tardis::config::Config;
use git_tardis::ui::{run_event_loop, CrosstermEvents, KeyDispatcher, KeymapRegistry};
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

    let stdin = std::io::stdin();
    if !git_tardis::rebase::check_and_handle_startup_rebase(&args.path, stdin.lock())? {
        return Ok(());
    }

    let interactive = stdout().is_terminal();

    let mut app = AppState::new(args.path.clone());

    // Apply theme color overrides if supplied via CLI or env
    let bg = args
        .theme_bg
        .as_deref()
        .and_then(git_tardis::ui::parse_color);
    let fg = args
        .theme_fg
        .as_deref()
        .and_then(git_tardis::ui::parse_color);
    if bg.is_some() || fg.is_some() {
        app.set_theme_colors(bg, fg);
    }

    // Put something on screen before talking to git, and let blame and candidate lookups
    // run off the UI thread from the start.
    let mut terminal = if interactive {
        enable_raw_mode()?;
        stdout().execute(EnterAlternateScreen)?;
        app.in_alternate_screen = true;
        app.enable_background_git();

        let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
        let ready_message =
            std::mem::replace(&mut app.status_message, "Loading repository...".to_string());
        terminal.draw(|f| git_tardis::ui::render(f, &mut app))?;
        app.status_message = ready_message;
        Some(terminal)
    } else {
        None
    };

    load_repo_data(&mut app);

    // If initial target file is specified, open file at specified line
    if let Some(target_file) = &args.file {
        app.open_file_at_line(target_file, args.line);
    } else if let Some(line) = args.line {
        app.goto_line(line);
    }

    if let Some(jump_arg) = args.effective_jump_mode() {
        let nav_mode: git_tardis::app::NavigationMode = jump_arg.into();
        app.nav_mode = nav_mode;
        match jump_arg {
            git_tardis::cli::NavigationModeArg::Commit => {
                app.dispatch_action(git_tardis::ui::Action::JumpPrevCommit);
            }
            git_tardis::cli::NavigationModeArg::File => {
                app.dispatch_action(git_tardis::ui::Action::JumpPrevFile);
            }
            git_tardis::cli::NavigationModeArg::Function => {
                app.dispatch_action(git_tardis::ui::Action::JumpPrevFunction);
            }
            git_tardis::cli::NavigationModeArg::Line => {
                app.dispatch_action(git_tardis::ui::Action::JumpPrevLine);
            }
        }
    }

    let mut registry = KeymapRegistry::new();
    if let Some(keymaps) = &config.keymaps {
        registry.apply_config(keymaps);
    }
    let mut dispatcher = KeyDispatcher::new(registry);

    let mut terminal = match terminal.take() {
        Some(terminal) => terminal,
        None => {
            println!("Initialized Git-tardis for repository: {:?}", app.repo_path);
            println!("State status: {}", app.status_message);
            println!("Non-interactive environment detected. Application loop ready.");
            return Ok(());
        }
    };

    run_event_loop(
        &mut terminal,
        &mut app,
        &mut dispatcher,
        &mut CrosstermEvents,
    )?;

    disable_raw_mode()?;
    if app.in_alternate_screen {
        stdout().execute(LeaveAlternateScreen)?;
        app.in_alternate_screen = false;
    }

    if let Some(msg) = &app.exit_message {
        println!("{}", msg);
        let _ = stdout().flush();
    }

    Ok(())
}
