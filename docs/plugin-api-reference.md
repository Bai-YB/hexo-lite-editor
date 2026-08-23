# Plugin API v0.1 reference

Workers exchange structured messages only:

```ts
type PluginRequest =
  | { id: string; method: "image.upload"; params: PluginUploadInput }
  | { id: string; method: "network.request"; params: NetworkRequest }
  | { id: string; method: "article.getCurrent"; params: {} };
```

Every response carries the same `id` and either `result` or an error. The host validates permissions before dispatch.

The worker can request a Host API by posting `{ kind: "hostRequest", id, method, params }`. The host answers with the same response shape. A contributed image-bed provider becomes selectable in Settings after the plugin is enabled; editor paste and file-picker uploads are then dispatched through `image.upload` in that worker.

## Image-bed provider

Providers implement `validateConfig`, `testConnection`, and `upload`. Listing, folder creation, rename, move, and delete are optional; unavailable operations must be omitted rather than emulated.

`network.request` accepts HTTPS URLs only and requires an exact `network:<origin>` manifest permission. Redirect targets are checked again by the host. Requests have a finite timeout and bounded response size.

## Compatibility

The host accepts only `apiVersion: "0.1"`. Additive fields may be ignored. A future incompatible contract uses another API version and requires an explicit plugin update.
