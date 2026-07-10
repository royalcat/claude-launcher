use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "claude-launcher",
    about = "Easily manage and switch inference provider configurations for Claude Code",
    version
)]
pub struct Cli {
    /// Use a saved workspace for this run only (mutually exclusive with --config)
    #[arg(long, value_name = "LABEL", global = true)]
    pub workspace: Option<String>,

    /// Ad-hoc path to a profiles JSON file (mutually exclusive with --workspace)
    #[arg(long, value_name = "PATH", global = true)]
    pub config: Option<String>,

    /// [Legacy] Directly launch with a specific profile slug
    #[arg(long, value_name = "SLUG", global = true)]
    pub profiles: Option<String>,

    /// Print env vars and command instead of launching (use with --profiles or launch)
    #[arg(long, global = true)]
    pub print: bool,

    #[command(subcommand)]
    pub command: Option<Command>,

    /// Arguments to forward verbatim to `claude` (after --)
    #[arg(last = true, value_name = "CLAUDE_ARGS")]
    pub claude_args: Vec<String>,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// List all saved profiles
    List,

    /// Launch Claude Code with a profile (interactive picker if no slug given)
    Launch {
        /// Profile slug to launch with
        slug: Option<String>,

        /// Print env vars and command instead of launching
        #[arg(long)]
        print: bool,
    },

    /// Output status line text for a profile (for use with Claude Code custom statusLine)
    Statusline {
        /// Profile slug to generate status line for
        #[arg(long, value_name = "SLUG")]
        profile: String,
    },
}
