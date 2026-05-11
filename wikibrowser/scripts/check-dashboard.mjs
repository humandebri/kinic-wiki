import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";

const dashboardClient = readFileSync(new URL("../app/dashboard/dashboard-client.tsx", import.meta.url), "utf8");
const dashboardIndex = readFileSync(new URL("../app/dashboard/page.tsx", import.meta.url), "utf8");
const dashboardRoute = readFileSync(new URL("../app/dashboard/[databaseId]/page.tsx", import.meta.url), "utf8");
const dashboardUi = readFileSync(new URL("../app/dashboard/dashboard-ui.tsx", import.meta.url), "utf8");
const homePage = readFileSync(new URL("../app/page.tsx", import.meta.url), "utf8");
const nextConfig = readFileSync(new URL("../next.config.ts", import.meta.url), "utf8");
const packageJson = JSON.parse(readFileSync(new URL("../package.json", import.meta.url), "utf8"));
const wikiRoute = readFileSync(new URL("../app/[databaseId]/[[...segments]]/page.tsx", import.meta.url), "utf8");
const wranglerConfig = readFileSync(new URL("../wrangler.jsonc", import.meta.url), "utf8");

assert.match(homePage, /href=\{`\/dashboard\/\$\{encodeURIComponent\(database\.databaseId\)\}`\}/);
assert.match(dashboardIndex, /<DashboardDatabaseClient databaseId="" \/>/);
assert.match(dashboardRoute, /params: Promise<\{ databaseId: string \}>/);
assert.match(dashboardRoute, /<DashboardDatabaseClient databaseId=\{databaseId\} \/>/);
assert.match(dashboardClient, /export function DashboardDatabaseClient\(\{ databaseId \}/);
assert.doesNotMatch(dashboardClient, /useSearchParams/);
assert.doesNotMatch(dashboardClient, /usePathname/);

assert.match(wikiRoute, /<WikiBrowser \/>/);
assert.equal(existsSync(new URL("../app/w/page.tsx", import.meta.url)), false);
assert.equal(existsSync(new URL("../vercel.json", import.meta.url)), false);
assert.doesNotMatch(nextConfig, /output:\s*"export"/);

assert.match(wranglerConfig, /"name": "kinic-wiki-browser"/);
assert.match(wranglerConfig, /"main": ".open-next\/worker.js"/);
assert.match(wranglerConfig, /"nodejs_compat"/);
assert.match(wranglerConfig, /"global_fetch_strictly_public"/);
assert.match(wranglerConfig, /"WORKER_SELF_REFERENCE"/);

assert.equal(packageJson.scripts.preview, "opennextjs-cloudflare build && opennextjs-cloudflare preview");
assert.equal(packageJson.scripts["build:worker"], "opennextjs-cloudflare build");
assert.equal(packageJson.scripts.deploy, "opennextjs-cloudflare build && opennextjs-cloudflare deploy");
assert.equal(packageJson.scripts["cf-typegen"], "wrangler types --env-interface CloudflareEnv cloudflare-env.d.ts");
assert.equal(packageJson.scripts["e2e:ii"], "scripts/run-ii-e2e.sh");
assert.equal(packageJson.scripts["e2e:ii:headed"], "scripts/run-ii-e2e.sh --headed");
assert.equal(packageJson.scripts["e2e:ii:setup"], "../scripts/setup-wikibrowser-ii-e2e.sh");
assert.match(nextConfig, /NEXT_PUBLIC_KINIC_WIKI_CANISTER_ID/);
assert.match(nextConfig, /NEXT_PUBLIC_II_PROVIDER_URL/);
assert.match(homePage, /NEXT_PUBLIC_KINIC_WIKI_CANISTER_ID/);
assert.match(dashboardClient, /NEXT_PUBLIC_KINIC_WIKI_CANISTER_ID/);
assert.doesNotMatch(homePage, /process\.env\.KINIC_WIKI_CANISTER_ID/);
assert.doesNotMatch(dashboardClient, /process\.env\.KINIC_WIKI_CANISTER_ID/);

assert.match(dashboardUi, /type PendingAclAction/);
assert.match(dashboardUi, /Enable public access/);
assert.match(dashboardUi, /Disable public access/);
assert.match(dashboardUi, /Grant owner access/);
assert.match(dashboardUi, /Revoke owner access/);
assert.match(dashboardUi, /ConfirmAclDialog/);
assert.match(dashboardUi, /This will grant \$\{role\} access to principal/);

assert.match(homePage, /refreshSeqRef/);
assert.match(homePage, /isCurrentRefresh/);
assert.match(dashboardClient, /refreshSeqRef/);
assert.match(dashboardClient, /isCurrentRefresh/);
assert.match(homePage, /refreshSeqRef\.current \+= 1;\n    await authClient\.logout/);
assert.match(dashboardClient, /refreshSeqRef\.current \+= 1;\n    await authClient\.logout/);

console.log("Dashboard checks OK");
