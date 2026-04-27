export function pathFromSegments(segments: string[]): string {
  if (segments.length === 0) {
    return "/Wiki";
  }
  return `/${segments.join("/")}`;
}

export function hrefForPath(canisterId: string, path: string, view?: string): string {
  const normalized = path.startsWith("/") ? path.slice(1) : path;
  const suffix = normalized
    .split("/")
    .filter(Boolean)
    .map(encodeURIComponent)
    .join("/");
  const query = view === "raw" ? "?view=raw" : "";
  return `/site/${encodeURIComponent(canisterId)}/${suffix}${query}`;
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
