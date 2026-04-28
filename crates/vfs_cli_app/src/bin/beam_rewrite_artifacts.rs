// Where: crates/vfs_cli_app/src/bin/beam_rewrite_artifacts.rs
// What: Rebuild BEAM artifact files from an existing results.jsonl snapshot.
// Why: Patched aggregate runs should reuse the canonical report writer instead of forking summary logic.
use anyhow::{Context, Result};
use clap::Parser;
use std::fs;
use std::path::PathBuf;
use vfs_cli_app::beam_bench::{QuestionResult, summarize, write_artifacts};

#[derive(Parser, Debug)]
#[command(name = "beam-rewrite-artifacts")]
#[command(about = "Rewrite BEAM artifacts from an existing results.jsonl file")]
struct Cli {
    #[arg(long)]
    results_path: PathBuf,
    #[arg(long)]
    output_dir: PathBuf,
    #[arg(long, default_value_t = 3)]
    top_k: u32,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let raw = fs::read_to_string(&cli.results_path)
        .with_context(|| format!("failed to read {}", cli.results_path.display()))?;
    let mut results = Vec::new();
    for (line_no, line) in raw.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let result: QuestionResult = serde_json::from_str(line).with_context(|| {
            format!(
                "failed to parse {} line {}",
                cli.results_path.display(),
                line_no + 1
            )
        })?;
        results.push(result);
    }
    results.sort_by(|left, right| {
        (&left.conversation_id, &left.question_id)
            .cmp(&(&right.conversation_id, &right.question_id))
    });

    let mut summary = summarize(&results, cli.top_k);
    summary.read_only_eval = true;
    write_artifacts(&cli.output_dir, &summary, &results)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::Cli;
    use clap::CommandFactory;

    #[test]
    fn dedicated_bin_help_mentions_rewrite() {
        let mut command = Cli::command();
        let help = command.render_long_help().to_string();

        assert!(help.contains("beam-rewrite-artifacts"));
        assert!(help.contains("Rewrite BEAM artifacts from an existing results.jsonl file"));
    }
}
