// Where: wikibrowser/lib/wiki-worker-types.ts
// What: Shared worker request, response, and generated draft types.
// Why: The route, OpenAI client, renderer, and orchestrator need one contract.
export const SCHEMA_VERSION = 1;

export type WikiWorkerRunInput = {
  canisterId?: string;
  databaseId: string;
  sourcePath: string;
  dryRun?: boolean;
};

export type WikiWorkerRunResult = {
  sourcePath: string;
  targetPath: string;
  dryRun: boolean;
  wrote: boolean;
  content: string;
  contextPaths: string[];
};

export type WikiWorkerGenerationConfig = {
  model: string;
  reasoningEffort: string;
  maxRawChars: number;
  maxOutputTokens: number;
};

export type WikiDraftItem = {
  text: string;
  source_path: string;
};

export type WikiDraft = {
  title: string;
  slug: string;
  summary: string;
  key_facts: WikiDraftItem[];
  decisions: WikiDraftItem[];
  open_questions: WikiDraftItem[];
  follow_ups: WikiDraftItem[];
};
