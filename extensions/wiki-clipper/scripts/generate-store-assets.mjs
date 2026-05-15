// Where: extensions/wiki-clipper/scripts/generate-store-assets.mjs
// What: Render Chrome Web Store promotional and screenshot PNGs.
// Why: Store assets should match the Kinic-branded extension UI without entering the upload zip.
import { mkdir } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const playwrightPath = new URL("../../../wikibrowser/node_modules/@playwright/test/index.js", import.meta.url);
const playwright = await import(playwrightPath.href);
const chromium = playwright.chromium ?? playwright.default?.chromium;

await mkdir(resolve(root, "store-listing/assets"), { recursive: true });
await mkdir(resolve(root, "store-listing/screenshots"), { recursive: true });

const browser = await chromium.launch({ headless: true });
try {
  await renderAsset(promoHtml(), "store-listing/assets/promo-small-440x280.png", 440, 280);
  await renderAsset(optionsHtml(), "store-listing/screenshots/options-1280x800.png", 1280, 800);
  await renderAsset(chatGptHtml(), "store-listing/screenshots/chatgpt-export-1280x800.png", 1280, 800);
} finally {
  await browser.close();
}

console.log("generated Chrome Web Store assets");

async function renderAsset(html, outputPath, width, height) {
  const page = await browser.newPage({ viewport: { width, height }, deviceScaleFactor: 1 });
  await page.setContent(html, { waitUntil: "load" });
  await page.screenshot({ path: resolve(root, outputPath), fullPage: false });
  await page.close();
}

function shell(content, width, height, extraCss = "") {
  return `<!doctype html><html><head><meta charset="utf-8"><style>${baseCss()}body{width:${width}px;height:${height}px;overflow:hidden}${extraCss}</style></head><body>${content}</body></html>`;
}

function promoHtml() {
  return shell(
    `<div class="promo-wrap"><div class="mark">K</div><h1>Kinic Wiki<br>Clipper</h1><p>Save web pages and ChatGPT conversations into your Kinic Wiki database.</p><div class="chips"><span class="pink"></span><span class="amber"></span><span class="blue"></span><span class="orange"></span></div><div class="stripe"></div></div>`,
    440,
    280,
    `.promo-wrap{position:relative;width:440px;height:280px;padding:34px;background:linear-gradient(135deg,#fff 0%,#f8fafc 54%,#eef6ff 100%)}.promo-wrap h1{margin:20px 0 8px;font-size:32px;line-height:1.05;letter-spacing:0}.promo-wrap p{width:270px;margin:0;font-size:15px;line-height:1.45;color:#475569}.chips{display:flex;gap:8px;margin-top:22px}.chips span{height:10px;width:52px;border-radius:999px}.pink{background:#ec1877}.amber{background:#fcb12b}.blue{background:#23a8e0}.orange{background:#f05c21}.stripe{position:absolute;right:-32px;top:0;width:140px;height:280px;background:linear-gradient(135deg,#ec1877 0 24%,#fcb12b 24% 48%,#23a8e0 48% 72%,#f05c21 72%);clip-path:polygon(32% 0,100% 0,68% 50%,100% 100%,32% 100%,64% 50%)}`
  );
}

function optionsHtml() {
  return shell(
    `<div class="stage"><div class="top"><div class="brand"><div class="mark">K</div><div><h1>Kinic Wiki Clipper</h1><p class="muted">Authenticated destination settings</p></div></div><span class="pill">Internet Identity</span></div><section class="card panel"><div class="row"><div><strong>Internet Identity</strong><p class="principal muted">principal-2vxsx-fae...</p></div><span class="pill">Connected</span></div><div class="actions"><button class="primary">Login</button><button class="secondary">Logout</button></div><span class="label">Database</span><div class="field select"><span>db_d36yep4rv43e (Writer)</span><span class="muted">Hot</span></div><div class="actions"><button class="secondary">Refresh</button><button class="primary">Save settings</button></div><div class="latest"><strong>Latest URL ingest</strong><p class="muted">ok: /Sources/ingest-requests/example.md</p></div><div class="status">Databases loaded</div></section><div class="hero"><div class="stripe"></div></div></div>`,
    1280,
    800,
    `.stage{position:relative;height:800px;padding:72px 88px;background:linear-gradient(135deg,#fff 0%,#f8fafc 62%,#eef6ff 100%)}.top{display:flex;align-items:center;justify-content:space-between;margin-bottom:42px}.brand{display:flex;align-items:center;gap:18px}.brand h1{margin:0;font-size:36px;letter-spacing:0}.brand p{margin:5px 0 0;font-size:18px}.panel{width:520px;padding:22px}.row{display:flex;align-items:center;justify-content:space-between;gap:18px}.principal{font-family:ui-monospace,Menlo,monospace;font-size:15px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;max-width:300px}.actions{display:grid;grid-template-columns:1fr 1fr;gap:14px;margin-top:18px}.label{display:block;margin:26px 0 9px;font-size:15px;font-weight:800}.select{height:52px;display:flex;align-items:center;justify-content:space-between}.latest{margin-top:18px;padding:18px;border-radius:14px;background:#f8fafc;border:1px solid #e2e8f0}.status{margin-top:18px;color:#475569;font-weight:700}.hero{position:absolute;right:96px;bottom:80px;width:400px;height:300px}.stripe{position:absolute;right:0;top:0;width:170px;height:300px;background:linear-gradient(135deg,#ec1877 0 24%,#fcb12b 24% 48%,#23a8e0 48% 72%,#f05c21 72%);clip-path:polygon(35% 0,100% 0,70% 50%,100% 100%,35% 100%,65% 50%);opacity:.95}`
  );
}

