// Where: wikibrowser/lib/aeo/parse-markdown.ts
// What: Parse the limited AEO frontmatter contract and Markdown body.
// Why: MVP avoids a new YAML dependency while enforcing indexed page quality.

export type AeoFrontmatter = {
  title: string;
  description: string;
  answerSummary: string;
  updated: string;
  index: true;
  canonical: string | null;
  entities: string[];
  sources: string[];
};

export type ParsedAeoMarkdown = {
  frontmatter: AeoFrontmatter;
  markdown: string;
};

type RawFrontmatter = {
  title?: string;
  description?: string;
  answer_summary?: string;
  updated?: string;
  index?: string;
  canonical?: string;
  entities?: string[];
  sources?: string[];
};

export function parseAeoMarkdown(content: string): ParsedAeoMarkdown | null {
  if (!content.startsWith("---\n")) {
    return null;
  }
  const endIndex = content.indexOf("\n---\n", 4);
  if (endIndex === -1) {
    return null;
  }
  const raw = parseFrontmatterBlock(content.slice(4, endIndex));
  const frontmatter = normalizeFrontmatter(raw);
  if (!frontmatter) {
    return null;
  }
  return {
    frontmatter,
    markdown: content.slice(endIndex + "\n---\n".length).trim()
  };
}

function parseFrontmatterBlock(block: string): RawFrontmatter {
  const raw: RawFrontmatter = {};
  let listKey: "entities" | "sources" | null = null;
  for (const line of block.split("\n")) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("#")) {
      continue;
    }
    if (listKey && trimmed.startsWith("- ")) {
      raw[listKey] = [...(raw[listKey] ?? []), unquote(trimmed.slice(2).trim())];
      continue;
    }
    listKey = null;
    const separator = trimmed.indexOf(":");
    if (separator === -1) {
      continue;
    }
    const key = trimmed.slice(0, separator).trim();
    const value = trimmed.slice(separator + 1).trim();
    if ((key === "entities" || key === "sources") && value === "") {
      raw[key] = [];
      listKey = key;
      continue;
    }
    assignScalar(raw, key, unquote(value));
  }
  return raw;
}

function assignScalar(raw: RawFrontmatter, key: string, value: string): void {
  if (key === "title") {
    raw.title = value;
  } else if (key === "description") {
    raw.description = value;
  } else if (key === "answer_summary") {
    raw.answer_summary = value;
  } else if (key === "updated") {
    raw.updated = value;
  } else if (key === "index") {
    raw.index = value;
  } else if (key === "canonical") {
    raw.canonical = value;
  }
}

function normalizeFrontmatter(raw: RawFrontmatter): AeoFrontmatter | null {
  if (!raw.title || !raw.description || !raw.answer_summary || !raw.updated) {
    return null;
  }
  if (raw.index !== "true") {
    return null;
  }
  if (!raw.sources || raw.sources.length === 0) {
    return null;
  }
  return {
    title: raw.title,
    description: raw.description,
    answerSummary: raw.answer_summary,
    updated: raw.updated,
    index: true,
    canonical: raw.canonical ?? null,
    entities: raw.entities ?? [],
    sources: raw.sources ?? []
  };
}

function unquote(value: string): string {
  if (value.length >= 2 && value.startsWith("\"") && value.endsWith("\"")) {
    return value.slice(1, -1);
  }
  if (value.length >= 2 && value.startsWith("'") && value.endsWith("'")) {
    return value.slice(1, -1);
  }
  return value;
}
