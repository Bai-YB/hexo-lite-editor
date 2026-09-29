import highlight from "highlight.js/lib/core";
import bash from "highlight.js/lib/languages/bash";
import css from "highlight.js/lib/languages/css";
import go from "highlight.js/lib/languages/go";
import java from "highlight.js/lib/languages/java";
import javascript from "highlight.js/lib/languages/javascript";
import json from "highlight.js/lib/languages/json";
import markdown from "highlight.js/lib/languages/markdown";
import python from "highlight.js/lib/languages/python";
import rust from "highlight.js/lib/languages/rust";
import sql from "highlight.js/lib/languages/sql";
import typescript from "highlight.js/lib/languages/typescript";
import xml from "highlight.js/lib/languages/xml";
import yaml from "highlight.js/lib/languages/yaml";

for (const [name, grammar] of Object.entries({
  bash, css, go, java, javascript, json, markdown, python, rust, sql, typescript, xml, yaml
})) highlight.registerLanguage(name, grammar);

/** Markdown-it escapes the source itself when this returns an empty string. */
export function highlightFencedCode(source: string, info: string): string {
  const language = info.trim().split(/\s+/, 1)[0]?.toLowerCase() ?? "";
  if (!language || !highlight.getLanguage(language) || source.length > 50_000) return "";
  try {
    return highlight.highlight(source, { language, ignoreIllegals: true }).value;
  } catch {
    return "";
  }
}
