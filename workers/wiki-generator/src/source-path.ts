// Where: workers/wiki-generator/src/source-path.ts
// What: Canonical raw source path validation.
// Why: The worker must mirror canister source path rules before queueing work.
export function validateCanonicalSourcePath(path: string, prefix: string): void {
  const boundary = `${prefix}/`;
  if (!path.startsWith(boundary)) {
    throw new Error(`sourcePath must be under ${prefix}`);
  }
  const parts = path.slice(boundary.length).split("/");
  if (parts.length !== 2 || !parts[0] || parts[1] !== `${parts[0]}.md`) {
    throw new Error(`sourcePath must use ${prefix}/<id>/<id>.md`);
  }
}

export function sourceIdFromPath(path: string, prefix: string): string {
  validateCanonicalSourcePath(path, prefix);
  return path.slice(`${prefix}/`.length).split("/")[0] ?? "";
}
