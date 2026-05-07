export function splitMarkdownPreviewSections(content: string): string[] {
  const sections: string[] = [];
  let current: string[] = [];
  let inFence = false;
  for (const line of content.split("\n")) {
    if (isFenceLine(line)) {
      inFence = !inFence;
    }
    if (!inFence && isPreviewSectionHeading(line) && current.length > 0) {
      sections.push(current.join("\n").trimEnd());
      current = [];
    }
    current.push(line);
  }
  if (current.length > 0) {
    sections.push(current.join("\n").trimEnd());
  }
  return sections.filter((section) => section.trim().length > 0);
}

function isPreviewSectionHeading(line: string): boolean {
  return line.startsWith("# ") || line.startsWith("## ");
}

function isFenceLine(line: string): boolean {
  return line.startsWith("```") || line.startsWith("~~~");
}
