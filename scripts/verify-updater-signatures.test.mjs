import { createHash, generateKeyPairSync, sign } from "node:crypto";
import { describe, expect, it } from "vitest";
import { verifyUpdaterSignature } from "./verify-updater-signatures.mjs";

const fixture = (algorithm = "ED") => {
  const { publicKey, privateKey } = generateKeyPairSync("ed25519");
  const rawKey = publicKey.export({ format: "der", type: "spki" }).subarray(-32);
  const keyId = Buffer.from("0102030405060708", "hex");
  const data = Buffer.from("A signed updater package");
  const message = algorithm === "ED" ? createHash("blake2b512").update(data).digest() : data;
  const signature = sign(null, message, privateKey);
  const comment = "timestamp:123456 file:setup.exe";
  const globalSignature = sign(null, Buffer.concat([signature, Buffer.from(comment)]), privateKey);
  const pubkey = Buffer.from(`untrusted comment: fixture\n${Buffer.concat([Buffer.from("Ed"), keyId, rawKey]).toString("base64")}\n`).toString("base64");
  const envelope = Buffer.from(`untrusted comment: fixture\n${Buffer.concat([Buffer.from(algorithm), keyId, signature]).toString("base64")}\ntrusted comment: ${comment}\n${globalSignature.toString("base64")}\n`).toString("base64");
  return { data, pubkey, envelope };
};

describe("updater signature verification", () => {
  it.each(["ED", "Ed"])("accepts valid %s signatures and rejects altered packages", (algorithm) => {
    const { data, pubkey, envelope } = fixture(algorithm);
    expect(() => verifyUpdaterSignature(data, envelope, pubkey)).not.toThrow();
    expect(() => verifyUpdaterSignature(Buffer.from("Changed package"), envelope, pubkey)).toThrow(/invalid/);
  });
  it("rejects a changed trusted comment and unrelated public key", () => {
    const { data, pubkey, envelope } = fixture();
    const changed = Buffer.from(Buffer.from(envelope, "base64").toString().replace("timestamp:123456", "timestamp:654321")).toString("base64");
    expect(() => verifyUpdaterSignature(data, changed, pubkey)).toThrow(/comment/);
    expect(() => verifyUpdaterSignature(data, envelope, fixture().pubkey)).toThrow();
  });
});
