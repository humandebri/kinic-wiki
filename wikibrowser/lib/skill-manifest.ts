export type SkillManifest = {
  kind: string;
  schemaVersion: string;
  id: string;
  version: string;
  publisher: string;
  entry: string;
  knowledge: string[];
  permissions: Record<string, string>;
  provenance: Record<string, string>;
};

// Browser rendering intentionally supports only the v1 frontmatter subset used by Skill Registry.
// Rust CLI owns full YAML parsing and validation.
export function parseSkillManifest(content: string): SkillManifest | null {
  const frontmatter = extractFrontmatter(content);
  if (!frontmatter) return null;
  const values = parseFrontmatter(frontmatter);
  if (scalar(values, "kind") !== "kinic.skill") return null;
  if (scalar(values, "schema_version") !== "1") return null;
  const id = scalar(values, "id");
  const version = scalar(values, "version");
  const publisher = scalar(values, "publisher");
  const entry = scalar(values, "entry");
  if (!id || !version || !publisher || !entry) return null;
  return {
    kind: "kinic.skill",
    schemaVersion: "1",
    id,
    version,
    publisher,
    entry,
    knowledge: values.knowledge ?? [],
    permissions: nested(values, "permissions"),
    provenance: nested(values, "provenance")
  };
}

export function isSkillRegistryPath(path: string): boolean {
  return path === "/Wiki/skills" || path.startsWith("/Wiki/skills/");
}

export function manifestPathForSkillRegistryFile(path: string): string | null {
  if (!isSkillRegistryPath(path) || path.endsWith("/manifest.md")) return null;
  for (const file of ["/SKILL.md", "/provenance.md", "/evals.md"]) {
    if (path.endsWith(file)) {
      return `${path.slice(0, -file.length)}/manifest.md`;
    }
  }
  return null;
}

function extractFrontmatter(content: string): string | null {
  if (!content.startsWith("---\n")) return null;
  const rest = content.slice(4);
  const end = rest.indexOf("\n---");
  return end >= 0 ? rest.slice(0, end) : null;
}

function parseFrontmatter(frontmatter: string): Record<string, string[]> {
  const root: Record<string, string[]> = {};
  let current: string | null = null;
  for (const line of frontmatter.split("\n")) {
    if (!line.trim()) continue;
    const item = line.trimStart().match(/^-\s+(.+)$/);
    if (item && current) {
      root[current] = [...(root[current] ?? []), cleanValue(item[1])];
      continue;
    }
    if (line.startsWith("  ") && current) {
      const nestedMatch = line.trim().match(/^([^:]+):(.*)$/);
      if (nestedMatch) {
        root[`${current}.${nestedMatch[1].trim()}`] = [cleanValue(nestedMatch[2])];
      }
      continue;
    }
    const match = line.match(/^([^:]+):(.*)$/);
    if (match) {
      current = match[1].trim();
      const value = cleanValue(match[2]);
      root[current] = value ? [value] : [];
    }
  }
  return root;
}

function scalar(values: Record<string, string[]>, key: string): string | null {
  return values[key]?.[0] ?? null;
}

function nested(values: Record<string, string[]>, parent: string): Record<string, string> {
  const result: Record<string, string> = {};
  for (const [key, value] of Object.entries(values)) {
    if (key.startsWith(`${parent}.`)) {
      result[key.slice(parent.length + 1)] = value[0] ?? "";
    }
  }
  return result;
}

function cleanValue(value: string): string {
  const trimmed = value.trim();
  return trimmed.startsWith("\"") && trimmed.endsWith("\"") ? trimmed.slice(1, -1) : trimmed;
}
