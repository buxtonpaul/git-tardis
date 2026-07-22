use std::env;
use std::path::PathBuf;

use git_tardis::git::GitRepo;

fn main() {
    let repo_path = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));

    println!("Opening Git repository at: {}", repo_path.display());

    let repo = match GitRepo::open(&repo_path) {
        Ok(repo) => repo,
        Err(err) => {
            eprintln!("Error opening repository: {}", err);
            std::process::exit(1);
        }
    };

    println!("\n--- 1. File Listing ---");
    match repo.list_files() {
        Ok(files) => println!("Found {} tracked/untracked files.", files.len()),
        Err(e) => eprintln!("Error listing files: {}", e),
    }

    println!("\n--- 2. Repository Status ---");
    match repo.get_status() {
        Ok(statuses) => {
            if statuses.is_empty() {
                println!("Working tree clean.");
            } else {
                for status in &statuses {
                    println!("  [{}] {}", status.status_code(), status.path);
                }
            }
        }
        Err(e) => eprintln!("Error getting status: {}", e),
    }

    println!("\n--- 3. Commit History (Last 5) ---");
    match repo.get_commit_history(Some(5)) {
        Ok(commits) => {
            for commit in &commits {
                println!(
                    "  {} [{}] {} - {}",
                    commit.short_hash, commit.author, commit.date, commit.summary
                );
            }
        }
        Err(e) => eprintln!("Error getting commits: {}", e),
    }

    println!("\n--- 4. Blame Query (Cargo.toml lines 1-5) ---");
    match repo.get_blame("Cargo.toml", Some(1), Some(5)) {
        Ok(blame) => {
            for line in &blame {
                println!(
                    "  L{:<2} ({}) {} | {}",
                    line.final_line,
                    &line.commit_hash[..7],
                    line.author,
                    line.content
                );
            }
        }
        Err(e) => eprintln!("Error getting blame: {}", e),
    }
}
