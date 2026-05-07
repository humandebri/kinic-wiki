// Where: wikibrowser/scripts/check-frontend-aeo-skill.mjs
// What: Static contract checks for the frontend-aeo-wiki skill.
// Why: AEO generation must stay scoped to visible frontend sources.

import fs from "node:fs";
import path from "node:path";

const root = path.resolve(process.cwd(), "..");
const skillPath = path.join(root, ".agents/skills/frontend-aeo-wiki/SKILL.md");
const source = fs.readFileSync(skillPath, "utf8");
const failures = [];

for (const required of [
  "Next.js App Router",
  "user-visible product behavior",
  "README",
  "public docs",
  "backend-only code",
  "secrets",
  "hidden admin surfaces",
  "sources",
  "slugs are unique",
  "required frontmatter"
]) {
  if (!source.includes(required)) {
    failures.push(`frontend-aeo-wiki skill is missing: ${required}`);
  }
}

if (failures.length > 0) {
  console.error(failures.join("\n"));
  process.exit(1);
}

console.log("Frontend AEO skill checks passed");
