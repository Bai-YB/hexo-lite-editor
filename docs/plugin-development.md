# Plugin development

Hexo Lite Editor Plugin API v0.1 runs JavaScript in a dedicated Web Worker. A plugin has no DOM, Tauri API, filesystem, credential-store, or process access. It can request only declared Host API capabilities.

## Package layout

```text
my-plugin/
|- manifest.json
|- index.js
|- settings.schema.json
|- icon.png
`- README.md
```

The `id` is a stable reverse-domain identifier. `apiVersion` must be `0.1`; incompatible plugins stay disabled. `entry` must be a package-relative `.js` file. Symlinks and path traversal are rejected during installation.

## Lifecycle

Install the directory, review requested permissions, and enable the plugin in Settings > Images and plugins. Disabled and incompatible plugins are never loaded. Worker crashes are isolated from the app; requests time out and terminate the worker.

## Permissions

- `network:https://example.com` permits only that exact HTTPS origin.
- `article:read` permits reading the current article through the Host API.
- `article:write` permits supported article mutations.
- `image:read-selected` permits access to images explicitly selected by the user.

Plugins cannot run commands, invoke Tauri, read arbitrary files, or obtain system credentials. Settings contain ordinary JSON only; secrets remain in the host credential store.

## Errors and logging

Reject calls with `{ code, message, recoverable }` semantics. Use stable lowercase error codes. Logs must not include authorization headers, passwords, tokens, image bytes, or full article content.

## Debugging and distribution

Develop in an unpacked directory, validate `manifest.json`, then install that directory. Test permission denial, timeout, worker errors, disable/enable, and API compatibility. Publish source and a versioned archive with checksums; never bundle native executables.

See [plugin-api-reference.md](plugin-api-reference.md) and the example in `examples/plugins/imagebed-example`.
