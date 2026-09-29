import { execSync } from "node:child_process";
import { readdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const inventory = JSON.parse(execSync("pnpm licenses list --prod --json", {
  cwd: root,
  encoding: "utf8",
  maxBuffer: 8 * 1024 * 1024
}));
const packages = Object.values(inventory).flat().sort((a, b) => a.name.localeCompare(b.name));
const missing = [];
const sections = [
  "Hexo Lite Editor — production JavaScript dependency notices",
  "Generated from the production pnpm dependency tree. Rust crate identities are in Cargo.lock; check each crate's source package for its license text.",
  ""
];

for (const entry of packages) {
  const packageRoot = entry.paths?.[0];
  const files = packageRoot && readdirSync(packageRoot);
  const licenses = files?.filter((name) => /^(licen[cs]e|copying)([._-].*)?$/i.test(name)) ?? [];
  const fileName = licenses.find((name) => /^(license|licence)[._-]mit/i.test(name))
    ?? licenses.find((name) => /^(license|licence)(\.[^/]*)?$/i.test(name))
    ?? licenses[0];
  if (!fileName && !["is-reference", "locate-character"].includes(entry.name)) {
    missing.push(`${entry.name}@${entry.versions?.join(",") ?? "?"}`);
    continue;
  }
  const notice = fileName
    ? readFileSync(join(packageRoot, fileName), "utf8").trim()
    : readFileSync(join(root, "node_modules", "@codemirror", "language-data", "LICENSE"), "utf8")
      .replace(/^Copyright.*$/m, "Copyright (c) Rich Harris").trim();
  sections.push("=".repeat(72));
  sections.push(`${entry.name}@${entry.versions?.join(",") ?? "?"} — ${entry.license}`);
  sections.push("=".repeat(72));
  sections.push(notice);
  sections.push("");
}

if (missing.length) throw new Error(`Missing dependency license files: ${missing.join(", ")}`);
writeFileSync(join(root, "src-tauri", "resources", "THIRD_PARTY_NOTICES.txt"), `${sections.join("\n").trimEnd()}\n`, "utf8");
console.log(`Wrote notices for ${packages.length} production JavaScript packages.`);
