"""Build the macOS Dock icon with consistent transparent optical padding.

Requires Pillow. The Windows icon and the in-app favicon use separate assets.
"""

from io import BytesIO
from pathlib import Path
from struct import pack

from PIL import Image


ROOT = Path(__file__).resolve().parent.parent
SOURCE = ROOT / "app-icon.png"
OUTPUT = ROOT / "src-tauri" / "icons"
# The former 88% artwork still looked larger than neighbouring Dock icons.
# Match their apparent size at the Dock's standard (non-magnified) setting.
ART_SCALE = 0.74
SIZES = {
    b"icp4": 16,
    b"icp5": 32,
    b"icp6": 64,
    b"ic07": 128,
    b"ic08": 256,
    b"ic09": 512,
    b"ic10": 1024,
    b"ic11": 32,
    b"ic12": 64,
    b"ic13": 256,
    b"ic14": 512,
}


def padded_icon(source: Image.Image, size: int) -> Image.Image:
    artwork_size = round(size * ART_SCALE)
    artwork = source.resize((artwork_size, artwork_size), Image.Resampling.LANCZOS)
    canvas = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    offset = (size - artwork_size) // 2
    canvas.alpha_composite(artwork, (offset, offset))
    return canvas


def main() -> None:
    source = Image.open(SOURCE).convert("RGBA")
    image_cache: dict[int, bytes] = {}
    for size in set(SIZES.values()):
        output = BytesIO()
        padded_icon(source, size).save(output, format="PNG", optimize=True)
        image_cache[size] = output.getvalue()

    (OUTPUT / "icon-macos.png").write_bytes(image_cache[1024])
    chunks = [kind + pack(">I", len(image_cache[size]) + 8) + image_cache[size]
              for kind, size in SIZES.items()]
    payload = b"".join(chunks)
    (OUTPUT / "icon.icns").write_bytes(b"icns" + pack(">I", len(payload) + 8) + payload)


if __name__ == "__main__":
    main()
