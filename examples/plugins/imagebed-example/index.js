let nextHostRequest = 0;
const pendingHostRequests = new Map();

self.addEventListener("message", async (event) => {
  const message = event.data;
  if (pendingHostRequests.has(message.id)) {
    const pending = pendingHostRequests.get(message.id);
    pendingHostRequests.delete(message.id);
    message.error ? pending.reject(new Error(message.error.message)) : pending.resolve(message.result);
    return;
  }

  try {
    if (message.method === "image.validateConfig") {
      const endpoint = typeof message.params?.endpoint === "string" ? message.params.endpoint : "";
      return respond(message.id, { ok: endpoint.startsWith("https://"), message: endpoint ? undefined : "endpoint is required" });
    }
    if (message.method === "image.testConnection") {
      const result = await hostRequest("network.request", { url: `${message.params.endpoint}/health`, method: "GET" });
      return respond(message.id, { ok: result.status >= 200 && result.status < 300 });
    }
    if (message.method === "image.upload") {
      // This example uses a deterministic URL so it remains safe to install without credentials.
      const name = encodeURIComponent(message.params.name || "image.png");
      const endpoint = String(message.params.config?.endpoint || "https://example.com").replace(/\/$/, "");
      return respond(message.id, { url: `${endpoint}/uploads/${name}`, markdown: `![${message.params.name}](${endpoint}/uploads/${name})` });
    }
    throw new Error(`Unsupported method: ${message.method}`);
  } catch (error) {
    self.postMessage({ id: message.id, error: { code: "example_failed", message: error.message } });
  }
});

function respond(id, result) {
  self.postMessage({ id, result });
}

function hostRequest(method, params) {
  const id = `host-${++nextHostRequest}`;
  return new Promise((resolve, reject) => {
    pendingHostRequests.set(id, { resolve, reject });
    self.postMessage({ kind: "hostRequest", id, method, params });
  });
}
