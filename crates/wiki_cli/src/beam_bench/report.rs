// Where: crates/wiki_cli/src/beam_bench/report.rs
// What: Machine-readable BEAM RAG results plus compact summary/report rendering.
// Why: The benchmark must separate retrieval and answer failures so regressions are attributable.
use anyhow::{Context, Result};
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

use super::dataset::BeamQuestionClass;
use super::model::ToolCallRecord;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureReason {
    MissedGoldPath,
    ReadWithoutSpan,
    WrongShortAnswer,
    ToolError,
    RoundtripLimit,
}

#[derive(Debug, Clone, Serialize)]
pub struct QuestionResult {
    pub conversation_id: String,
    pub question_id: String,
    pub question_type: String,
    pub question_class: BeamQuestionClass,
    pub prompt: String,
    pub reference_answer: Option<String>,
    pub predicted_answer: Option<String>,
    pub gold_paths: Vec<String>,
    pub gold_spans: Vec<String>,
    pub retrieved_paths: Vec<String>,
    pub matched_gold_path: Option<String>,
    pub matched_gold_span: Option<String>,
    pub source_note_type: Option<String>,
    pub included_in_primary_metrics: bool,
    pub retrieval_evaluable: bool,
    pub retrieval_hit: bool,
    pub answer_exact_match: bool,
    pub answer_normalized_match: bool,
    pub tool_call_count: usize,
    pub tool_error_count: usize,
    pub docs_read_count: usize,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub total_tokens: Option<u64>,
    pub latency_ms: u128,
    pub failure_reason: Option<FailureReason>,
    pub tool_calls: Vec<ToolCallRecord>,
    pub raw_events: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BenchmarkSummary {
    pub total_questions: usize,
    pub primary_questions: usize,
    pub retrieval_questions: usize,
    pub retrieval_hits: usize,
    pub answer_exact_matches: usize,
    pub answer_normalized_matches: usize,
    pub retrieval_hit_rate: f64,
    pub answer_exact_match_rate: f64,
    pub answer_normalized_match_rate: f64,
    pub total_tool_calls: usize,
    pub total_tool_errors: usize,
    pub average_docs_read_per_question: f64,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_tokens: u64,
    pub avg_latency_ms: f64,
    pub by_question_class: BTreeMap<String, ClassSummary>,
    pub by_source_note_type: BTreeMap<String, usize>,
    pub failure_reasons: BTreeMap<String, usize>,
    pub top_k_hit_rate_by_rank: Vec<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClassSummary {
    pub questions: usize,
    pub retrieval_hits: usize,
    pub answer_normalized_matches: usize,
}

pub fn summarize(results: &[QuestionResult], top_k: u32) -> BenchmarkSummary {
    let primary: Vec<&QuestionResult> = results
        .iter()
        .filter(|item| item.included_in_primary_metrics)
        .collect();
    let retrieval: Vec<&QuestionResult> = primary
        .iter()
        .copied()
        .filter(|item| item.retrieval_evaluable)
        .collect();
    let retrieval_hits = retrieval.iter().filter(|item| item.retrieval_hit).count();
    let answer_exact_matches = primary
        .iter()
        .filter(|item| item.answer_exact_match)
        .count();
    let answer_normalized_matches = primary
        .iter()
        .filter(|item| item.answer_normalized_match)
        .count();
    let total_input_tokens = results.iter().filter_map(|item| item.input_tokens).sum();
    let total_output_tokens = results.iter().filter_map(|item| item.output_tokens).sum();
    let total_tokens = results.iter().filter_map(|item| item.total_tokens).sum();
    let total_tool_calls = results.iter().map(|item| item.tool_call_count).sum();
    let total_tool_errors = results.iter().map(|item| item.tool_error_count).sum();
    let average_docs_read_per_question = if results.is_empty() {
        0.0
    } else {
        results
            .iter()
            .map(|item| item.docs_read_count as f64)
            .sum::<f64>()
            / results.len() as f64
    };
    let avg_latency_ms = if results.is_empty() {
        0.0
    } else {
        results
            .iter()
            .map(|item| item.latency_ms as f64)
            .sum::<f64>()
            / results.len() as f64
    };
    BenchmarkSummary {
        total_questions: results.len(),
        primary_questions: primary.len(),
        retrieval_questions: retrieval.len(),
        retrieval_hits,
        answer_exact_matches,
        answer_normalized_matches,
        retrieval_hit_rate: ratio(retrieval_hits, retrieval.len()),
        answer_exact_match_rate: ratio(answer_exact_matches, primary.len()),
        answer_normalized_match_rate: ratio(answer_normalized_matches, primary.len()),
        total_tool_calls,
        total_tool_errors,
        average_docs_read_per_question,
        total_input_tokens,
        total_output_tokens,
        total_tokens,
        avg_latency_ms,
        by_question_class: summarize_by_class(results),
        by_source_note_type: summarize_note_types(primary.as_slice()),
        failure_reasons: summarize_failure_reasons(results),
        top_k_hit_rate_by_rank: summarize_top_k(retrieval.as_slice(), top_k),
    }
}

pub fn write_artifacts(
    output_dir: &Path,
    summary: &BenchmarkSummary,
    results: &[QuestionResult],
) -> Result<()> {
    fs::create_dir_all(output_dir)
        .with_context(|| format!("failed to create output dir: {}", output_dir.display()))?;
    fs::write(
        output_dir.join("summary.json"),
        serde_json::to_vec_pretty(summary)?,
    )?;
    write_jsonl(output_dir.join("results.jsonl").as_path(), results.iter())?;
    write_jsonl(
        output_dir.join("failures.jsonl").as_path(),
        results.iter().filter(|item| item.failure_reason.is_some()),
    )?;
    fs::write(output_dir.join("report.md"), render_report(summary))?;
    Ok(())
}

fn summarize_by_class(results: &[QuestionResult]) -> BTreeMap<String, ClassSummary> {
    let mut summary = BTreeMap::new();
    for result in results {
        let key = serde_json::to_string(&result.question_class)
            .unwrap_or_else(|_| "\"unknown\"".to_string())
            .trim_matches('"')
            .to_string();
        let entry = summary.entry(key).or_insert(ClassSummary {
            questions: 0,
            retrieval_hits: 0,
            answer_normalized_matches: 0,
        });
        entry.questions += 1;
        if result.retrieval_evaluable && result.retrieval_hit {
            entry.retrieval_hits += 1;
        }
        if result.answer_normalized_match {
            entry.answer_normalized_matches += 1;
        }
    }
    summary
}

fn summarize_note_types(results: &[&QuestionResult]) -> BTreeMap<String, usize> {
    let mut summary = BTreeMap::new();
    for result in results {
        if let Some(note_type) = &result.source_note_type {
            *summary.entry(note_type.clone()).or_insert(0) += 1;
        }
    }
    summary
}

fn summarize_failure_reasons(results: &[QuestionResult]) -> BTreeMap<String, usize> {
    let mut summary = BTreeMap::new();
    for result in results {
        if let Some(reason) = result.failure_reason {
            let key = serde_json::to_string(&reason)
                .unwrap_or_else(|_| "\"unknown\"".to_string())
                .trim_matches('"')
                .to_string();
            *summary.entry(key).or_insert(0) += 1;
        }
    }
    summary
}

fn summarize_top_k(results: &[&QuestionResult], top_k: u32) -> Vec<f64> {
    let mut rates = Vec::new();
    for rank in 1..=top_k.max(1) {
        let hits = results
            .iter()
            .filter(|item| {
                item.gold_paths.iter().any(|gold| {
                    item.retrieved_paths
                        .iter()
                        .take(rank as usize)
                        .any(|path| path == gold)
                })
            })
            .count();
        rates.push(ratio(hits, results.len()));
    }
    rates
}

fn write_jsonl<'a, T>(path: &Path, values: impl Iterator<Item = &'a T>) -> Result<()>
where
    T: Serialize + 'a,
{
    let mut file =
        File::create(path).with_context(|| format!("failed to create {}", path.display()))?;
    for value in values {
        writeln!(file, "{}", serde_json::to_string(value)?)
            .with_context(|| format!("failed to write {}", path.display()))?;
    }
    Ok(())
}

fn render_report(summary: &BenchmarkSummary) -> String {
    format!(
        "# BEAM RAG Benchmark Report\n\n- total questions: {}\n- primary questions: {}\n- retrieval questions: {}\n- retrieval hit rate: {:.4}\n- answer exact match rate: {:.4}\n- answer normalized match rate: {:.4}\n- total tool calls: {}\n- total tool errors: {}\n- average docs read: {:.2}\n- total input tokens: {}\n- total output tokens: {}\n- total tokens: {}\n- average latency ms: {:.2}\n",
        summary.total_questions,
        summary.primary_questions,
        summary.retrieval_questions,
        summary.retrieval_hit_rate,
        summary.answer_exact_match_rate,
        summary.answer_normalized_match_rate,
        summary.total_tool_calls,
        summary.total_tool_errors,
        summary.average_docs_read_per_question,
        summary.total_input_tokens,
        summary.total_output_tokens,
        summary.total_tokens,
        summary.avg_latency_ms
    )
}

fn ratio(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        return 0.0;
    }
    numerator as f64 / denominator as f64
}

