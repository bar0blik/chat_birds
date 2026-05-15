use clap::{ArgGroup, Parser, Subcommand};

/// Parse a command string (without the leading $).
/// Returns Some(Command) if parsing succeeds, None if it fails.
pub fn parse_command(input: &str) -> Result<Command, String> {
    let args: Vec<&str> = input.split_whitespace().collect();
    Command::try_parse_from(args).map_err(|e| e.to_string())
}

#[derive(Parser, Debug)]
#[command(name = "chirps")]
#[command(no_binary_name = true)]
pub struct Command {
    #[command(subcommand)]
    pub cmd: SubCommand,
}

#[derive(Subcommand, Debug)]
pub enum SubCommand {
    /// Agent management
    Agent(AgentCmd),
    /// Channel management
    Channel(ChannelCmd),
    /// Trigger replies
    Reply(ReplyCmd),
}

/// Agent management commands
#[derive(Parser, Debug)]
#[command(
    group(
        ArgGroup::new("agent_action")
            .required(true)
            .multiple(true)
            .args(["new", "kill", "select", "select_default"])
    )
)]
pub struct AgentCmd {
    /// Create a new agent
    #[arg(long = "new", value_name = "NAME")]
    pub new: Option<String>,

    /// Remove an agent (current or specified)
    #[arg(
        long = "kill",
        value_name = "NAME_OR_ID",
        num_args = 0..=1,
        default_missing_value = ""
    )]
    pub kill: Option<String>,

    /// Select an agent to focus on
    #[arg(long = "select", value_name = "NAME_OR_ID")]
    pub select: Option<String>,

    /// Select an agent to focus on (default action)
    #[arg(value_name = "NAME_OR_ID", conflicts_with = "select")]
    pub select_default: Option<String>,
}

/// Channel management commands
#[derive(Parser, Debug)]
#[command(
    group(
        ArgGroup::new("channel_action")
            .required(true)
            .multiple(true)
            .args(["new", "add", "kick"])
    )
)]
pub struct ChannelCmd {
    /// Reset the channel
    #[arg(long = "new")]
    pub new: bool,

    /// Add agents to the channel
    #[arg(long = "add", value_name = "NAMES_OR_IDS", num_args = 1..)]
    pub add: Vec<String>,

    /// Remove agents from the channel
    #[arg(long = "kick", value_name = "NAMES_OR_IDS", num_args = 1..)]
    pub kick: Vec<String>,
}

/// Trigger replies from agents
#[derive(Parser, Debug)]
pub struct ReplyCmd {
    /// Agent names or IDs to trigger replies from (space-separated)
    #[arg(value_name = "NAMES_OR_IDS")]
    pub targets: Vec<String>,
}
