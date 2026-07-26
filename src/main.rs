use std::io::{stdout, IsTerminal};

use crossterm::{
    event::{self, Event, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use git_tardis::app::AppState;
use git_tardis::cli::CliArgs;
use git_tardis::config::Config;
use git_tardis::git::GitRepo;
use git_tardis::ui::{render, KeyDispatcher, KeymapRegistry};
use ratatui::{backend::CrosstermBackend, Terminal};

fn load_repo_data(app: &mut AppState) {
    if let Ok(repo) = GitRepo::open(&app.repo_path) {
        if let Ok(files) = repo.list_files() {
            app.files = files;
        }
        if let Ok(statuses) = repo.get_status() {
            let items: Vec<String> = statuses
                .into_iter()
                .map(|s| format!("{} ({})", s.path, s.status_code()))
                .collect();
            app.uncommitted_files = items.clone();
            app.modified_files = items;
        }
        if let Ok(commits) = repo.get_commit_history(Some(50)) {
            app.commits = commits
                .into_iter()
                .map(|c| (c.hash[..7.min(c.hash.len())].to_string(), c.summary))
                .collect();
        }
        if let Some(first_file) = app.files.first() {
            let file_path = app.repo_path.join(first_file);
            if let Ok(content) = std::fs::read_to_string(&file_path) {
                app.code_lines = content.lines().map(|s| s.to_string()).collect();
                app.update_current_line_blame();
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = CliArgs::parse_args();
    let config = Config::load_or_default(args.config.as_deref());

    if let Some(target_hash) = args.mark_edit {
        println!(
            "Git-tardis sequence editor mode: marking commit '{}' for edit",
            target_hash
        );
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
        terminal.draw(|f| render(f, &app))?;

        if event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    if let Some(action) = dispatcher.handle_event(key, app.active_scope()) {
                        app.dispatch_action(action);
                    }
                }
            }
        } else if let Some(action) = dispatcher.check_timeout(app.active_scope()) {
            app.dispatch_action(action);
        }
    }

    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;

    Ok(())
}
