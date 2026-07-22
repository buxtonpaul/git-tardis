use clap::Parser;
use std::path::PathBuf;

/// Time travelling git repository inspector & historical rebase tool
#[derive(Parser, Debug, PartialEq, Eq)]
#[command(author, version, about, long_about = None)]
pub struct CliArgs {
    /// Path to target Git repository directory
    #[arg(default_value = ".")]
    pub path: PathBuf,

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
        assert_eq!(args.mark_edit, None);
        assert_eq!(args.config, None);
    }

    #[test]
    fn test_custom_repo_path() {
        let args = CliArgs::try_parse_from(["git-tardis", "/path/to/repo"]).unwrap();
        assert_eq!(args.path, PathBuf::from("/path/to/repo"));
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
