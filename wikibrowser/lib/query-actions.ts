export type QueryIdentityMode = "anonymous" | "user";

export type QueryAction =
  | { kind: "recent"; targetPath: null; sideEffect: "none"; identityMode: QueryIdentityMode }
  | { kind: "lint"; targetPath: string; sideEffect: "none"; identityMode: QueryIdentityMode }
  | { kind: "queue_url"; targetPath: "/Sources/ingest-requests"; sideEffect: "queue request"; identityMode: "user"; url: string }
  | { kind: "ask"; targetPath: "/Wiki"; sideEffect: "none"; identityMode: QueryIdentityMode; question: string };

export function classifyQueryInput(value: string, selectedPath: string, identityMode: QueryIdentityMode): QueryAction | null {
  const text = value.trim();
  if (!text) return null;
  const url = firstHttpUrl(text);
  if (url) return { kind: "queue_url", targetPath: "/Sources/ingest-requests", sideEffect: "queue request", identityMode: "user", url };
  if (/(^|\s)(recent|最近)(\s|$)/i.test(text)) return { kind: "recent", targetPath: null, sideEffect: "none", identityMode };
  if (/(lint|点検|検査)/i.test(text)) {
    return { kind: "lint", targetPath: /facts\.md|facts|事実/i.test(text) ? "/Wiki/facts.md" : selectedPath, sideEffect: "none", identityMode };
  }
  return { kind: "ask", targetPath: "/Wiki", sideEffect: "none", identityMode, question: text };
}

function firstHttpUrl(value: string): string | null {
  const match = value.match(/https?:\/\/[^\s]+/i);
  return match?.[0] ?? null;
}
