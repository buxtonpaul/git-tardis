use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use git_tardis::app::AppState;
use git_tardis::ui::{run_event_loop, EventSource, KeyDispatcher, KeymapRegistry};
use ratatui::{backend::TestBackend, Terminal};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::time::Duration;

enum Step {
    /// One poll interval passes with no input.
    Idle,
    Input(Event),
}

struct ScriptedEvents {
    steps: VecDeque<Step>,
}

impl ScriptedEvents {
    fn new(steps: Vec<Step>) -> Self {
        Self {
            steps: steps.into(),
        }
    }
}

impl EventSource for ScriptedEvents {
    fn poll(&mut self, timeout: Duration) -> std::io::Result<bool> {
        match self.steps.front() {
            Some(Step::Input(_)) => Ok(true),
            Some(Step::Idle) => {
                if !timeout.is_zero() {
                    self.steps.pop_front();
                }
                Ok(false)
            }
            None => panic!("event script ran out before the app quit"),
        }
    }

    fn read(&mut self) -> std::io::Result<Event> {
        match self.steps.pop_front() {
            Some(Step::Input(event)) => Ok(event),
            _ => panic!("read called with no event ready"),
        }
    }
}

fn key(c: char) -> Step {
    Step::Input(Event::Key(KeyEvent::new(
        KeyCode::Char(c),
        KeyModifiers::NONE,
    )))
}

fn key_release(c: char) -> Step {
    Step::Input(Event::Key(KeyEvent::new_with_kind(
        KeyCode::Char(c),
        KeyModifiers::NONE,
        KeyEventKind::Release,
    )))
}

fn idle(n: usize) -> Vec<Step> {
    (0..n).map(|_| Step::Idle).collect()
}

fn test_app() -> AppState {
    let mut app = AppState::new(PathBuf::from("."));
    app.files = (0..10).map(|i| format!("missing_file_{}.txt", i)).collect();
    // Pin the version so its background lookup cannot add a redraw part-way through a test.
    app.git_version = Some("git version 0.0.0-test".to_string());
    app
}

fn run(app: &mut AppState, steps: Vec<Step>) -> usize {
    let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
    let mut dispatcher = KeyDispatcher::new(KeymapRegistry::new());
    let mut events = ScriptedEvents::new(steps);
    run_event_loop(&mut terminal, app, &mut dispatcher, &mut events).unwrap()
}

#[test]
fn test_idle_loop_draws_only_the_first_frame() {
    let mut app = test_app();
    let mut steps = idle(50);
    steps.push(key('q'));

    let frames = run(&mut app, steps);

    assert_eq!(frames, 1);
    assert!(!app.running);
}

#[test]
fn test_key_press_triggers_exactly_one_redraw() {
    let mut app = test_app();
    let mut steps = vec![key('j')];
    steps.extend(idle(10));
    steps.push(key('q'));

    let frames = run(&mut app, steps);

    assert_eq!(app.file_selected, 1);
    assert_eq!(frames, 2);
}

#[test]
fn test_queued_keys_are_applied_before_the_next_frame() {
    let mut app = test_app();
    let mut steps: Vec<Step> = (0..8).map(|_| key('j')).collect();
    steps.extend(idle(1));
    steps.push(key('q'));

    let frames = run(&mut app, steps);

    assert_eq!(app.file_selected, 8, "every queued key must be applied");
    assert!(
        frames < 8,
        "queued keys should share frames, but {} frames were drawn",
        frames
    );
}

#[test]
fn test_resize_redraws_but_key_release_does_not() {
    let mut app = test_app();
    let mut steps = vec![key_release('j')];
    steps.extend(idle(1));
    steps.push(Step::Input(Event::Resize(80, 24)));
    steps.extend(idle(1));
    steps.push(key('q'));

    let frames = run(&mut app, steps);

    assert_eq!(app.file_selected, 0, "key release must not move the cursor");
    assert_eq!(frames, 2);
}
