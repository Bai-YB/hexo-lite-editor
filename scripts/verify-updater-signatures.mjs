import { createHash, createPublicKey, verify } from "node:crypto";
import { readFile } from "node:fs/promises";
import { basename, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

// Verify the same Minisign envelope as Tauri's updater, including its signed comment.
export function verifyUpdaterSignature(data, signatureBase64, publicKeyBase64) {
  const keyLines = Buffer.from(publicKeyBase64, "base64").toString("utf8").trim().split(/\r?\n/);
  const lines = Buffer.from(signatureBase64.trim(), "base64").toString("utf8").trim().split(/\r?\n/);
  const publicKey = Buffer.from(keyLines[1] ?? "", "base64");
  const signature = Buffer.from(lines[1] ?? "", "base64");
  const globalSignature = Buffer.from(lines[3] ?? "", "base64");
  if (publicKey.length !== 42 || signature.length !== 74 || globalSignature.length !== 64 || !lines[2]?.startsWith("trusted comment: ")) throw new Error("Invalid Minisign envelope");
  if (!signature.subarray(2, 10).equals(publicKey.subarray(2, 10))) throw new Error("Updater signing key does not match");
  const algorithm = signature.subarray(0, 2).toString();
  if (!["ED", "Ed"].includes(algorithm)) throw new Error("Unsupported Minisign algorithm");
  const key = createPublicKey({ key: Buffer.concat([Buffer.from("302a300506032b6570032100", "hex"), publicKey.subarray(10)]), format: "der", type: "spki" });
  const message = algorithm === "ED" ? createHash("blake2b512").update(data).digest() : data;
  if (!verify(null, message, key, signature.subarray(10))) throw new Error("Updater package signature is invalid");
  const signedComment = Buffer.concat([signature.subarray(10), Buffer.from(lines[2].slice(17))]);
  if (!verify(null, signedComment, key, globalSignature)) throw new Error("Updater trusted comment signature is invalid");
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const [manifestPath, directory, configPath = "src-tauri/tauri.conf.json"] = process.argv.slice(2);
  if (!manifestPath || !directory) throw new Error("Usage: verify-updater-signatures.mjs <latest.json> <artifact-directory> [tauri.conf.json]");
  const manifest = JSON.parse(await readFile(manifestPath, "utf8"));
  const config = JSON.parse(await readFile(configPath, "utf8"));
  const checked = new Set();
  for (const entry of Object.values(manifest.platforms ?? {})) {
    if (checked.has(entry.url)) continue;
    const name = decodeURIComponent(new URL(entry.url).pathname.split("/").at(-1));
    if (!name || basename(name) !== name) throw new Error("Invalid updater asset name");
    const data = await readFile(join(directory, name));
    if (!Number.isSafeInteger(entry.size) || entry.size !== data.length) throw new Error(`Updater size mismatch: ${name}`);
    verifyUpdaterSignature(data, entry.signature, config.plugins.updater.pubkey);
    checked.add(entry.url);
    console.log(`Verified updater signature and size: ${name}`);
  }
  if (!checked.size) throw new Error("No updater packages found");
}
