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
  return `/${encodeURIComponent(canisterId)}/${suffix}${queryString}`;
}

export function hrefForSearch(canisterId: string, searchQuery: string, searchKind: string): string {
  const params = new URLSearchParams();
  if (searchQuery) {
    params.set("q", searchQuery);
  }
  if (searchKind) {
    params.set("kind", searchKind);
  }
  const queryString = params.size > 0 ? `?${params.toString()}` : "";
  return `/${encodeURIComponent(canisterId)}/search${queryString}`;
}

export function hrefForMarkdownLink(canisterId: string, currentPath: string, href: string | undefined): string | null {
  if (!href) {
    return null;
  }
  const trimmed = href.trim();
  if (!trimmed || isExternalHref(trimmed) || trimmed.startsWith("#")) {
    return null;
  }
  const target = splitMarkdownHref(trimmed);
  if (trimmed.startsWith("/Wiki") || trimmed.startsWith("/Sources")) {
    return `${hrefForPath(canisterId, target.path)}${target.suffix}`;
  }
  if (trimmed.startsWith("/")) {
    return null;
  }
  return `${hrefForPath(canisterId, resolveRelativeWikiPath(currentPath, target.path))}${target.suffix}`;
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

function resolveRelativeWikiPath(currentPath: string, href: string): string {
  const base = parentPath(currentPath) ?? "/Wiki";
  const parts = [...base.split("/"), ...href.split("/")].filter(Boolean);
  const resolved: string[] = [];
  for (const part of parts) {
    if (part === ".") {
      continue;
    }
    if (part === "..") {
      resolved.pop();
      continue;
    }
    resolved.push(part);
  }
  return `/${resolved.join("/")}`;
}

function isExternalHref(href: string): boolean {
  return /^[a-z][a-z0-9+.-]*:/i.test(href) || href.startsWith("//");
}

function splitMarkdownHref(href: string): { path: string; suffix: string } {
  const hashIndex = href.indexOf("#");
  const pathAndQuery = hashIndex === -1 ? href : href.slice(0, hashIndex);
  const hash = hashIndex === -1 ? "" : href.slice(hashIndex);
  const queryIndex = pathAndQuery.indexOf("?");
  if (queryIndex === -1) {
    return { path: pathAndQuery, suffix: hash };
  }
  return {
    path: pathAndQuery.slice(0, queryIndex),
    suffix: `${pathAndQuery.slice(queryIndex)}${hash}`
  };
}
