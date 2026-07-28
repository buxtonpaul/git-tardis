use clap::Parser;
use std::path::PathBuf;

/// Time travelling git repository inspector & historical rebase tool
#[derive(Parser, Debug, PartialEq, Eq)]
#[command(author, version, about, long_about = None)]
pub struct CliArgs {
    /// Path to target Git repository directory
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Target file path to focus on startup
    #[arg(short = 'f', long = "file")]
    pub file: Option<PathBuf>,

    /// Target line number to jump to on startup (1-based index)
    #[arg(short = 'l', long = "line")]
    pub line: Option<usize>,

    /// Custom background color override (hex #RRGGBB or color name)
    #[arg(long = "theme-bg", env = "GIT_TARDIS_BG")]
    pub theme_bg: Option<String>,

    /// Custom foreground color override (hex #RRGGBB or color name)
    #[arg(long = "theme-fg", env = "GIT_TARDIS_FG")]
    pub theme_fg: Option<String>,

    /// Internal flag used by GIT_SEQUENCE_EDITOR during 'Edit here' interactive rebase
    #[arg(long)]
    pub mark_edit: Option<String>,

    /// Path to custom TOML configuration file
    #[arg(short, long)]
    pub config: Option<PathBuf>,
}

impl CliArgs {
    pub fn parse_args() -> Self {
        Self::parse()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_cli_args() {
        let args = CliArgs::try_parse_from(["git-tardis"]).unwrap();
        assert_eq!(args.path, PathBuf::from("."));
        assert_eq!(args.file, None);
        assert_eq!(args.line, None);
        assert_eq!(args.theme_bg, None);
        assert_eq!(args.theme_fg, None);
        assert_eq!(args.mark_edit, None);
        assert_eq!(args.config, None);
    }

    #[test]
    fn test_custom_repo_path() {
        let args = CliArgs::try_parse_from(["git-tardis", "/path/to/repo"]).unwrap();
        assert_eq!(args.path, PathBuf::from("/path/to/repo"));
    }

    #[test]
    fn test_file_and_line_args() {
        let args = CliArgs::try_parse_from(["git-tardis", "--file", "src/main.rs", "--line", "42"])
            .unwrap();
        assert_eq!(args.file, Some(PathBuf::from("src/main.rs")));
        assert_eq!(args.line, Some(42));
    }

    #[test]
    fn test_short_file_and_line_flags() {
        let args =
            CliArgs::try_parse_from(["git-tardis", "-f", "src/lib.rs", "-l", "100"]).unwrap();
        assert_eq!(args.file, Some(PathBuf::from("src/lib.rs")));
        assert_eq!(args.line, Some(100));
    }

    #[test]
    fn test_theme_color_args() {
        let args = CliArgs::try_parse_from([
            "git-tardis",
            "--theme-bg",
            "#1e1e2e",
            "--theme-fg",
            "#cdd6f4",
        ])
        .unwrap();
        assert_eq!(args.theme_bg, Some("#1e1e2e".to_string()));
        assert_eq!(args.theme_fg, Some("#cdd6f4".to_string()));
    }

    #[test]
    fn test_mark_edit_flag() {
        let args = CliArgs::try_parse_from(["git-tardis", "--mark-edit", "a1b2c3d"]).unwrap();
        assert_eq!(args.mark_edit, Some("a1b2c3d".to_string()));
    }

    #[test]
    fn test_custom_config_path() {
        let args =
            CliArgs::try_parse_from(["git-tardis", "-c", "/home/user/.git-tardis.toml"]).unwrap();
        assert_eq!(
            args.config,
            Some(PathBuf::from("/home/user/.git-tardis.toml"))
        );
    }
}
