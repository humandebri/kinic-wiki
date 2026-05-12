import type { Identity } from "@icp-sdk/core/agent";
import { splitMarkdownFrontmatter } from "@/lib/markdown-frontmatter";
import { parseSkillManifest, type SkillManifest } from "@/lib/skill-manifest";
import type { ChildNode } from "@/lib/types";
import { listChildren, readNode } from "@/lib/vfs-client";

const REGISTRY_ROOTS = [
  { label: "Team", path: "/Wiki/skills", catalog: "private" },
  { label: "Public", path: "/Wiki/public-skills", catalog: "public" }
] as const;

export type StatusFilter = "active" | "all" | "deprecated";

export type CatalogSkill = {
  catalog: "private" | "public";
  rootLabel: string;
  basePath: string;
  manifestPath: string;
  manifest: SkillManifest;
  missingFiles: string[];
  recentRuns: SkillRunEvidence[];
  proposals: SkillProposal[];
  runSummary: SkillRunSummary;
  trust: SkillRunSummary;
  events: SkillEvent[];
};

export type SkillRunEvidence = {
  path: string;
  outcome: string;
  task: string;
  agent: string;
  recordedAt: string;
};

export type SkillProposal = {
  path: string;
  id: string;
  title: string;
  status: string;
  createdAt: string;
  sourceRuns: string[];
  diff: string | null;
  appliedAt: string | null;
};

export type SkillEvent = {
  path: string;
  action: string;
  actor: string;
  recordedAt: string;
  targetPath: string;
  result: string;
};

export type SkillRunSummary = {
  runs: number;
  success: number;
  partial: number;
  fail: number;
  lastUsedAt: string | null;
  lastOutcome: string | null;
};

export type CatalogSummary = {
  total: number;
  promoted: number;
  reviewed: number;
  draft: number;
  deprecated: number;
};

export async function loadSkillCatalog(canisterId: string, databaseId: string, identity?: Identity): Promise<CatalogSkill[]> {
  const loaded: CatalogSkill[] = [];
  for (const root of REGISTRY_ROOTS) {
    const entries = await listRegistryChildren(canisterId, databaseId, root.path, identity);
    for (const entry of entries.filter((item) => item.kind === "directory")) {
      const manifestPath = `${entry.path}/manifest.md`;
      const node = await readNode(canisterId, databaseId, manifestPath, identity);
      const manifest = node ? parseSkillManifest(node.content) : null;
      if (!manifest) continue;
      const [children, runs, proposals, events] = await Promise.all([
        listRegistryChildren(canisterId, databaseId, entry.path, identity),
        loadRecentRuns(canisterId, databaseId, manifest.id, identity),
        loadProposals(canisterId, databaseId, entry.path, identity),
        loadEvents(canisterId, databaseId, manifest.id, identity)
      ]);
      const trust = summarizeRuns(runs);
      loaded.push({
        catalog: root.catalog,
        rootLabel: root.label,
        basePath: entry.path,
        manifestPath,
        manifest,
        missingFiles: missingPackageFiles(children),
        recentRuns: runs.slice(0, 5),
        proposals,
        runSummary: trust,
        trust,
        events
      });
    }
  }
  return loaded.sort(compareSkills);
}

async function loadRecentRuns(canisterId: string, databaseId: string, skillId: string, identity?: Identity): Promise<SkillRunEvidence[]> {
  const runDir = `/Sources/skill-runs/${skillId}`;
  const entries = await listRegistryChildren(canisterId, databaseId, runDir, identity);
  const nodes = await Promise.all(entries.filter((item) => item.kind !== "directory").slice(-100).map((item) => readNode(canisterId, databaseId, item.path, identity)));
  return nodes
    .flatMap((node) => (node ? [parseRunEvidence(node.path, node.content)] : []))
    .filter((run): run is SkillRunEvidence => Boolean(run))
    .sort((left, right) => right.recordedAt.localeCompare(left.recordedAt))
}

async function loadProposals(canisterId: string, databaseId: string, basePath: string, identity?: Identity): Promise<SkillProposal[]> {
  const entries = await listRegistryChildren(canisterId, databaseId, `${basePath}/improvement-proposals`, identity);
  const nodes = await Promise.all(entries.filter((item) => item.kind !== "directory").map((item) => readNode(canisterId, databaseId, item.path, identity)));
  return nodes
    .flatMap((node) => (node ? [parseProposal(node.path, node.content)] : []))
    .filter((proposal): proposal is SkillProposal => Boolean(proposal))
    .sort((left, right) => right.createdAt.localeCompare(left.createdAt));
}

async function loadEvents(canisterId: string, databaseId: string, skillId: string, identity?: Identity): Promise<SkillEvent[]> {
  const entries = await listRegistryChildren(canisterId, databaseId, `/Sources/skill-events/${skillId}`, identity);
  const nodes = await Promise.all(entries.filter((item) => item.kind !== "directory").slice(-20).map((item) => readNode(canisterId, databaseId, item.path, identity)));
  return nodes
    .flatMap((node) => (node ? [parseEvent(node.path, node.content)] : []))
    .filter((event): event is SkillEvent => Boolean(event))
    .sort((left, right) => right.recordedAt.localeCompare(left.recordedAt))
    .slice(0, 5);
}

