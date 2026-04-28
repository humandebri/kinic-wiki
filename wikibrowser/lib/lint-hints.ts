export type LintSeverity = "warning" | "error" | "ok";

export type LintHint = {
  severity: LintSeverity;
  title: string;
  detail: string;
  line: number | null;
};

const futurePattern = /\b(deadline|meeting|check-?in|pending|tomorrow|next\s+\w+|will|plan to|scheduled)\b/i;
const exactValuePattern = /(\b\d{4}-\d{2}-\d{2}\b|\b[A-Z]{2,}-?\d{4,}\b|\$\d|¥\d|\b\d{1,2}:\d{2}\b)/;
const filePathPattern = /(`[^`]+\.[a-z0-9]+`|\/[A-Za-z0-9._/-]+\.[A-Za-z0-9]+)/;

export function collectLintHints(path: string, content: string): LintHint[] {
  const role = path.split("/").at(-1) ?? "";
  const hints: LintHint[] = [];
  if (role === "facts.md") {
    hints.push(...findLineHints(content, futurePattern, "Possible future or pending item", "facts.md should hold stable facts, not schedules, pending decisions, or next actions."));
  }
  if (role === "summary.md") {
    hints.push(...findLineHints(content, exactValuePattern, "Possible exact evidence leak", "summary.md should recap; exact dates, money, receipts, or IDs belong in canonical notes or raw sources."));
  }
  hints.push(...findCodeNoteHints(path, content));
  return hints;
}

function findLineHints(content: string, pattern: RegExp, title: string, detail: string): LintHint[] {
  return content
    .split("\n")
    .map((line, index) => ({ line, index }))
    .filter((entry) => pattern.test(entry.line))
    .slice(0, 8)
    .map((entry) => ({
      severity: "warning",
      title,
      detail,
      line: entry.index + 1
    }));
}

function findCodeNoteHints(path: string, content: string): LintHint[] {
  const hints: LintHint[] = [];
  const codeBlocks = content.match(/```[\s\S]*?```/g) ?? [];
  for (const block of codeBlocks) {
    if (block.split("\n").length > 12) {
      hints.push({
        severity: "warning",
        title: "Long code block",
        detail: "Wiki code notes should point to source paths and decisions, not store long implementation copies.",
        line: firstLineOf(content, block)
      });
      break;
    }
  }
  if (isCodeNote(path, content) && filePathPattern.test(content) && !hasDecisionContext(content)) {
    hints.push({
      severity: "warning",
      title: "Code note lacks decision context",
      detail: "Add Why or Verification so the note records judgment, not just a file pointer.",
      line: null
    });
  }
  return hints;
}

function isCodeNote(path: string, content: string): boolean {
  return path.toLowerCase().includes("code") || /Source of Truth|Implementation:|Tests:/i.test(content);
}

function hasDecisionContext(content: string): boolean {
  return /(^|\n)##\s+(Why|Verification|Current Decision)\b/i.test(content);
}

function firstLineOf(content: string, needle: string): number {
  return content.slice(0, content.indexOf(needle)).split("\n").length;
}
