use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

/// Number of rows at the top of the art that make up the roof lamp.
const LAMP_ROWS: usize = 2;

/// The TARDIS, drawn with box-drawing and block characters. Every row has the same width so
/// the art stays aligned when centred.
pub fn get_tardis_ascii_art() -> Vec<&'static str> {
    vec![
        "          ▄          ",
        "         ▐█▌         ",
        "     ▄▄▄▄▄▄▄▄▄▄▄     ",
        "  ▄███████████████▄  ",
        " ┏━━━━━━━━━━━━━━━━━┓ ",
        " ┃   POLICE  BOX   ┃ ",
        " ┣━━━━━━━━┳━━━━━━━━┫ ",
        " ┃ ┌────┐ ┃ ┌────┐ ┃ ",
        " ┃ │░░░░│ ┃ │░░░░│ ┃ ",
        " ┃ ├────┤ ┃ ├────┤ ┃ ",
        " ┃ │ ≡≡ │ ┃ │    │ ┃ ",
        " ┃ ├────┤ ┃ ├────┤ ┃ ",
        " ┃ │    │ ┃ │    │ ┃ ",
        " ┃ ├────┤ ┃ ├────┤ ┃ ",
        " ┃ │    │ ┃ │    │ ┃ ",
        " ┃ └────┘ ┃ └────┘ ┃ ",
        " ┗━━━━━━━━┻━━━━━━━━┛ ",
        "▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀",
    ]
}

/// Colour one row of the art: a yellow lamp, white lettering, windows and door notice, and a
/// blue box.
fn style_art_row(row_index: usize, row: &str) -> Line<'static> {
    let box_style = Style::default()
        .fg(Color::Blue)
        .add_modifier(Modifier::BOLD);
    let lamp_style = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    let detail_style = Style::default()
        .fg(Color::White)
        .add_modifier(Modifier::BOLD);

    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut run = String::new();
    let mut run_style = box_style;

    for ch in row.chars() {
        let style = if row_index < LAMP_ROWS {
            lamp_style
        } else if ch.is_ascii_alphabetic() || ch == '░' || ch == '≡' {
            detail_style
        } else {
            box_style
        };
        // Spaces carry no colour, so they extend whichever run is open.
        if style != run_style && ch != ' ' && !run.is_empty() {
            spans.push(Span::styled(std::mem::take(&mut run), run_style));
        }
        if ch != ' ' || run.is_empty() {
            run_style = style;
        }
        run.push(ch);
    }
    if !run.is_empty() {
        spans.push(Span::styled(run, run_style));
    }

    Line::from(spans)
}

pub fn render_splashscreen_lines(git_version: Option<&str>) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    for (row_index, row) in get_tardis_ascii_art().into_iter().enumerate() {
        lines.push(style_art_row(row_index, row));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(
            "Git-tardis",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" - Code Time-Travel Navigator"),
    ]));

    let version_str = format!("Version: v{}", env!("CARGO_PKG_VERSION"));
    lines.push(Line::from(Span::styled(
        version_str,
        Style::default().fg(Color::Yellow),
    )));

    if let Some(git_v) = git_version {
        lines.push(Line::from(Span::styled(
            format!("System Git: {}", git_v),
            Style::default().fg(Color::DarkGray),
        )));
    }

    lines.push(Line::from(Span::styled(
        "Copyright (c) 2026 Paul Buxton",
        Style::default().fg(Color::White),
    )));

    lines.push(Line::from(""));
    for hint in [
        "Press [?] for Keybindings Help | [Tab] Switch Focus Panel",
        "[1-3] Sidebar Tabs | [c] Filter",
    ] {
        lines.push(Line::from(Span::styled(
            hint,
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::ITALIC),
        )));
    }

    lines.into_iter().map(Line::centered).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_art_rows_share_one_width() {
        let art = get_tardis_ascii_art();
        let width = art[0].chars().count();
        for row in &art {
            assert_eq!(row.chars().count(), width, "row {:?}", row);
        }
    }

    #[test]
    fn test_styled_row_keeps_text_and_colours_details() {
        let art = get_tardis_ascii_art();
        for (i, row) in art.iter().enumerate() {
            let line = style_art_row(i, row);
            let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
            assert_eq!(&text, row);
        }

        let lamp = style_art_row(0, art[0]);
        assert!(lamp.spans.iter().all(|s| s.style.fg == Some(Color::Yellow)));

        let sign_row = art.iter().position(|r| r.contains("POLICE  BOX")).unwrap();
        let sign = style_art_row(sign_row, art[sign_row]);
        let lettering = sign
            .spans
            .iter()
            .find(|s| s.content.contains("POLICE  BOX"))
            .expect("sign lettering should be a single span");
        assert_eq!(lettering.style.fg, Some(Color::White));
    }
}
