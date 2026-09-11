#!/usr/bin/env bash
set -euo pipefail

if [[ "${1:-}" == "--" ]]; then
  shift
fi
version="${1:-1.0.6.1}"
if [[ ! "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+(\.[0-9]+)?$ ]]; then
  echo "Invalid release version: $version" >&2
  exit 1
fi
if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "macOS packages must be built on macOS." >&2
  exit 1
fi

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
runtime_version="$(node "$repo_root/scripts/release-version.mjs" runtimeVersion)"
if [[ "$version" != "$(node "$repo_root/scripts/release-version.mjs" version)" ]]; then
  echo "Requested release version does not match package.json" >&2
  exit 1
fi
output_dir="$repo_root/release-artifacts/$version"
target="universal-apple-darwin"
bundle_root="$repo_root/src-tauri/target/$target/release/bundle"

mkdir -p "$output_dir"
pnpm tauri build --target "$target"

app_source="$bundle_root/macos/Hexo Lite Editor.app"
dmg_source="$bundle_root/dmg/Hexo Lite Editor_${runtime_version}_universal.dmg"
if [[ ! -d "$app_source" || ! -f "$dmg_source" ]]; then
  echo "Missing macOS bundle output under $bundle_root" >&2
  exit 1
fi

short_version="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$app_source/Contents/Info.plist")"
bundle_version="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleVersion' "$app_source/Contents/Info.plist")"
if [[ "$short_version" != "1.0.6" || "$bundle_version" != "1.0.601" ]]; then
  echo "Unexpected macOS bundle versions: $short_version / $bundle_version" >&2
  exit 1
fi

app_zip="Hexo-Lite-Editor_${version}_macos-universal.app.zip"
updater_source_name="Hexo Lite Editor.app.tar.gz"
updater_name="Hexo-Lite-Editor_${version}_macos-universal.app.tar.gz"
dmg_name="Hexo-Lite-Editor_${version}_macos-universal.dmg"
ditto -c -k --sequesterRsrc --keepParent "$app_source" "$output_dir/$app_zip"
cp "$dmg_source" "$output_dir/$dmg_name"
cp "$bundle_root/macos/$updater_source_name" "$output_dir/$updater_name"
cp "$bundle_root/macos/$updater_source_name.sig" "$output_dir/$updater_name.sig"

code_signed=false
if codesign --verify --deep --strict "$app_source" 2>/dev/null; then
  code_signed=true
  spctl --assess --type execute "$app_source" || true
else
  echo "Unsigned development build; Gatekeeper may require Finder > Open on first launch."
fi

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
  "runtimeVersion": "$runtime_version",
  "bundleShortVersion": "$short_version",
  "bundleVersion": "$bundle_version",
  "platform": "macos",
  "architecture": "$architecture",
  "sourceCommit": "$commit",
  "generatedAt": "$generated_at",
  "codeSigned": $code_signed,
  "notarized": false,
  "assets": ["$dmg_name", "$app_zip", "$updater_name", "$updater_name.sig"]
}
EOF

(
  cd "$output_dir"
  shasum -a 256 "$dmg_name" "$app_zip" "$updater_name" "$updater_name.sig" release-manifest-macos.json > SHA256SUMS-macos.txt
)
echo "macOS release artifacts: $output_dir"
