use git_tardis::app::{AppState, NavigationMode};
use git_tardis::ui::Action;
use std::path::PathBuf;
use std::time::Instant;

fn get_perf_repo_path() -> Option<PathBuf> {
    if let Ok(env_path) = std::env::var("GIT_TARDIS_PERF_REPO") {
        let p = PathBuf::from(env_path);
        if p.exists() && p.join(".git").exists() {
            return Some(p);
        }
    }

    if let Ok(home) = std::env::var("HOME") {
        let p = PathBuf::from(home).join("devel/qemu");
        if p.exists() && p.join(".git").exists() {
            return Some(p);
        }
    }

    None
}

#[test]
fn test_qemu_commit_switching_and_scrolling_performance() {
    let qemu_path = match get_perf_repo_path() {
        Some(p) => p,
        None => {
            println!(
                "Skipping QEMU performance test: repository not found in $GIT_TARDIS_PERF_REPO or $HOME/devel/qemu."
            );
            return;
        }
    };

    println!("\n=== QEMU Repository Performance Benchmark ===");
    println!("Repository path: {:?}", qemu_path);

    // 1. Measure initial AppState creation
    let t_start_init = Instant::now();
    let mut app = AppState::new(qemu_path);
    let dur_init = t_start_init.elapsed();
    println!("[Init] AppState initialization: {:?}", dur_init);

    // 2. Load file: accel/tcg/cpu-exec.c
    let target_file = "accel/tcg/cpu-exec.c";
    app.active_file = Some(target_file.to_string());

    let repo = app.repo().unwrap();

    // Measure individual component timings for initial file load
    let t_file = Instant::now();
    let content = repo.get_file_at_commit("HEAD", target_file).unwrap();
    let dur_get_file = t_file.elapsed();

    let t_blame = Instant::now();
    let _blame = repo.get_blame_at_commit(Some("HEAD"), target_file, Some(100), Some(100));
    let dur_get_blame = t_blame.elapsed();

    let t_cands = Instant::now();
    let file_commits = repo.get_file_commits(target_file, None).unwrap();
    let dur_file_commits = t_cands.elapsed();

    let t_diff_hl = Instant::now();
    let _diff_hl = repo.get_working_diff(Some(target_file)).unwrap();
    let dur_diff_hl = t_diff_hl.elapsed();

    println!("\n--- Component Breakdown for Initial File Load ---");
    println!(
        "  repo.get_file_at_commit('HEAD'): {:?} ({} lines)",
        dur_get_file,
        content.lines().count()
    );
    println!("  repo.get_blame_at_commit: {:?}", dur_get_blame);
    println!(
        "  repo.get_file_commits ('git log --follow'): {:?} ({} commits)",
        dur_file_commits,
        file_commits.len()
    );
    println!("  repo.get_working_diff ('git diff'): {:?}", dur_diff_hl);

    // Now load file through AppState
    app.load_currently_selected_file();
    app.update_candidate_commits();

    // 3. Detailed breakdown of a single JumpPrevFile call
    println!("\n--- Detailed Breakdown of 1 Commit Switch (JumpPrevFile) ---");
    let c_start = Instant::now();

    let old_commit = app.selected_commit_hash.clone();
    app.dispatch_action(Action::JumpPrevFile);
    let new_commit = app.selected_commit_hash.clone().unwrap();

    let t_step_total = c_start.elapsed();
    println!("  Total JumpPrevFile duration: {:?}", t_step_total);

    let t_diff_between = Instant::now();
    let _diff_between = repo
        .get_diff_between(old_commit.as_deref(), Some(&new_commit), target_file)
        .unwrap();
    let dur_diff_between = t_diff_between.elapsed();
    println!(
        "  repo.get_diff_between ({:?} -> {}): {:?}",
        old_commit,
        &new_commit[..7],
        dur_diff_between
    );

    let t_diff_file = Instant::now();
    let _diff_file = repo.get_diff_file(&new_commit, target_file).unwrap();
    let dur_diff_file = t_diff_file.elapsed();
    println!("  repo.get_diff_file (for highlights): {:?}", dur_diff_file);

    // 4. Benchmark commit-to-commit switching (JumpPrevFile x 10)
    app.set_navigation_mode(NavigationMode::File);
    let mut commit_switch_times = Vec::new();

    println!("\n--- Benchmarking 10 Commit Switches (JumpPrevFile) ---");
    for i in 1..=10 {
        let t_step = Instant::now();
        app.dispatch_action(Action::JumpPrevFile);
        let step_dur = t_step.elapsed();
        commit_switch_times.push(step_dur);

        let hash_str = app
            .selected_commit_hash
            .as_deref()
            .map(|h| &h[..7.min(h.len())])
            .unwrap_or("NONE");

        println!(
            "  Switch #{:2}: commit {} | took {:?}",
            i, hash_str, step_dur
        );
    }

    let total_switch_dur: std::time::Duration = commit_switch_times.iter().sum();
    let avg_switch_dur = total_switch_dur / 10;
    println!(
        "\n[Commit Switching Summary] Total for 10 switches: {:?} (Avg: {:?}/switch)",
        total_switch_dur, avg_switch_dur
    );

    // 5. Benchmark reverse commit switching (JumpNextFile x 10) - tests cached revisiting
    println!("\n--- Benchmarking 10 Reverse Commit Switches (JumpNextFile) ---");
    let mut reverse_switch_times = Vec::new();
    for i in 1..=10 {
        let t_step = Instant::now();
        app.dispatch_action(Action::JumpNextFile);
        let step_dur = t_step.elapsed();
        reverse_switch_times.push(step_dur);

        let hash_str = app
            .selected_commit_hash
            .as_deref()
            .map(|h| &h[..7.min(h.len())])
            .unwrap_or("HEAD");

        println!(
            "  Reverse #{:2}: commit {} | took {:?}",
            i, hash_str, step_dur
        );
    }

    let total_reverse_dur: std::time::Duration = reverse_switch_times.iter().sum();
    let avg_reverse_dur = total_reverse_dur / 10;
    println!(
        "\n[Cached Reverse Switching Summary] Total for 10 reverse switches: {:?} (Avg: {:?}/switch)",
        total_reverse_dur, avg_reverse_dur
    );

    println!("\n=== Benchmark Complete ===");
}
