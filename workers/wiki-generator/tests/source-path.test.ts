// Where: workers/wiki-generator/tests/source-path.test.ts
// What: Raw source path validation tests.
// Why: Queueing invalid source paths should fail before VFS writes.
import assert from "node:assert/strict";
import test from "node:test";
import { sourceIdFromPath, validateCanonicalSourcePath } from "../src/source-path.js";

test("canonical raw source path is accepted", () => {
  assert.doesNotThrow(() => validateCanonicalSourcePath("/Sources/raw/alpha/alpha.md", "/Sources/raw"));
  assert.equal(sourceIdFromPath("/Sources/raw/alpha/alpha.md", "/Sources/raw"), "alpha");
});

test("non-canonical raw source paths are rejected", () => {
  assert.throws(() => validateCanonicalSourcePath("/Sources/raw/alpha/beta.md", "/Sources/raw"), /<id>\/<id>\.md/);
  assert.throws(() => validateCanonicalSourcePath("/Sources/rawfoo/alpha/alpha.md", "/Sources/raw"), /under/);
});
