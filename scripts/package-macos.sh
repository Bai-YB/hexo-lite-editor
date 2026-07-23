#!/usr/bin/env bash
set -euo pipefail

version="${1:-1.0.4}"
if [[ ! "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+([+-][0-9A-Za-z.-]+)?$ ]]; then
  echo "Invalid release version: $version" >&2
  exit 1
fi
if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "macOS packages must be built on macOS." >&2
  exit 1
fi

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
output_dir="$repo_root/release-artifacts/$version"
target="universal-apple-darwin"
bundle_root="$repo_root/src-tauri/target/$target/release/bundle"

mkdir -p "$output_dir"
pnpm tauri build --target "$target"

app_source="$bundle_root/macos/Hexo Lite Editor.app"
dmg_source="$bundle_root/dmg/Hexo Lite Editor_${version}_universal.dmg"
if [[ ! -d "$app_source" || ! -f "$dmg_source" ]]; then
  echo "Missing macOS bundle output under $bundle_root" >&2
  exit 1
fi

app_zip="Hexo-Lite-Editor_${version}_macos-universal.app.zip"
dmg_name="Hexo-Lite-Editor_${version}_macos-universal.dmg"
ditto -c -k --sequesterRsrc --keepParent "$app_source" "$output_dir/$app_zip"
cp "$dmg_source" "$output_dir/$dmg_name"

codesign --verify --deep --strict "$app_source"
spctl --assess --type execute "$app_source" || true

commit="$(git -C "$repo_root" rev-parse HEAD)"
architecture="$(lipo -archs "$app_source/Contents/MacOS/hexo-lite-editor")"
if [[ "$architecture" != *"arm64"* || "$architecture" != *"x86_64"* ]]; then
  echo "::error title=Invalid macOS architecture::Expected arm64 and x86_64, found: $architecture"
  exit 1
fi
echo "::notice title=macOS universal binary::Architectures: $architecture"
generated_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
cat > "$output_dir/release-manifest-macos.json" <<EOF
{
  "version": "$version",
  "platform": "macos",
  "architecture": "$architecture",
  "sourceCommit": "$commit",
  "generatedAt": "$generated_at",
  "notarized": false,
  "assets": ["$dmg_name", "$app_zip"]
}
EOF

(
  cd "$output_dir"
  shasum -a 256 "$dmg_name" "$app_zip" release-manifest-macos.json > SHA256SUMS-macos.txt
)
echo "macOS release artifacts: $output_dir"
