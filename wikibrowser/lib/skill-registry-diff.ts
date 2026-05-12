import type { Identity } from "@icp-sdk/core/agent";
import { splitMarkdownFrontmatter } from "@/lib/markdown-frontmatter";
import type { CatalogSkill, SkillProposal } from "@/lib/skill-registry-catalog";
import type { WikiNode } from "@/lib/types";
import { readNode, writeNodeAuthenticated } from "@/lib/vfs-client";

export type ProposalDiffPreview = {
  proposalPath: string;
  targetPath: string;
  nextContent: string;
  currentEtag: string;
  metadataJson: string;
  additions: number;
  removals: number;
};

export async function previewApplyProposalDiff(
  canisterId: string,
  databaseId: string,
  identity: Identity,
  skill: CatalogSkill,
  proposal: SkillProposal
): Promise<ProposalDiffPreview> {
  if (!proposal.diff) throw new Error("Proposal diff is missing.");
  const patch = parseSingleFilePatch(proposal.diff);
  const targetPath = `${skill.basePath}/${patch.path}`;
  if (!targetPath.startsWith(`${skill.basePath}/`) || !targetPath.endsWith(".md")) throw new Error("Diff target must be a package markdown file.");
  const node = await requireNode(canisterId, databaseId, targetPath, identity);
  return {
    proposalPath: proposal.path,
    targetPath,
    nextContent: applyPatch(node.content, patch.hunk),
    currentEtag: node.etag,
    metadataJson: node.metadataJson,
    additions: patch.additions,
    removals: patch.removals
  };
}

export async function applyProposalDiff(
  canisterId: string,
  databaseId: string,
  identity: Identity,
  proposal: SkillProposal,
  preview: ProposalDiffPreview
): Promise<void> {
  await writeNodeAuthenticated(canisterId, identity, {
    databaseId,
    path: preview.targetPath,
    kind: "file",
    content: preview.nextContent,
    metadataJson: preview.metadataJson,
    expectedEtag: preview.currentEtag
  });
  const proposalNode = await requireNode(canisterId, databaseId, proposal.path, identity);
  await writeNodeAuthenticated(canisterId, identity, {
    databaseId,
    path: proposal.path,
    kind: "file",
    content: replaceRootFrontmatter(proposalNode.content, {
      status: "applied",
      applied_at: new Date().toISOString(),
      applied_by: "browser"
    }),
    metadataJson: proposalNode.metadataJson,
    expectedEtag: proposalNode.etag
  });
}

type Patch = {
  path: string;
  hunk: Hunk;
  additions: number;
  removals: number;
};

type Hunk = {
  oldStart: number;
  lines: string[];
};

function parseSingleFilePatch(diff: string): Patch {
  const lines = diff.split("\n");
  if (lines.some((line) => line.startsWith("Binary files ") || line.startsWith("rename ") || line.startsWith("deleted file mode"))) {
    throw new Error("Only text modifications are supported.");
  }
  const target = lines.find((line) => line.startsWith("+++ "));
  if (!target) throw new Error("Unified diff target is missing.");
  const path = cleanDiffPath(target.slice(4).trim());
  let hunk: Hunk | null = null;
  let additions = 0;
  let removals = 0;
  for (const line of lines) {
    if (line.startsWith("@@")) {
      if (hunk) throw new Error("Only one hunk proposal diffs are supported.");
      hunk = { oldStart: parseOldStart(line), lines: [] };
      continue;
    }
    if (!hunk) continue;
    if (line.startsWith("+") && !line.startsWith("+++")) additions += 1;
    if (line.startsWith("-") && !line.startsWith("---")) removals += 1;
    hunk.lines.push(line);
  }
  if (!hunk) throw new Error("Unified diff hunk is missing.");
  if (!hasContext(hunk.lines)) throw new Error("Proposal diff requires context lines.");
  return { path, hunk, additions, removals };
}

function applyPatch(content: string, hunk: Hunk): string {
  const lines = content.split("\n");
  const oldLines = hunk.lines.filter((line) => line.startsWith(" ") || (line.startsWith("-") && !line.startsWith("---"))).map((line) => line.slice(1));
  const newLines = hunk.lines.filter((line) => line.startsWith(" ") || (line.startsWith("+") && !line.startsWith("+++"))).map((line) => line.slice(1));
  const index = hunk.oldStart - 1;
  if (index < 0 || oldLines.length === 0) throw new Error("Proposal diff hunk position is invalid.");
  if (!oldLines.every((line, offset) => lines[index + offset] === line)) {
    throw new Error("Current file content does not match proposal diff context.");
  }
  return [...lines.slice(0, index), ...newLines, ...lines.slice(index + oldLines.length)].join("\n");
}

function parseOldStart(header: string): number {
  const match = header.match(/^@@ -(\d+)(?:,\d+)? \+\d+(?:,\d+)? @@/);
  if (!match) throw new Error("Unified diff hunk header is unsupported.");
  return Number.parseInt(match[1], 10);
}

function hasContext(lines: string[]): boolean {
  return lines.some((line) => line.startsWith(" "));
}

function cleanDiffPath(value: string): string {
  const path = value.replace(/^"|"$/g, "").replace(/^[ab]\//, "");
  if (!path.endsWith(".md") || path.startsWith("/") || path.includes("..") || path.includes("://")) throw new Error("Diff target path is unsupported.");
  return path;
}

async function requireNode(canisterId: string, databaseId: string, path: string, identity: Identity): Promise<WikiNode> {
  const node = await readNode(canisterId, databaseId, path, identity);
  if (!node) throw new Error(`Node not found: ${path}`);
  return node;
}

function replaceRootFrontmatter(content: string, updates: Record<string, string>): string {
  if (!splitMarkdownFrontmatter(content)) throw new Error("Markdown frontmatter is missing.");
  const rest = content.slice(4);
  const end = rest.indexOf("\n---");
  const lines = rest.slice(0, end).split("\n");
  const pending = new Set(Object.keys(updates));
  const replaced = lines.map((line) => {
    const match = line.match(/^([^:\s][^:]*):(.*)$/);
    if (!match || !(match[1].trim() in updates)) return line;
    const key = match[1].trim();
    pending.delete(key);
    return `${key}: ${JSON.stringify(updates[key])}`;
  });
  for (const key of pending) replaced.push(`${key}: ${JSON.stringify(updates[key])}`);
  return `---\n${replaced.join("\n")}${rest.slice(end)}`;
}
