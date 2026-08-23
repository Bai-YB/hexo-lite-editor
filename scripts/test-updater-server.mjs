import { createServer } from "node:http";
import { readFile } from "node:fs/promises";
import { join } from "node:path";
const root = join(process.cwd(), "tests", "fixtures", "updater");
const port = Number(process.env.HLEX_UPDATER_TEST_PORT || 4876);
createServer(async (request, response) => {
  const name = request.url === "/update.bin" ? "mock-update.bin" : request.url?.slice(1) || "latest-newer.json";
  if (!/^(?:latest-(?:newer|same|invalid)\.json|mock-update\.bin)$/.test(name)) { response.writeHead(404).end(); return; }
  try { const data = await readFile(join(root, name)); response.setHeader("Content-Type", name.endsWith(".json") ? "application/json" : "application/octet-stream"); response.setHeader("Content-Length", data.length); response.end(data); } catch { response.writeHead(404).end(); }
}).listen(port, "127.0.0.1", () => console.log(`Updater fixture server: http://127.0.0.1:${port}`));
