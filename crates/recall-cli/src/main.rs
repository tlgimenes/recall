use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "recall",
    version,
    about = "Your personal coding-convention brain"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Run the Recall MCP server over stdio
    Mcp,
    /// Teach Recall a convention
    Learn {
        /// The rule, e.g. "Import directly; no barrel files"
        rule: String,
        /// global | repo | branch | language:<lang>
        #[arg(long, default_value = "global")]
        scope: String,
        /// Optional tag (repeatable)
        #[arg(long)]
        tag: Vec<String>,
    },
    /// List active conventions
    List,
    /// Show where a convention came from
    Why {
        /// Convention id (or unique prefix)
        id: String,
    },
    /// Retire a convention
    Forget {
        /// Convention id (or unique prefix)
        id: String,
    },
    /// Show Recall status
    Status,
}

fn db_path() -> PathBuf {
    if let Ok(p) = std::env::var("RECALL_DB") {
        return PathBuf::from(p);
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".recall").join("recall.db")
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let db = db_path();
    match cli.cmd {
        Cmd::Mcp => recall_mcp::run_stdio(db).await?,
        Cmd::Learn { rule, scope, tag } => {
            println!("{}", recall_cli::cmd_learn(&db, &rule, &scope, tag)?)
        }
        Cmd::List => println!("{}", recall_cli::cmd_list(&db)?),
        Cmd::Why { id } => println!("{}", recall_cli::cmd_why(&db, &id)?),
        Cmd::Forget { id } => println!("{}", recall_cli::cmd_forget(&db, &id)?),
        Cmd::Status => println!("{}", recall_cli::cmd_status(&db)?),
    }
    Ok(())
}
