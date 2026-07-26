import { spawn, spawnSync } from "node:child_process";
import { copyFile, mkdir, readFile, rm, stat } from "node:fs/promises";
import net from "node:net";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "@playwright/test";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const assetDir = path.join(root, ".github", "assets");
const fixtureDir = path.join(root, "tests", "fixtures", "readme-assets");
const outputDir = path.join(root, "output", "readme-assets");
const frameDir = path.join(outputDir, "workflow-frames");
const maximumGifBytes = 8 * 1024 * 1024;
const frameRate = 10;
const frameCount = 110;

function run(command, args, options = {}) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, { cwd: root, stdio: ["ignore", "pipe", "pipe"], ...options });
    let output = "";
    child.stdout.on("data", (chunk) => { output += chunk; });
    child.stderr.on("data", (chunk) => { output += chunk; });
    child.on("error", reject);
    child.on("exit", (code) => {
      if (code === 0) resolve(output);
      else reject(new Error(`${command} exited with ${code}\n${output.slice(-4000)}`));
    });
  });
}

async function freePort() {
  return new Promise((resolve, reject) => {
    const server = net.createServer();
    server.unref();
    server.on("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      server.close(() => resolve(address.port));
    });
  });
}

async function waitForServer(url, child) {
  const deadline = Date.now() + 40_000;
  while (Date.now() < deadline) {
    if (child.exitCode != null) throw new Error(`Vite exited before becoming ready (${child.exitCode})`);
    try {
      const response = await fetch(url);
      if (response.ok) return;
    } catch {
      // The development server is still starting.
    }
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  throw new Error(`Timed out waiting for ${url}`);
}

async function stopServer(child) {
  if (child.exitCode != null) return;
  if (process.platform === "win32") {
    spawnSync("taskkill", ["/PID", String(child.pid), "/T", "/F"], { stdio: "ignore" });
  } else {
    child.kill("SIGTERM");
  }
}

function fixtureIndex(seed) {
  const numeric = seed.match(/(\d+)/)?.[1];
  if (numeric) return ((Number(numeric) - 1) % 10) + 1;
  let hash = 0;
  for (const character of seed) hash = (hash + character.codePointAt(0)) % 10;
  return hash + 1;
}

async function installDemoImageRoute(page) {
  await page.route("**/readme-demo/*.png", async (route) => {
    const seed = path.basename(new URL(route.request().url()).pathname, ".png");
    const fixture = path.join(fixtureDir, `ui-${fixtureIndex(seed)}.png`);
    await route.fulfill({ status: 200, contentType: "image/png", body: await readFile(fixture) });
  });
}

async function openDemo(page, baseUrl) {
  await installDemoImageRoute(page);
  await page.goto(`${baseUrl}/?demo=1&readme=1`, { waitUntil: "domcontentloaded" });
  await page.getByRole("button", { name: /Quiet Notes/ }).waitFor({ state: "visible", timeout: 20_000 });
}

async function setTheme(page, theme) {
  await page.evaluate((value) => { document.documentElement.dataset.theme = value; }, theme);
  await page.waitForTimeout(220);
}

async function captureWritingWorkflow(browser, baseUrl) {
  await rm(frameDir, { recursive: true, force: true });
  await mkdir(frameDir, { recursive: true });
  const page = await browser.newPage({ viewport: { width: 1360, height: 860 }, deviceScaleFactor: 1 });
  await openDemo(page, baseUrl);
  await setTheme(page, "light");

  const article = page.locator(".article-item").filter({ hasText: "盛夏散步" });
  const editor = page.locator(".cm-content");
  const addition = "## 今日记录\n\n写下当时的光线、声音，以及为什么想把这一刻留下。";
  let typed = 0;

  for (let frame = 0; frame < frameCount; frame += 1) {
    if (frame === 8) {
      await article.click();
      await page.locator(".markdown-preview").getByText("城市很热，但树影下仍有一些安静的时刻。", { exact: true }).waitFor();
    }
    if (frame === 18) {
      await editor.click();
      await page.keyboard.press("Control+End");
      await page.keyboard.press("Enter");
      await page.keyboard.press("Enter");
    }
    if (frame >= 22 && frame <= 55) {
      const target = Math.ceil(((frame - 21) / 34) * addition.length);
      if (target > typed) {
        await page.keyboard.insertText(addition.slice(typed, target));
        typed = target;
        await page.waitForTimeout(18);
      }
    }
    if (frame === 66) {
      await page.getByRole("button", { name: "隐藏预览" }).click();
      await page.locator(".editor-grid.preview-hidden").waitFor();
    }
    if (frame === 84) {
      await page.getByRole("button", { name: "显示预览" }).click();
      await page.locator(".preview-pane").waitFor();
    }
    await page.screenshot({
      path: path.join(frameDir, `frame-${String(frame).padStart(4, "0")}.png`),
      animations: "disabled"
    });
  }
  await page.close();

  const candidates = [1100, 960];
  let selected = null;
  for (const width of candidates) {
    const candidate = path.join(outputDir, `writing-workflow-${width}.gif`);
    await run("ffmpeg", [
      "-hide_banner", "-loglevel", "error", "-y",
      "-framerate", String(frameRate),
      "-i", path.join(frameDir, "frame-%04d.png"),
      "-filter_complex",
      `[0:v]scale=${width}:-2:flags=lanczos,split[source][paletteInput];[paletteInput]palettegen=max_colors=96:stats_mode=diff[palette];[source][palette]paletteuse=dither=bayer:bayer_scale=3:diff_mode=rectangle`,
      "-loop", "0",
      candidate
    ]);
    const size = (await stat(candidate)).size;
    if (size <= maximumGifBytes) {
      selected = candidate;
      break;
    }
  }
  if (!selected) throw new Error("The README workflow GIF is larger than 8 MiB at every configured size");
  await copyFile(selected, path.join(assetDir, "writing-workflow.gif"));
}

async function waitForImageGrid(page) {
  await page.waitForFunction(() => {
    const images = [...document.querySelectorAll(".image-grid .asset-thumb img")];
    return images.length === 10 && images.every((image) => image.complete && image.naturalWidth > 0);
  }, null, { timeout: 20_000 });
}

async function captureImageBed(browser, baseUrl) {
  const page = await browser.newPage({ viewport: { width: 1360, height: 860 }, deviceScaleFactor: 1 });
  await openDemo(page, baseUrl);
  await page.getByRole("button", { name: "图床" }).click();
  await page.getByRole("heading", { name: "图床" }).waitFor();
  await waitForImageGrid(page);
  for (const theme of ["light", "dark"]) {
    await setTheme(page, theme);
    await page.screenshot({ path: path.join(assetDir, `image-bed-${theme}.png`), animations: "disabled" });
  }
  await page.close();
}

async function captureContentSync(browser, baseUrl) {
  const page = await browser.newPage({ viewport: { width: 1360, height: 860 }, deviceScaleFactor: 1 });
  await openDemo(page, baseUrl);
  await page.getByRole("button", { name: "设置" }).click();
  await page.getByRole("button", { name: /内容同步/ }).click();
  const panel = page.locator(".settings-content-panel");
  await panel.getByLabel("同步方式").selectOption("webdav");
  await panel.getByLabel("WebDAV 服务器地址").fill("https://dav.example.com/dav");
  await panel.getByLabel("WebDAV 远端目录").fill("hexo/my-blog");
  await panel.getByLabel("WebDAV 用户名").fill("blogger");
  await panel.getByLabel("WebDAV 密码").fill("temporary-demo-password");
  await panel.getByRole("button", { name: "保存并测试连接" }).click();
  await panel.getByText("WebDAV 真实连接和预检通过").waitFor();
  await panel.getByRole("button", { name: "确认启用 WebDAV" }).click();
  await panel.getByText("当前已应用连接").waitFor();
  if (await panel.getByLabel("WebDAV 密码").inputValue()) throw new Error("README capture retained a demo password");
  await page.locator(".task-indicator").waitFor({ state: "detached", timeout: 7_000 });
  await page.locator(".settings-page").evaluate((element) => { element.scrollTop = 120; });
  await page.waitForTimeout(220);

  for (const theme of ["light", "dark"]) {
    await setTheme(page, theme);
    await page.screenshot({ path: path.join(assetDir, `content-sync-${theme}.png`), fullPage: true, animations: "disabled" });
  }
  await page.close();
}

async function main() {
  if (spawnSync("ffmpeg", ["-version"], { stdio: "ignore" }).status !== 0) {
    throw new Error("ffmpeg is required to generate the README GIF");
  }
  await mkdir(assetDir, { recursive: true });
  await mkdir(outputDir, { recursive: true });
  const port = await freePort();
  const baseUrl = `http://127.0.0.1:${port}`;
  const viteEntry = path.join(root, "node_modules", "vite", "bin", "vite.js");
  const server = spawn(process.execPath, [viteEntry, "dev", "--host", "127.0.0.1", "--port", String(port)], {
    cwd: root,
    stdio: ["ignore", "pipe", "pipe"]
  });
  let serverOutput = "";
  server.stdout.on("data", (chunk) => { serverOutput += chunk; });
  server.stderr.on("data", (chunk) => { serverOutput += chunk; });

  try {
    await waitForServer(baseUrl, server);
    const launchOptions = process.platform === "win32" ? { channel: "msedge", headless: true } : { headless: true };
    const browser = await chromium.launch(launchOptions);
    try {
      await captureWritingWorkflow(browser, baseUrl);
      await captureImageBed(browser, baseUrl);
      await captureContentSync(browser, baseUrl);
    } finally {
      await browser.close();
    }
  } catch (error) {
    if (serverOutput) process.stderr.write(serverOutput.slice(-4000));
    throw error;
  } finally {
    await stopServer(server);
  }

  process.stdout.write("Generated README GIF and light/dark product screenshots.\n");
}

main().catch((error) => {
  process.stderr.write(`${error instanceof Error ? error.stack : String(error)}\n`);
  process.exitCode = 1;
});