function chatGptHtml() {
  return shell(
    `<div class="chat"><div class="chatline"></div><div class="chatline short"></div><div class="chatline"></div><div class="chatline short"></div></div><section class="panel"><div class="header"><div class="brand"><div class="mark">K</div><div><h1>Kinic Wiki Clipper</h1><p class="muted">Export ChatGPT conversations into your wiki</p></div></div><span class="pill">ChatGPT export</span></div><div class="body"><div class="settings"><div class="row"><strong>Database ID</strong><div class="field input">db_d36yep4rv43e</div></div><div class="export"><strong>Export recent chats</strong><p class="muted">Processing takes about 10 seconds per chat.</p><div class="control"><div class="field count">10</div><button class="primary">Export</button></div></div></div><div class="logs"><strong>Logs</strong><p class="muted">Export complete. Success 10.</p><div class="log"><span class="ok">K</span><div><strong>Saved conversation source</strong><div class="muted">ChatGPT</div></div></div></div></div></section><button class="fab"><span class="mark small">K</span> Kinic Wiki Clipper</button>`,
    1280,
    800,
    `body{background:#111827}.chat{position:absolute;inset:0;background:linear-gradient(180deg,#111827,#0f172a);padding:70px 100px;color:#e5e7eb}.chatline{width:680px;height:20px;border-radius:999px;background:#334155;margin:20px 0}.chatline.short{width:420px}.fab{position:absolute;right:72px;bottom:64px;display:flex;align-items:center;gap:10px;border-radius:999px;background:#162338;color:#fff;border:0;padding:13px 18px;font-weight:800;box-shadow:0 18px 40px rgb(0 0 0 / 35%)}.small{width:24px;height:24px;font-size:12px;border-radius:8px}.panel{position:absolute;right:72px;bottom:128px;width:660px;background:#fff;color:#0f172a;border:1px solid #e2e8f0;border-radius:18px;box-shadow:0 30px 80px rgb(0 0 0 / 35%);overflow:hidden}.header{display:flex;align-items:center;justify-content:space-between;padding:18px 22px;border-bottom:1px solid #e2e8f0}.brand{display:flex;align-items:center;gap:14px}.brand h1{margin:0;font-size:20px}.brand p{margin:3px 0 0;font-size:13px}.body{padding:18px}.settings{border:1px solid #e2e8f0;border-radius:14px;background:#f8fafc;padding:18px}.row{display:flex;align-items:center;justify-content:space-between}.input{width:270px;text-align:left}.export{margin-top:16px;background:#fff;border:1px solid #e2e8f0;border-radius:14px;padding:18px}.control{display:flex;justify-content:space-between;align-items:center;margin-top:14px}.count{width:68px;text-align:center}.logs{margin-top:14px;border:1px solid #e2e8f0;border-radius:14px;padding:16px}.log{display:flex;gap:12px;align-items:center;padding:12px;background:#f8fafc;border-radius:12px}.ok{width:38px;height:38px;border-radius:12px;background:#dcfce7;color:#166534;display:grid;place-items:center;font-weight:900}`
  );
}

function baseCss() {
  return `*{box-sizing:border-box}body{margin:0;font-family:Inter,ui-sans-serif,system-ui,-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif;color:#0f172a}.mark{display:grid;place-items:center;width:48px;height:48px;border-radius:14px;background:linear-gradient(135deg,#ec1877 0%,#ec1877 30%,#fcb12b 31%,#23a8e0 62%,#f05c21 100%);color:#fff;font-weight:900;font-size:22px;box-shadow:inset 0 0 0 1px rgb(255 255 255 / 32%)}.primary{background:#162338;color:#fff;border:1px solid #162338;border-radius:12px;padding:13px 18px;font-weight:800;box-shadow:0 14px 30px rgb(22 35 56 / 20%)}.secondary{background:#fff;color:#162338;border:1px solid #cbd5e1;border-radius:12px;padding:12px 16px;font-weight:750}.card{background:#fff;border:1px solid #e2e8f0;border-radius:18px;box-shadow:0 24px 60px rgb(15 23 42 / 12%)}.muted{color:#64748b}.pill{border:1px solid #e2e8f0;border-radius:999px;background:#f8fafc;color:#475569;padding:7px 12px;font-size:14px;font-weight:800}.field{border:1px solid #cbd5e1;border-radius:12px;background:#fff;padding:14px 16px;color:#0f172a}`;
}
