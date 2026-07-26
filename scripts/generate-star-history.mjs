import { readFile, writeFile } from "node:fs/promises";
import { spawnSync } from "node:child_process";
import path from "node:path";
import { pathToFileURL } from "node:url";

const DEFAULT_REPOSITORY = "Bai-YB/hexo-lite-editor";
const DEFAULT_OUTPUT = ".github/assets/star-history.svg";
const DAY_MS = 86_400_000;

export function escapeXml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&apos;");
}

export function normalizeStargazers(items) {
  const counts = new Map();
  for (const item of items) {
    if (!item?.starred_at) continue;
    const date = new Date(item.starred_at);
    if (Number.isNaN(date.getTime())) continue;
    const day = date.toISOString().slice(0, 10);
    counts.set(day, (counts.get(day) ?? 0) + 1);
  }

  let total = 0;
  return [...counts.entries()]
    .sort(([left], [right]) => left.localeCompare(right))
    .map(([date, count]) => ({ date, stars: (total += count) }));
}

function parseNextLink(header) {
  if (!header) return null;
  for (const part of header.split(",")) {
    const match = part.match(/<([^>]+)>;\s*rel="next"/);
    if (match) return match[1];
  }
  return null;
}

export async function fetchStargazers(repository, { token = "", fetchImpl = fetch } = {}) {
  let url = `https://api.github.com/repos/${repository}/stargazers?per_page=100&page=1`;
  const items = [];
  const headers = {
    Accept: "application/vnd.github.star+json",
    "User-Agent": "hexo-lite-editor-star-history"
  };
  if (token) headers.Authorization = `Bearer ${token}`;

  while (url) {
    const response = await fetchImpl(url, { headers });
    if (!response.ok) {
      throw new Error(`GitHub stargazers request failed (${response.status} ${response.statusText})`);
    }
    const page = await response.json();
    if (!Array.isArray(page)) throw new Error("GitHub stargazers response was not an array");
    items.push(...page);
    url = parseNextLink(response.headers.get("link"));
  }
  return items;
}

function formatDate(value) {
  return new Intl.DateTimeFormat("en", {
    month: "short",
    day: "numeric",
    year: "numeric",
    timeZone: "UTC"
  }).format(new Date(`${value}T00:00:00Z`));
}

