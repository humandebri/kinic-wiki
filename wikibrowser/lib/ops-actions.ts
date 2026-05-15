export type OpsIdentityMode = "anonymous" | "user";

export type OpsAction =
  | { kind: "recent"; targetPath: null; sideEffect: "none"; identityMode: OpsIdentityMode }
  | { kind: "lint"; targetPath: string; sideEffect: "none"; identityMode: OpsIdentityMode }
  | { kind: "queue_url"; targetPath: "/Sources/ingest-requests"; sideEffect: "queue request"; identityMode: "user"; url: string }
  | { kind: "search"; targetPath: "/Wiki"; sideEffect: "none"; identityMode: OpsIdentityMode; query: string }
  | { kind: "ask"; targetPath: "/Wiki"; sideEffect: "none"; identityMode: OpsIdentityMode; question: string };

export function classifyOpsInput(value: string, selectedPath: string, identityMode: OpsIdentityMode): OpsAction | null {
  const text = value.trim();
  if (!text) return null;
  const url = firstHttpUrl(text);
  if (url) return { kind: "queue_url", targetPath: "/Sources/ingest-requests", sideEffect: "queue request", identityMode: "user", url };
  if (/(^|\s)(recent|最近)(\s|$)/i.test(text)) return { kind: "recent", targetPath: null, sideEffect: "none", identityMode };
  if (/(lint|点検|検査)/i.test(text)) {
    return { kind: "lint", targetPath: /facts\.md|facts|事実/i.test(text) ? "/Wiki/facts.md" : selectedPath, sideEffect: "none", identityMode };
  }
  if (/(search|検索)/i.test(text)) {
    const query = text.replace(/(search|検索)/gi, "").trim();
    return query ? { kind: "search", targetPath: "/Wiki", sideEffect: "none", identityMode, query } : null;
  }
  return { kind: "ask", targetPath: "/Wiki", sideEffect: "none", identityMode, question: text };
}

function firstHttpUrl(value: string): string | null {
  const match = value.match(/https?:\/\/[^\s]+/i);
  return match?.[0] ?? null;
}
