"""Generate a seed PNG, then run: npx tauri icon src-tauri/icons/128x128.png"""
from __future__ import annotations

import struct
import zlib
from pathlib import Path


def png(w: int, h: int, rgb: tuple[int, int, int]) -> bytes:
    def chunk(tag: bytes, data: bytes) -> bytes:
        crc = struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)
        return struct.pack(">I", len(data)) + tag + data + crc

    raw = b"".join(b"\x00" + bytes(rgb) * w for _ in range(h))
    ihdr = struct.pack(">IIBBBBB", w, h, 8, 2, 0, 0, 0)
    return (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", ihdr)
        + chunk(b"IDAT", zlib.compress(raw))
        + chunk(b"IEND", b"")
    )


def main() -> None:
    root = Path(__file__).resolve().parent.parent / "src-tauri" / "icons"
    root.mkdir(parents=True, exist_ok=True)
    color = (14, 99, 156)
    for name, size in [("32x32.png", 32), ("128x128.png", 128), ("128x128@2x.png", 256)]:
        (root / name).write_bytes(png(size, size, color))
    print(f"wrote seed PNG to {root} — run: npx tauri icon src-tauri/icons/128x128.png")


if __name__ == "__main__":
    main()