export function renderStarHistorySvg(repository, rawItems) {
  const points = normalizeStargazers(rawItems);
  const total = points.at(-1)?.stars ?? 0;
  const safeRepository = escapeXml(repository);
  const width = 800;
  const height = 260;
  const left = 64;
  const right = 752;
  const top = 76;
  const bottom = 208;

  let chart;
  if (!total) {
    chart = `
    <g data-state="empty">
      <line class="grid" x1="${left}" y1="${bottom}" x2="${right}" y2="${bottom}" />
      <text class="zero" x="400" y="142" text-anchor="middle">0 stars</text>
      <text class="muted" x="400" y="168" text-anchor="middle">The chart will update automatically after the first star.</text>
      <text class="axis" x="${left}" y="230">No history yet</text>
    </g>`;
  } else {
    const firstTime = Date.parse(`${points[0].date}T00:00:00Z`) - DAY_MS;
    const lastTime = Date.parse(`${points.at(-1).date}T00:00:00Z`) + DAY_MS;
    const x = (date) => left + ((Date.parse(`${date}T00:00:00Z`) - firstTime) / (lastTime - firstTime)) * (right - left);
    const y = (stars) => bottom - (stars / total) * (bottom - top);
    const linePoints = [[left, bottom], ...points.map((point) => [x(point.date), y(point.stars)])];
    const line = linePoints.map(([px, py]) => `${px.toFixed(1)},${py.toFixed(1)}`).join(" ");
    const area = `M ${left} ${bottom} L ${linePoints.slice(1).map(([px, py]) => `${px.toFixed(1)} ${py.toFixed(1)}`).join(" L ")} L ${linePoints.at(-1)[0].toFixed(1)} ${bottom} Z`;
    const lastPoint = linePoints.at(-1);
    chart = `
    <g data-state="history">
      <line class="grid" x1="${left}" y1="${bottom}" x2="${right}" y2="${bottom}" />
      <line class="grid" x1="${left}" y1="${top}" x2="${right}" y2="${top}" />
      <text class="axis" x="48" y="${bottom + 4}" text-anchor="end">0</text>
      <text class="axis" x="48" y="${top + 4}" text-anchor="end">${total}</text>
      <path class="area" d="${area}" />
      <polyline class="line" points="${line}" />
      <circle class="point" cx="${lastPoint[0].toFixed(1)}" cy="${lastPoint[1].toFixed(1)}" r="4" />
      <text class="axis" x="${left}" y="230">${escapeXml(formatDate(points[0].date))}</text>
      <text class="axis" x="${right}" y="230" text-anchor="end">${escapeXml(formatDate(points.at(-1).date))}</text>
    </g>`;
  }

  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${width} ${height}" role="img" aria-labelledby="title description">
  <title id="title">Star History for ${safeRepository}</title>
  <desc id="description">${total ? `${total} GitHub stars over time.` : "This repository currently has no GitHub stars."}</desc>
  <style>
    :root { color-scheme: light dark; }
    .title { fill: #24292f; font: 600 18px -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; }
    .count { fill: #57606a; font: 13px -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; }
    .axis, .muted { fill: #6e7781; font: 12px -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; }
    .zero { fill: #24292f; font: 600 28px -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; }
    .grid { stroke: #d0d7de; stroke-width: 1; }
    .area { fill: #d8e4ea; opacity: .58; }
    .line { fill: none; stroke: #526d7a; stroke-linecap: round; stroke-linejoin: round; stroke-width: 2.5; }
    .point { fill: #526d7a; }
    @media (prefers-color-scheme: dark) {
      .title, .zero { fill: #f0f3f6; }
      .count, .axis, .muted { fill: #8c959f; }
      .grid { stroke: #30363d; }
      .area { fill: #344b56; opacity: .55; }
      .line { stroke: #9fb6c1; }
      .point { fill: #9fb6c1; }
    }
  </style>
  <text class="title" x="32" y="38">Star History</text>
  <text class="count" x="768" y="38" text-anchor="end">${total} ${total === 1 ? "star" : "stars"}</text>${chart}
</svg>
`;
}

export function parseArgs(argv) {
  const options = {
    repository: process.env.GITHUB_REPOSITORY || DEFAULT_REPOSITORY,
    output: DEFAULT_OUTPUT,
    fixture: ""
  };
  for (let index = 0; index < argv.length; index += 1) {
    const value = argv[index];
    if (value === "--repository") options.repository = argv[++index];
    else if (value === "--output") options.output = argv[++index];
    else if (value === "--fixture") options.fixture = argv[++index];
    else throw new Error(`Unknown argument: ${value}`);
  }
  if (!options.repository?.includes("/")) throw new Error("--repository must use owner/name format");
  return options;
}

function fetchStargazersWithGh(repository) {
  const result = spawnSync("gh", [
    "api",
    "--paginate",
    "--slurp",
    `repos/${repository}/stargazers`,
    "-H",
    "Accept: application/vnd.github.star+json"
  ], { encoding: "utf8", stdio: ["ignore", "pipe", "ignore"], maxBuffer: 32 * 1024 * 1024 });
  if (result.status !== 0) return null;
  const pages = JSON.parse(result.stdout);
  return Array.isArray(pages) ? pages.flat() : null;
}

export async function main(argv = process.argv.slice(2)) {
  const options = parseArgs(argv);
  let rawItems;
  if (options.fixture) {
    rawItems = JSON.parse(await readFile(path.resolve(options.fixture), "utf8"));
  } else if (process.env.GITHUB_TOKEN) {
    rawItems = await fetchStargazers(options.repository, { token: process.env.GITHUB_TOKEN });
  } else {
    rawItems = fetchStargazersWithGh(options.repository) ?? await fetchStargazers(options.repository);
  }
  const svg = renderStarHistorySvg(options.repository, rawItems);
  await writeFile(path.resolve(options.output), svg, "utf8");
  process.stdout.write(`Wrote ${options.output} with ${normalizeStargazers(rawItems).at(-1)?.stars ?? 0} stars.\n`);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main().catch((error) => {
    process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`);
    process.exitCode = 1;
  });
}