#[cfg(test)]
mod tests {
    use super::{BenchmarkSummary, FailureReason, QuestionResult, summarize};
    use crate::beam_bench::dataset::BeamQuestionClass;

    fn result(id: &str, class: BeamQuestionClass) -> QuestionResult {
        QuestionResult {
            conversation_id: "conv".to_string(),
            question_id: id.to_string(),
            question_type: "factoid".to_string(),
            question_class: class,
            prompt: "When?".to_string(),
            reference_answer: Some("March 15, 2024".to_string()),
            predicted_answer: Some("March 15, 2024".to_string()),
            gold_paths: vec!["/Wiki/beam/run/conv/messages/0002-assistant.md".to_string()],
            gold_spans: vec!["March 15, 2024".to_string()],
            retrieved_paths: vec!["/Wiki/beam/run/conv/messages/0002-assistant.md".to_string()],
            matched_gold_path: Some("/Wiki/beam/run/conv/messages/0002-assistant.md".to_string()),
            matched_gold_span: Some("March 15, 2024".to_string()),
            source_note_type: Some("messages".to_string()),
            included_in_primary_metrics: class == BeamQuestionClass::Factoid,
            retrieval_evaluable: true,
            retrieval_hit: true,
            answer_exact_match: true,
            answer_normalized_match: true,
            tool_call_count: 2,
            tool_error_count: 0,
            docs_read_count: 1,
            input_tokens: Some(0),
            output_tokens: Some(0),
            total_tokens: Some(0),
            latency_ms: 10,
            failure_reason: None,
            tool_calls: Vec::new(),
            raw_events: Vec::new(),
        }
    }

