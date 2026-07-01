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
    /// Capture conventions from a session transcript (used by the Stop hook)
    Capture {
        /// Path to the session transcript file
        transcript: String,
    },
    /// Review pending conventions
    Review {
        /// Accept a pending convention by id (or prefix)
        #[arg(long)]
        accept: Option<String>,
        /// Reject a pending convention by id (or prefix)
        #[arg(long)]
        reject: Option<String>,
    },
    /// Hook entrypoint for plugins (reads the hook JSON on stdin)
    Hook {
        /// session-start | stop
        event: String,
    },
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
        Cmd::Capture { transcript } => {
            let provider = agent_cli::detect()
                .ok_or_else(|| anyhow::anyhow!("no LLM provider; install Claude Code or Codex"))?;
            let ctx = recall_inject::detect_context(&std::env::current_dir()?);
            println!(
                "{}",
                recall_cli::cmd_capture(
                    &db,
                    std::path::Path::new(&transcript),
                    &ctx,
                    provider.as_ref()
                )
                .await?
            );
        }
        Cmd::Review { accept, reject } => {
            if let Some(id) = accept {
                let provider = agent_cli::detect();
                println!(
                    "{}",
                    recall_cli::cmd_review_accept(&db, &id, provider.as_deref()).await?
                );
            } else if let Some(id) = reject {
                println!("{}", recall_cli::cmd_review_reject(&db, &id)?);
            } else {
                println!("{}", recall_cli::cmd_review_list(&db)?);
            }
        }
        Cmd::Hook { event } => {
            use std::io::Read;
            let mut input = String::new();
            std::io::stdin().read_to_string(&mut input).ok();
            match event.as_str() {
                "session-start" => {
                    let out = recall_cli::hook_session_start(&db, &input)?;
                    if !out.is_empty() {
                        println!("{out}");
                    }
                }
                "stop" => {
                    if let Some(tp) = recall_cli::hook_stop_transcript(&input) {
                        // fire-and-forget: run capture in the background, don't block session end.
                        // Explicit stdio redirection is required — without it the child inherits
                        // the parent's fds and keeps the hook's stdout pipe open for the child's
                        // entire (LLM-round-trip-bound) lifetime, defeating fire-and-forget.
                        if let Ok(exe) = std::env::current_exe() {
                            let _ = std::process::Command::new(exe)
                                .arg("capture")
                                .arg(tp)
                                .stdin(std::process::Stdio::null())
                                .stdout(std::process::Stdio::null())
                                .stderr(std::process::Stdio::null())
                                .spawn();
                        }
                    }
                }
                "pre-tool-use" => {
                    let mode = recall_cli::EnforceMode::from_env();
                    if mode != recall_cli::EnforceMode::Off {
                        if let Some(provider) = agent_cli::detect() {
                            if let Some(out) = recall_cli::cmd_hook_pre_tool_use(
                                &db,
                                &input,
                                mode,
                                provider.as_ref(),
                            )
                            .await?
                            {
                                println!("{out}");
                            }
                        }
                    }
                }
                other => eprintln!("unknown hook event: {other}"),
            }
        }
    }
    Ok(())
}
