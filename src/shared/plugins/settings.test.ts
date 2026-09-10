import { describe, expect, it } from "vitest";
import { initializeSettings, validateSettings } from "./settings";

describe("plugin settings form contract", () => {
  it("materializes defaults while preserving explicit false, zero and saved values", () => {
    const schema = { properties: { endpoint: { default: "https://example.com" }, enabled: { default: true }, count: { default: 4 } } };
    expect(initializeSettings(schema, { enabled: false, count: 0 })).toEqual({ endpoint: "https://example.com", enabled: false, count: 0 });
  });
  it("reports required fields, types, ranges and enums before saving", () => {
    const schema = { required: ["endpoint"], properties: { endpoint: { type: "string", format: "uri" }, enabled: { type: "boolean" }, count: { type: "integer", minimum: 1 }, mode: { enum: ["a", "b"] } } };
    expect(Object.keys(validateSettings(schema, { enabled: "false", count: 0, mode: "c" }))).toEqual(["endpoint", "enabled", "count", "mode"]);
    expect(validateSettings(schema, { endpoint: "https://example.com", enabled: false, count: 2, mode: "a" })).toEqual({});
  });
});
