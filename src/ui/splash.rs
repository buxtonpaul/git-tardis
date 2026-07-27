use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

pub fn get_tardis_ascii_art() -> Vec<&'static str> {
    vec![
        "         .---.         ",
        "        /  |  \\        ",
        "       /   |   \\       ",
        "      |____|____|      ",
        "     | POLICE  BOX |   ",
        "     |====|====|===|   ",
        "     |    |    |   |   ",
        "     |____|____|___|   ",
        "     | [ ] |  | [ ]|   ",
        "     | [ ] |  | [ ]|   ",
        "     |-----|--|----|   ",
        "     | [ ] |  | [ ]|   ",
        "     | [ ] |  | [ ]|   ",
        "     |-----|--|----|   ",
        "     | [ ] |  | [ ]|   ",
        "     | [ ] |  | [ ]|   ",
        "     '============='   ",
    ]
}

pub fn render_splashscreen_lines(git_version: Option<&str>) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    let art = get_tardis_ascii_art();
    for row in art {
        lines.push(Line::from(vec![Span::styled(
            row.to_string(),
            Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        )]));
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
    lines.push(Line::from(Span::styled(
        "Press [?] for Keybindings Help | [Tab] Switch Focus Panel | [1-3] Sidebar Tabs | [c] Filter",
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::ITALIC),
    )));

    lines
}
