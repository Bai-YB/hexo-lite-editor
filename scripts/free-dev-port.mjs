import { execFileSync } from "node:child_process";
import path from "node:path";

const DEV_PORT = 1420;
const projectRoot = path.resolve(process.cwd()).toLowerCase();

if (process.platform !== "win32") {
  process.exit(0);
}

function runPowerShell(script) {
  try {
    return execFileSync("powershell.exe", ["-NoProfile", "-Command", script], {
      encoding: "utf8",
      stdio: ["ignore", "pipe", "pipe"]
    }).trim();
  } catch {
    return "";
  }
}

function findListeningPids(port) {
  const output = runPowerShell(
    `Get-NetTCPConnection -LocalPort ${port} -State Listen -ErrorAction SilentlyContinue | Select-Object -ExpandProperty OwningProcess | Sort-Object -Unique | ConvertTo-Json`
  );
  if (!output) return [];
  const data = JSON.parse(output);
  return Array.isArray(data) ? data : [data];
}

function getProcessInfo(pid) {
  const output = runPowerShell(
    `Get-CimInstance Win32_Process -Filter "ProcessId=${pid}" | Select-Object ProcessId,Name,CommandLine | ConvertTo-Json -Compress`
  );
  return output ? JSON.parse(output) : null;
}

function belongsToThisDevServer(processInfo) {
  if (!processInfo?.CommandLine) return false;
  const commandLine = processInfo.CommandLine.toLowerCase();
  const name = String(processInfo.Name || "").toLowerCase();
  const isNodeDevProcess = name.includes("node") || name.includes("pnpm") || commandLine.includes("vite");
  const isViteDevServer =
    commandLine.includes(projectRoot) ||
    commandLine.includes("vite.js") ||
    commandLine.includes("vite\\bin\\vite") ||
    commandLine.includes("vite/bin/vite");
  return isNodeDevProcess && isViteDevServer;
}

for (const pid of findListeningPids(DEV_PORT)) {
  const processInfo = getProcessInfo(pid);
  if (!belongsToThisDevServer(processInfo)) {
    console.warn(`Port ${DEV_PORT} is used by another process; not killing PID ${pid}.`);
    continue;
  }
  console.log(`Stopping stale Vite dev server on port ${DEV_PORT}: PID ${pid}`);
  execFileSync("taskkill.exe", ["/PID", String(pid), "/T", "/F"], { stdio: "inherit" });
}
