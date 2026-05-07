export function nodeRequestKey(canisterId: string, path: string): string {
  return `${canisterId}\n${path}`;
}

export function graphRequestKey(canisterId: string, centerPath: string | null, depth: 1 | 2): string | null {
  if (!centerPath) {
    return null;
  }
  return `${canisterId}\n${centerPath}\n${depth}`;
}
