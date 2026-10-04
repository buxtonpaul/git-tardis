use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyEventKind};
use ratatui::{backend::Backend, Terminal};

use super::{render, Action, KeyDispatcher};
use crate::app::AppState;

/// How long to wait for input before checking for a pending key-chord timeout.
pub const IDLE_POLL_INTERVAL: Duration = Duration::from_millis(50);

/// Shorter wait used while a background git query is outstanding, so results show promptly.
pub const BACKGROUND_POLL_INTERVAL: Duration = Duration::from_millis(10);

/// Upper bound on time spent applying queued input before the next redraw, so that slow
/// actions still show progress while fast ones are batched into a single frame.
pub const EVENT_BATCH_BUDGET: Duration = Duration::from_millis(16);

/// Source of terminal input events, abstracted so the loop can be driven by tests.
pub trait EventSource {
    fn poll(&mut self, timeout: Duration) -> std::io::Result<bool>;
    fn read(&mut self) -> std::io::Result<Event>;
}

/// Reads events from the real terminal via crossterm.
pub struct CrosstermEvents;

impl EventSource for CrosstermEvents {
    fn poll(&mut self, timeout: Duration) -> std::io::Result<bool> {
        event::poll(timeout)
    }

    fn read(&mut self) -> std::io::Result<Event> {
        event::read()
    }
}

/// Run the interactive loop until the app stops. The screen is only redrawn after something
/// changed, and input that queued up while drawing is applied before the next frame.
///
/// Returns the number of frames drawn.
pub fn run_event_loop<B, E>(
    terminal: &mut Terminal<B>,
    app: &mut AppState,
    dispatcher: &mut KeyDispatcher,
    events: &mut E,
) -> Result<usize, Box<dyn std::error::Error>>
where
    B: Backend,
    B::Error: 'static,
    E: EventSource,
{
    let mut frames_drawn = 0;
    let mut needs_redraw = true;

    while app.running {
        if needs_redraw {
            terminal.draw(|f| render(f, app))?;
            frames_drawn += 1;
            needs_redraw = false;
        }

        let poll_interval = if app.has_pending_background() {
            BACKGROUND_POLL_INTERVAL
        } else {
            IDLE_POLL_INTERVAL
        };
        if events.poll(poll_interval)? {
            let batch_start = Instant::now();
            loop {
                let mut ran_edit_here = false;
                match events.read()? {
                    Event::Key(key) if key.kind == KeyEventKind::Press => {
                        needs_redraw = true;
                        if app.input_prompt.is_some() {
                            app.handle_input_key(key);
                        } else if let Some(action) =
                            dispatcher.handle_event(key, app.active_scope())
                        {
                            ran_edit_here = action == Action::EditHere;
                            app.dispatch_action(action);
                        }
                    }
                    Event::Resize(_, _) => needs_redraw = true,
                    _ => {}
                }

                if ran_edit_here && app.running {
                    // The rebase subshell wrote over the screen; force a full repaint.
                    terminal.clear()?;
                    break;
                }
                if !app.running
                    || batch_start.elapsed() >= EVENT_BATCH_BUDGET
                    || !events.poll(Duration::ZERO)?
                {
                    break;
                }
            }
        } else if app.input_prompt.is_none() {
            if let Some(action) = dispatcher.check_timeout(app.active_scope()) {
                app.dispatch_action(action);
                needs_redraw = true;
            }
        }

        if app.poll_background() {
            needs_redraw = true;
        }
    }

    Ok(frames_drawn)
}
