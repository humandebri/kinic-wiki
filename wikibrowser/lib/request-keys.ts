export function nodeRequestKey(canisterId: string, databaseId: string, path: string): string {
  return `${canisterId}\n${databaseId}\n${path}`;
}

export function graphRequestKey(canisterId: string, databaseId: string, centerPath: string | null, depth: 1 | 2): string | null {
  if (!centerPath) {
    return null;
  }
  return `${canisterId}\n${databaseId}\n${centerPath}\n${depth}`;
}

export function searchRequestKey(canisterId: string, databaseId: string, searchKind: string, searchText: string): string {
  return `${canisterId}\n${databaseId}\n${searchKind}\n${searchText}`;
}
