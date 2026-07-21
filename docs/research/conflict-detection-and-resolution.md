# Research: Conflict Detection and Resolution UI Workflow

This research document details the user perspective, state transitions, and process handling for merge and rebase conflicts during "Inline rewrite" and "Edit here" operations in Git-tardis.

## Executive Summary

When Git operations encounter conflicts, Git-tardis handles them deterministically based on the operation context:
1. **"Inline rewrite" (Fail-Fast & Auto-Clean)**: Designed as an atomic, seamless in-viewer action. If a conflict occurs during non-interactive rebase, Git-tardis immediately executes `git rebase --abort` in the background to restore working tree safety, autostashes unrelated changes, and displays a prominent informative modal in the TUI detailing the conflicting commit and file.
2. **"Edit here" (Terminal Subshell Delegation)**: Designed as an interactive historical pause. Control is handed to an interactive shell with TUI rendering suspended. If subsequent commits conflict during `git rebase --continue`, standard Git CLI conflict messages output directly to terminal stdout. The user resolves conflicts in their terminal/editor, and TUI rendering resumes when the subshell exits.

---

## 1. Inline Rewrite Conflict Specification

The "Inline rewrite" operation allows users to edit lines in a historical commit $C_{target}$ directly from the TUI file viewer.

### Step-by-Step State Lifecycle:

```text
[TUI View] ──(Trigger Rewrite)──> [Stage Fixup & Autostash]
                                         │
                                   [Execute Rebase]
                                         │
                           ┌─────────────┴─────────────┐
                    (Rebase Success)            (Conflict Detected)
                           │                           │
                   [Update TUI State]           [git rebase --abort]
                                                       │
                                            [Render Conflict Modal]
                                                       │
                                             (User Press Enter/Esc)
                                                       │
                                              [Return to TUI View]
```

1. **Pre-flight & Working Tree Safety**:
   * Git-tardis stages only the intended fixup edits (`git commit --fixup <C_target>`).
   * Unrelated uncommitted working tree changes are automatically stashed via `--autostash` (`git rebase -i --autosquash --autostash <upstream>`).
2. **Conflict Detection**:
   * If `git rebase` exits with a non-zero exit code (e.g. exit status `1`) and `.git/rebase-merge` exists, a conflict has occurred.
3. **Automatic Cleanup & Rebase Abort**:
   * Git-tardis immediately executes `git rebase --abort` in the background.
   * Any autostashed uncommitted changes are restored automatically.
   * The repository and working tree return to their exact pre-rewrite state without leaving orphan rebase artifacts.
4. **TUI Conflict Modal Presentation**:
   * Git-tardis renders an overlay modal dialog over the active panel:
     ```text
     ┌─────────────────────────────────────────────────────────────┐
     │                   Inline Rewrite Failed                     │
     ├─────────────────────────────────────────────────────────────┤
     │ Changes in 'src/main.rs' conflict with commit 3aaba99       │
     │ ("Commit B: Feature component").                            │
     │                                                             │
     │ Rebase has been aborted and repository restored.            │
     │                                                             │
     │                 [Press Enter or Esc to dismiss]             │
     └─────────────────────────────────────────────────────────────┘
     ```
   * Pressing `Enter` or `Esc` dismisses the modal and returns focus to the file viewer.

---

## 2. Edit Here Conflict Specification

The "Edit here" operation pauses rebase at commit $C_{target}$ and hands control to an interactive subshell.

### Step-by-Step State Lifecycle:

1. **TUI Suspension & Subshell Execution**:
   * Crossterm raw mode and alternate screen are suspended (`disable_raw_mode()`, `LeaveAlternateScreen`).
   * An interactive subshell (`$SHELL`) is spawned with full stdin/stdout/stderr terminal control.
2. **Rebase Continuation & Conflict Resolution**:
   * The user edits files and runs `git commit --amend`, followed by `git rebase --continue`.
   * If applying subsequent commits ($C_{target+1}, C_{target+2}, \dots$) causes a merge conflict, Git CLI outputs standard conflict information to the terminal stdout:
     ```text
     Auto-merging src/main.rs
     CONFLICT (content): Merge conflict in src/main.rs
     error: could not apply 2c07287... Commit C
     hint: Resolve all conflicts manually...
     ```
   * The user resolves conflicts using their standard terminal tools (`nvim`, `git add`, `git rebase --continue`, or `git rebase --abort`).
3. **TUI Resumption**:
   * When the rebase completes or is aborted and the user exits the subshell, Git-tardis re-enables Crossterm raw mode and alternate screen (`enable_raw_mode()`, `EnterAlternateScreen`) and redraws the updated state.

---

## 3. Summary of Architectural Decisions

* **Fail-Fast for Inline Rewrite**: Automatically aborting conflicted inline rewrites guarantees that the TUI never gets stuck in an ambiguous background rebase state.
* **Autostash Protection**: Using `git rebase --autostash` protects uncommitted user work without refusing operations or requiring manual stashing.
* **Native Terminal Delegation**: Delegating interactive conflict resolution during "Edit here" to the standard terminal shell avoids building complex, fragile 3-way merge editors inside Ratatui.