    #[test]
    fn summarize_tracks_primary_metrics_only() {
        let mut reasoning = result("reasoning", BeamQuestionClass::Reasoning);
        reasoning.included_in_primary_metrics = false;
        reasoning.retrieval_hit = false;
        reasoning.answer_normalized_match = false;
        reasoning.failure_reason = Some(FailureReason::MissedGoldPath);
        let summary: BenchmarkSummary =
            summarize(&[result("fact", BeamQuestionClass::Factoid), reasoning], 3);
        assert_eq!(summary.primary_questions, 1);
        assert_eq!(summary.retrieval_questions, 1);
        assert_eq!(summary.retrieval_hits, 1);
        assert_eq!(
            summary.failure_reasons.get("missed_gold_path").copied(),
            Some(1)
        );
        assert_eq!(summary.top_k_hit_rate_by_rank.len(), 3);
    }

    #[test]
    fn summarize_excludes_non_evaluable_retrieval_from_denominator() {
        let mut legacy = result("legacy", BeamQuestionClass::Factoid);
        legacy.retrieval_evaluable = false;
        legacy.retrieval_hit = false;
        let summary = summarize(&[legacy], 3);
        assert_eq!(summary.primary_questions, 1);
        assert_eq!(summary.retrieval_questions, 0);
        assert_eq!(summary.retrieval_hit_rate, 0.0);
    }
}
