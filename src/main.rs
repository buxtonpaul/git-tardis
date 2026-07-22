use git_tardis::app::AppState;
use git_tardis::cli::CliArgs;
use git_tardis::config::Config;

fn main() {
    let args = CliArgs::parse_args();
    let _config = Config::load_or_default(args.config.as_deref());

    if let Some(target_hash) = args.mark_edit {
        println!(
            "Git-tardis sequence editor mode: marking commit '{}' for edit",
            target_hash
        );
        // Sequence editor invocation handling will be wired in Issue #17
        return;
    }

    let mut app = AppState::new(args.path);
    println!("Initialized Git-tardis for repository: {:?}", app.repo_path);
    println!("State status: {}", app.status_message);
    println!("Application loop ready.");

    // TUI event loop will be attached in Issue #16
    app.quit();
}