export function filterSkills(skills: CatalogSkill[], query: string, statusFilter: StatusFilter): CatalogSkill[] {
  const normalized = query.trim().toLowerCase();
  return skills.filter((skill) => {
    const status = skill.manifest.status ?? "draft";
    if (statusFilter === "active" && status === "deprecated") return false;
    if (statusFilter === "deprecated" && status !== "deprecated") return false;
    if (!normalized) return true;
    return searchableSkillText(skill).includes(normalized);
  });
}

export function summarizeSkills(skills: CatalogSkill[]): CatalogSummary {
  const summary: CatalogSummary = { total: skills.length, promoted: 0, reviewed: 0, draft: 0, deprecated: 0 };
  for (const skill of skills) {
    const status = skill.manifest.status ?? "draft";
    if (status === "promoted") summary.promoted += 1;
    else if (status === "reviewed") summary.reviewed += 1;
    else if (status === "deprecated") summary.deprecated += 1;
    else summary.draft += 1;
  }
  return summary;
}

export function statusRank(status: string | null): number {
  if (status === "promoted") return 0;
  if (status === "reviewed") return 1;
  if (status === "draft" || !status) return 2;
  if (status === "deprecated") return 3;
  return 4;
}

async function listRegistryChildren(canisterId: string, databaseId: string, path: string, identity?: Identity): Promise<ChildNode[]> {
  try {
    return await listChildren(canisterId, databaseId, path, identity);
  } catch {
    return [];
  }
}

function missingPackageFiles(children: ChildNode[]): string[] {
  const names = new Set(children.filter((child) => child.kind !== "directory").map((child) => child.name));
  return ["manifest.md", "SKILL.md"].filter((name) => !names.has(name));
}

function summarizeRuns(runs: SkillRunEvidence[]): SkillRunSummary {
  const summary: SkillRunSummary = { runs: 0, success: 0, partial: 0, fail: 0, lastUsedAt: null, lastOutcome: null };
  for (const run of runs) {
    summary.runs += 1;
    if (run.outcome === "success") summary.success += 1;
    else if (run.outcome === "partial") summary.partial += 1;
    else if (run.outcome === "fail") summary.fail += 1;
    if (run.recordedAt && (!summary.lastUsedAt || run.recordedAt > summary.lastUsedAt)) {
      summary.lastUsedAt = run.recordedAt;
      summary.lastOutcome = run.outcome;
    }
  }
  return summary;
}

function parseRunEvidence(path: string, content: string): SkillRunEvidence | null {
  const fields = frontmatterFields(content);
  if (fields.kind !== "kinic.skill_run") return null;
  return {
    path,
    outcome: fields.outcome ?? "unknown",
    task: fields.task ?? "",
    agent: fields.agent ?? "",
    recordedAt: fields.recorded_at ?? ""
  };
}

function parseProposal(path: string, content: string): SkillProposal | null {
  const fields = frontmatterFields(content);
  if (fields.kind !== "kinic.skill_improvement_proposal") return null;
  return {
    path,
    id: fields.id ?? path.split("/").pop()?.replace(/\.md$/, "") ?? path,
    title: fields.title ?? fields.id ?? path,
    status: fields.status ?? "proposed",
    createdAt: fields.created_at ?? "",
    sourceRuns: sourceRuns(content),
    diff: proposalDiff(content),
    appliedAt: fields.applied_at ?? null
  };
}

function parseEvent(path: string, content: string): SkillEvent | null {
  const fields = frontmatterFields(content);
  if (fields.kind !== "kinic.skill_event") return null;
  return {
    path,
    action: fields.action ?? "",
    actor: fields.actor ?? "",
    recordedAt: fields.recorded_at ?? "",
    targetPath: fields.target_path ?? "",
    result: fields.result ?? ""
  };
}

function frontmatterFields(content: string): Record<string, string> {
  return Object.fromEntries(splitMarkdownFrontmatter(content)?.fields.map((field) => [field.key, field.value]) ?? []);
}

function sourceRuns(content: string): string[] {
  const end = content.startsWith("---\n") ? content.indexOf("\n---", 4) : -1;
  const frontmatter = end >= 0 ? content.slice(4, end) : "";
  const lines = frontmatter.split("\n");
  const start = lines.findIndex((line) => line.trim() === "source_runs:");
  if (start < 0) return [];
  return lines.slice(start + 1).filter((line) => line.startsWith("  - ")).map((line) => line.slice(4).trim());
}

function proposalDiff(content: string): string | null {
  const start = content.indexOf("```diff");
  if (start < 0) return null;
  const bodyStart = content.indexOf("\n", start);
  const end = content.indexOf("\n```", bodyStart);
  return bodyStart >= 0 && end >= 0 ? content.slice(bodyStart + 1, end) : null;
}

function compareSkills(left: CatalogSkill, right: CatalogSkill): number {
  return statusRank(left.manifest.status) - statusRank(right.manifest.status) || left.manifest.id.localeCompare(right.manifest.id);
}

function searchableSkillText(skill: CatalogSkill): string {
  const manifest = skill.manifest;
  return [
    skill.catalog,
    skill.basePath,
    manifest.id,
    manifest.title,
    manifest.summary,
    manifest.status,
    manifest.version,
    ...manifest.tags,
    ...manifest.useCases,
    ...manifest.knowledge,
    ...manifest.related,
    ...Object.values(manifest.provenance)
  ]
    .filter(Boolean)
    .join("\n")
    .toLowerCase();
}
