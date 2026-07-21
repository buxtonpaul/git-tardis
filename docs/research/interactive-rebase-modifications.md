# Research: Interactive Rebase Sequence Modifications in Rust

This research document details the technical mechanics, process environment configurations, and Crossterm TUI lifecycle management required to implement interactive rebase modifications in Git-tardis.

## Executive Summary

Git-tardis requires two core historical rebase operations:
1. **"Inline rewrite"**: A non-interactive, automated autosquash rebase targeting a chosen historical commit. Uncommitted changes are staged and formatted into a fixup commit (`git commit --fixup <target>`), then rebased into the historical commit silently using `git rebase -i --autosquash --autostash` with `GIT_SEQUENCE_EDITOR="true"`.
2. **"Edit here"**: An interactive rebase that automatically pauses at a chosen historical commit. The rebase sequence todo file is modified using a custom `GIT_SEQUENCE_EDITOR` command to change the target commit action from `pick` to `edit`. The TUI process suspends its terminal raw mode and alternate screen, hands interactive control to a subshell or editor, and resumes rendering upon rebase continuation or completion.

Both workflows were implemented and fully verified in Rust on macOS and Linux in `research/interactive-rebase-poc/`.

---

## 1. Sequence Editor Mechanics (`GIT_SEQUENCE_EDITOR`)

When `git rebase -i <upstream>` is executed, Git initializes an interactive rebase instruction sheet (located at `.git/rebase-merge/git-rebase-todo`). Before presenting or executing the todo list, Git spawns the command configured in:
1. `GIT_SEQUENCE_EDITOR` environment variable (highest precedence)
2. `sequence.editor` git config
3. `GIT_EDITOR` environment variable
4. `core.editor` git config

Git executes the sequence editor command, appending the absolute path to the todo file as its final argument (`$1`).

### The `git-rebase-todo` File Structure
The todo file consists of action lines followed by comments:
```text
pick 44c166d Commit B: Feature component
pick 299af0d Commit C: Final additions

# Rebase 0ba5f61..299af0d onto 0ba5f61 (2 commands)
```

Key action verbs:
* `pick` (`p`): Keep and apply the commit as-is.
* `edit` (`e`): Apply commit, but pause rebase execution for manual amending.
* `fixup` (`f`): Meld commit into previous commit, discarding its commit message.
* `squash` (`s`): Meld commit into previous commit, keeping its commit message.
* `break` (`b`): Stop rebase at this point.

---

## 2. Scenario 1: "Inline Rewrite" (Silent Autosquash)

The "Inline rewrite" operation allows users to make quick edits to a file at a specific historical commit $C_{target}$ without leaving the file viewer.

### Step-by-step Workflow:
1. **Upstream Determination**:
   Determine the rebase base commit. If $C_{target}$ has a parent (`git rev-parse --verify --quiet C_target~1`), the upstream is `C_target~1`. If $C_{target}$ is the root commit, the upstream flag is `--root`.
2. **Staging and Fixup Commit**:
   Stage the modified files and create a fixup commit explicitly referencing $C_{target}$:
   ```bash
   git add -u
   git commit --fixup <C_target_hash>
   ```
   This generates a commit with the subject `fixup! <C_target_summary>`.
3. **Automated Non-Interactive Rebase**:
   Invoke `git rebase` with `--autosquash` and `--autostash`:
   ```bash
   GIT_SEQUENCE_EDITOR="true" git rebase -i --autosquash --autostash <upstream>
   ```
   Setting `GIT_SEQUENCE_EDITOR="true"` (or `":"` on Unix) tells Git to accept the automatically generated `--autosquash` sequence without prompting the user in a text editor. Git automatically moves the `fixup!` commit directly beneath $C_{target}$, changes its action verb to `fixup`, and squashes it silently.

### Handling Conflicts Gracefully:
If applying the fixup commit or re-applying subsequent commits causes a merge conflict:
1. `git rebase` exits with a non-zero exit status (e.g. exit status `1`).
2. The directory `.git/rebase-merge` remains on disk, indicating an active rebase session.
3. The application detects the failure, checks for `.git/rebase-merge`, and alerts the user.
4. If the user cancels the resolution, Git-tardis executes `git rebase --abort` to return the working tree and branch back to its exact pre-rebase state.

---

## 3. Scenario 2: "Edit Here" (Pause, Suspend TUI, Edit, Resume)

The "Edit here" operation allows users to jump back in time to commit $C_{target}$, pause the rebase, edit files or run arbitrary commands in their shell, and resume the timeline.

### Programmatically Marking Commit as `edit`:
To pause Git rebase at $C_{target}$, its line in `git-rebase-todo` must be changed from `pick <hash>` to `edit <hash>`.

Using a shell pipeline like `sed -i` is error-prone across operating systems (`sed -i ''` on macOS vs `sed -i` on GNU/Linux). Instead, Git-tardis uses its own binary executable as a cross-platform sequence editor helper:

```bash
GIT_SEQUENCE_EDITOR="/path/to/git-tardis --internal-sequence-editor-mark-edit <C_target_short_hash>" git rebase -i <upstream>
```

When Git calls `git-tardis` with the todo file path:
1. `git-tardis` reads the todo file line by line.
2. It matches the line starting with `pick <C_target_hash>`.
3. It rewrites `pick` to `edit`.
4. It saves the file and exits with code `0`.

Git then proceeds with the rebase, applies $C_{target}$, and stops execution with:
`Stopped at <hash>... Commit summary`
`You can amend the commit now...`

### TUI Suspension and Subshell Execution:
When `git rebase` pauses at $C_{target}$, control returns to the Rust process. To allow the user to perform edits:

1. **Disable TUI Modes**:
   ```rust
   crossterm::terminal::disable_raw_mode()?;
   crossterm::execute!(stdout, LeaveAlternateScreen, Show)?;
   ```
2. **Spawn Interactive Subshell or Shell Out**:
   Retrieve the user's preferred shell (from `$SHELL` or defaulting to `/bin/sh` or `bash`):
   ```rust
   let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
   let mut child = std::process::Command::new(&shell).status()?;
   ```
3. **Continue Rebase and Restore TUI**:
   After the user exits the subshell (or finishes editing):
   ```rust
   // Check if rebase is still in progress and continue
   if repo_path.join(".git/rebase-merge").exists() {
       Command::new("git").args(&["rebase", "--continue"]).status()?;
   }

   // Re-enable TUI rendering
   crossterm::terminal::enable_raw_mode()?;
   crossterm::execute!(stdout, EnterAlternateScreen, Hide)?;
   ```

---

## 4. Rust PoC Implementation and Verification

A self-contained Proof of Concept was constructed in `research/interactive-rebase-poc/`.

### Running the PoC:
```bash
cd research/interactive-rebase-poc
cargo run
```

### Execution Output Evidence:
```text
=== Git-Tardis Interactive Rebase PoC ===
Self binary path: /Users/paulbuxton/devel/git-tardis/research/interactive-rebase-poc/target/debug/interactive-rebase-poc

--- [Scenario 1] Testing Inline Rewrite (Silent Autosquash) ---
[main (root-commit) 0ba5f61] Commit A: Initial setup
[main 44c166d] Commit B: Feature component
[main 299af0d] Commit C: Final additions
Repository created with 3 commits:
  A: 0ba5f61
  B (Target): 44c166d
[main 5cbec53] fixup! Commit B: Feature component
Executing `git rebase -i --autosquash --autostash 44c166d...~1` with GIT_SEQUENCE_EDITOR="true"...
Rebasing (2/3)Rebasing (3/3)Successfully rebased and updated refs/heads/main.
Rebase succeeded!
Updated Git log:
dc50255 Commit C: Final additions
08ffc45 Commit B: Feature component
0ba5f61 Commit A: Initial setup
Verified file_b.txt content contains inline rewrite!

--- [Scenario 1 Conflict] Testing Inline Rewrite Conflict Handling ---
Executing rebase that causes merge conflict...
CONFLICT (content): Merge conflict in conflict.txt
error: could not apply 011a48d... fixup! Commit A: Initial
Rebase exited with non-zero status as expected!
Detected active rebase-merge state in .git!
Successfully aborted conflicting rebase!

--- [Scenario 2] Testing 'Edit Here' (Sequence Editor & TUI Suspend) ---
Targeting Commit B (00c2727) for 'Edit here'
Rebase pausing at target commit. Suspending TUI...
>>> TUI Suspended successfully! <<<
Sequence editor: Successfully marked commit '00c2727' as 'edit'.
Stopped at 00c2727...  # Commit B
Git rebase successfully stopped at commit B as expected!
Simulating user editing files at commit B...
Running `git commit --amend --no-edit`...
Running `git rebase --continue`...
Successfully rebased and updated refs/heads/main.
Resuming TUI state (raw mode, alternate screen)...
Verified 'Edit here' successfully modified Commit B!

=== All Interactive Rebase PoC scenarios completed successfully! ===
```

---

## 5. Architectural Key Takeaways for Git-tardis

1. **Cross-Platform Sequence Editor**: Never rely on external tools like `sed` or `awk` for sequence editing. Use `git-tardis`'s own binary path (`std::env::current_exe()`) as the value for `GIT_SEQUENCE_EDITOR`.
2. **Root Commit Awareness**: Always check if $C_{target}$ is a root commit before calling `git rebase -i`. Use `--root` for root commits and `<hash>~1` for all other commits.
3. **Safe TUI Lifecycles**: Always pair Crossterm screen suspend routines (`disable_raw_mode` / `LeaveAlternateScreen`) with subshell spawning or external process execution to prevent terminal corruption.
