# Query Rules

## Goal

Keep query answers evidence-backed, minimal, and consistent.

## Rules

- Read at least one note that directly supports the final answer.
- Do not answer from `index.md`, a list result, or a search hit alone.
- Do not conclude absence before you have read the highest-priority role-matched note for the current question.
- If the primary role-matched note is empty or lacks the value, inspect the next canonical note directly before broad search.
- Use `search-path-remote` and `search-remote` only after role-matched notes are still insufficient.
- Prefer reading the note whose role best matches the question shape before broad search.
- Treat `summary.md` as recap support for summary-style synthesis, not as the primary source for exact attribute extraction.
- Preserve exact value formatting for dates, times, places, person names, identifiers, and other explicit attribute values.
- Do not paraphrase, normalize, or complete an exact value when the wiki already states it directly.
- For exact-value or single-attribute extraction questions, answer with the value first and avoid explanation unless the question explicitly asks for it.
- When a note contains the requested value directly, stay on that span and do not drift into summary, background, or inferred context.
- Do not return `insufficient evidence` when the exact value is present in the note you read.
- Do not return `insufficient evidence` while a higher-priority canonical note remains unread.
- If a note you read already contains one requested slot, keep checking the remaining requested slots before concluding `insufficient evidence`.
- Do not normalize spelling variants such as `colour` -> `color` when the note already states the value.
- Do not convert ratios or fractions into decimals unless the note already uses the decimal form.
- If the question asks for a single turn, timestamp, or attribute value, prefer extraction over summarization.
- Return the smallest answer span that directly matches the evidence.
- For multi-value extraction, keep the answer aligned to the requested slots and preserve their order instead of replacing them with a generic recommendation.
- For paired-slot extraction such as `when and where` or `age and role`, answer every requested slot in one short response.
- Return only the requested attribute, not nearby qualifiers such as size, variant, or adjacent summary text.
- Use normal synthesis only for open-ended, comparative, or multi-fact explanation questions.
- For contradiction questions, if the notes contain unresolved conflict, explicitly state that there is contradictory information and ask for clarification instead of choosing one side.
- For temporal questions, extract the relevant time anchors before answering and compute the result from those anchors.
- For ordering questions, return the ordered items directly instead of replacing the order with a thematic summary.
- If the requested attribute or value is not directly supported by the notes you read, answer exactly `insufficient evidence`.
- For abstention questions, only an explicit statement in a note counts as evidence.
- For abstention questions, do not treat recap notes or cross-note synthesis as direct evidence for a missing relation or attribute.
- For abstention questions, if you only find adjacent context, implication, or summary-level overlap, answer exactly `insufficient evidence`.
- If the question is about order or time, do not answer from the index alone.
- Do not paraphrase quoted text or referenced turn content when the question asks for that exact item.

## Output

Prefer one of these outputs:

- a direct answer grounded in current wiki pages
- a comparison or synthesis summary
