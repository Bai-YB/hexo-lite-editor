const path = require("node:path");
const { createRequire } = require("node:module");

async function main() {
  const projectRoot = path.resolve(process.argv[2] || "");
  const requestedSource = String(process.argv[3] || "").replaceAll("\\", "/");
  if (!projectRoot || !requestedSource) throw new Error("缺少项目或文章参数");

  const projectRequire = createRequire(path.join(projectRoot, "package.json"));
  const Hexo = projectRequire("hexo");
  const hexo = new Hexo(projectRoot, { silent: true });
  try {
    await hexo.init();
    await hexo.load();
    const candidates = [
      ...(hexo.locals.get("posts")?.toArray?.() || []),
      ...(hexo.locals.get("pages")?.toArray?.() || [])
    ];
    const normalize = (value) => String(value || "").replaceAll("\\", "/").replace(/^\/+/, "");
    const requested = normalize(requestedSource);
    const article = candidates.find((candidate) => {
      const source = normalize(candidate.source);
      const fullSource = normalize(candidate.full_source);
      return source === requested || fullSource === requested || fullSource.endsWith(`/${requested}`);
    });
    if (!article) throw new Error(`Hexo 未收录文章：${requested}`);
    const route = article.permalink || article.path;
    if (!route) throw new Error("文章没有可用的 permalink 或 path");
    process.stdout.write(JSON.stringify({ path: String(route) }));
  } finally {
    await hexo.exit();
  }
}

main().catch((error) => {
  process.stderr.write(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
});
