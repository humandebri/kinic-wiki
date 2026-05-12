// Where: workers/wiki-generator/tests/openai.test.ts
// What: Generated draft response parsing tests.
// Why: The model boundary must stay schema-checked before rendering or writes.
import assert from "node:assert/strict";
import test from "node:test";
import { parseDraftResponse, parseDraftText, validateDraftSources } from "../src/openai.js";

const draftJson = JSON.stringify({
  title: "Project Notes",
  slug: "project-notes",
  summary: "Short summary",
  key_facts: [{ text: "Fact", source_path: "/Sources/raw/a/a.md" }],
  decisions: [],
  open_questions: [],
  follow_ups: []
});

test("Responses API output_text parses into a draft", () => {
  const draft = parseDraftResponse({ output_text: draftJson });
  assert.equal(draft.title, "Project Notes");
  validateDraftSources(draft, "/Sources/raw/a/a.md");
});

test("invalid draft schema is rejected", () => {
  assert.throws(() => parseDraftText('{"title":"Bad"}'), /schema/);
  const draft = parseDraftResponse({ output_text: draftJson });
  assert.throws(() => validateDraftSources(draft, "/Sources/raw/b/b.md"), /unsupported source/);
});
