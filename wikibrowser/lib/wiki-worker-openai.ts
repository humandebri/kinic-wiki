// Where: wikibrowser/lib/wiki-worker-openai.ts
// What: OpenAI Responses API call and generated draft validation.
// Why: API parsing stays isolated from VFS write and route concerns.
import type { SearchNodeHit, WikiNode } from "@/lib/types";
import type { WikiDraft, WikiDraftItem, WikiWorkerGenerationConfig } from "@/lib/wiki-worker-types";

type OpenAITextContent = {
  type?: string;
  text?: string;
};

type OpenAIOutputItem = {
  content?: OpenAITextContent[];
};

type OpenAIResponse = {
  output_text?: string;
  output?: OpenAIOutputItem[];
};

export async function generateDraft(source: WikiNode, contextHits: SearchNodeHit[], config: WikiWorkerGenerationConfig): Promise<WikiDraft> {
  const apiKey = process.env.OPENAI_API_KEY;
  if (!apiKey) {
    throw new Error("OPENAI_API_KEY is required");
  }
  const response = await fetch("https://api.openai.com/v1/responses", {
    method: "POST",
    headers: {
      Authorization: `Bearer ${apiKey}`,
      "Content-Type": "application/json"
    },
    body: JSON.stringify({
      model: config.model,
      reasoning: { effort: config.reasoningEffort },
      max_output_tokens: config.maxOutputTokens,
      text: {
        format: {
          type: "json_schema",
          name: "wiki_draft",
          strict: true,
          schema: wikiDraftSchema()
        }
      },
      input: [
        {
          role: "system",
          content:
            "Generate one review-ready wiki draft from raw source material. Keep raw transcript out of wiki content. Every item must cite the provided source_path. Do not canonicalize decisions."
        },
        {
          role: "user",
          content: JSON.stringify({
            source_path: source.path,
            raw_content: source.content.slice(0, config.maxRawChars),
            context: contextHits.map((hit) => ({
              path: hit.path,
              preview: hit.preview?.excerpt ?? hit.snippet ?? ""
            }))
          })
        }
      ]
    })
  });
  const body = await response.json();
  if (!response.ok) {
    throw new Error(openAIErrorMessage(body));
  }
  return parseDraft(extractResponseText(body));
}

function wikiDraftSchema(): object {
  const item = {
    type: "object",
    additionalProperties: false,
    required: ["text", "source_path"],
    properties: {
      text: { type: "string" },
      source_path: { type: "string" }
    }
  };
  return {
    type: "object",
    additionalProperties: false,
    required: ["title", "slug", "summary", "key_facts", "decisions", "open_questions", "follow_ups"],
    properties: {
      title: { type: "string" },
      slug: { type: "string" },
      summary: { type: "string" },
      key_facts: { type: "array", items: item },
      decisions: { type: "array", items: item },
      open_questions: { type: "array", items: item },
      follow_ups: { type: "array", items: item }
    }
  };
}

function openAIErrorMessage(body: unknown): string {
  if (isObject(body)) {
    const error = body.error;
    if (isObject(error) && typeof error.message === "string") {
      return error.message;
    }
  }
  return "OpenAI request failed";
}

function extractResponseText(body: unknown): string {
  if (!isOpenAIResponse(body)) {
    throw new Error("OpenAI response shape is invalid");
  }
  if (body.output_text) {
    return body.output_text;
  }
  for (const item of body.output ?? []) {
    for (const content of item.content ?? []) {
      if (typeof content.text === "string" && content.text) {
        return content.text;
      }
    }
  }
  throw new Error("OpenAI response did not include text");
}

function parseDraft(text: string): WikiDraft {
  const parsed = JSON.parse(text);
  if (!isWikiDraft(parsed)) {
    throw new Error("generated wiki draft does not match schema");
  }
  return parsed;
}

function isOpenAIResponse(value: unknown): value is OpenAIResponse {
  if (!isObject(value)) return false;
  if ("output_text" in value && typeof value.output_text !== "string") return false;
  if (!("output" in value) || value.output === undefined) return true;
  if (!Array.isArray(value.output)) return false;
  return value.output.every((item) => {
    if (!isObject(item)) return false;
    if (!("content" in item) || item.content === undefined) return true;
    return Array.isArray(item.content) && item.content.every(isOpenAITextContent);
  });
}

function isOpenAITextContent(value: unknown): value is OpenAITextContent {
  return isObject(value) && (!("text" in value) || value.text === undefined || typeof value.text === "string");
}

function isWikiDraft(value: unknown): value is WikiDraft {
  if (!isObject(value)) return false;
  return (
    typeof value.title === "string" &&
    typeof value.slug === "string" &&
    typeof value.summary === "string" &&
    isDraftItemArray(value.key_facts) &&
    isDraftItemArray(value.decisions) &&
    isDraftItemArray(value.open_questions) &&
    isDraftItemArray(value.follow_ups)
  );
}

function isDraftItemArray(value: unknown): value is WikiDraftItem[] {
  return Array.isArray(value) && value.every(isDraftItem);
}

function isDraftItem(value: unknown): value is WikiDraftItem {
  return isObject(value) && typeof value.text === "string" && typeof value.source_path === "string";
}

function isObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}
