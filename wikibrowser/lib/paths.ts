export function pathFromSegments(segments: string[]): string {
  if (segments.length === 0) {
    return "/Wiki";
  }
  return `/${segments.join("/")}`;
}

export function hrefForPath(
  canisterId: string,
  path: string,
  view?: string,
  tab?: string,
  searchQuery?: string,
  searchKind?: string
): string {
  const normalized = path.startsWith("/") ? path.slice(1) : path;
  const suffix = normalized
    .split("/")
    .filter(Boolean)
    .map(encodeURIComponent)
    .join("/");
  const params = new URLSearchParams();
  if (view === "raw") {
    params.set("view", "raw");
  }
  if (tab) {
    params.set("tab", tab);
  }
  if (searchQuery) {
    params.set("q", searchQuery);
  }
  if (searchKind) {
    params.set("kind", searchKind);
  }
  const queryString = params.size > 0 ? `?${params.toString()}` : "";
  return `/site/${encodeURIComponent(canisterId)}/${suffix}${queryString}`;
}

export function parentPath(path: string): string | null {
  if (path === "/") {
    return null;
  }
  const index = path.lastIndexOf("/");
  if (index <= 0) {
    return "/";
  }
  return path.slice(0, index);
}
