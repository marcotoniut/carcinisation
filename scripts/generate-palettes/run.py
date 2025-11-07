"""Generate palette and filter textures used by the runtime/editor."""

from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable, Sequence

from PIL import Image

REPO_ROOT = Path(__file__).resolve().parents[2]
PALETTE_OUTPUT = REPO_ROOT / "assets" / "palette"
FILTER_OUTPUT = REPO_ROOT / "assets" / "filter"
PALETTE_SOURCE = Path(__file__).with_name("palettes.json")
TRANSPARENT_PIXEL = (0, 0, 0, 0)


@dataclass(frozen=True)
class Palette:
    name: str
    colors: list[tuple[int, int, int, int]]


def load_palettes(path: Path) -> list[Palette]:
    data = json.loads(path.read_text(encoding="utf-8"))
    raw_palettes: Iterable[dict[str, object]] = data.get("palettes", [])

    palettes: list[Palette] = []
    for entry in raw_palettes:
        name = str(entry["name"])
        raw_colors: Iterable[Sequence[int]] = entry["colors"]
        colors = [
            (
                int(channel[0]),
                int(channel[1]),
                int(channel[2]),
                int(channel[3]),
            )
            for channel in raw_colors
        ]
        palettes.append(Palette(name=name, colors=colors))
    return palettes


def build_strip(colors: Sequence[Sequence[int]]) -> Image.Image:
    width = len(colors) + 1
    image = Image.new("RGBA", (width, 1))
    image.putpixel((0, 0), TRANSPARENT_PIXEL)

    for idx, color in enumerate(colors, start=1):
        image.putpixel((idx, 0), tuple(color))
    return image


def save_palette_strip(palette: Palette) -> None:
    PALETTE_OUTPUT.mkdir(parents=True, exist_ok=True)
    image = build_strip(palette.colors)
    image.save(PALETTE_OUTPUT / f"{palette.name}.png")


def save_base_filters(palette: Palette) -> None:
    FILTER_OUTPUT.mkdir(parents=True, exist_ok=True)
    frame_width = len(palette.colors) + 1

    for index, color in enumerate(palette.colors):
        image = Image.new("RGBA", (frame_width, 1))
        image.putpixel((0, 0), TRANSPARENT_PIXEL)
        for column in range(1, frame_width):
            image.putpixel((column, 0), tuple(color))
        image.save(FILTER_OUTPUT / f"color{index}.px_filter.png")

    invert_image = build_strip(list(reversed(palette.colors)))
    invert_image.save(FILTER_OUTPUT / "invert.px_filter.png")


def main() -> None:
    palettes = load_palettes(PALETTE_SOURCE)
    filter_sets = 0
    for palette in palettes:
        save_palette_strip(palette)
        if palette.name == "base":
            save_base_filters(palette)
            filter_sets += 1
    palette_dir = PALETTE_OUTPUT.relative_to(REPO_ROOT)
    filter_dir = FILTER_OUTPUT.relative_to(REPO_ROOT)
    print(
        f"Generated {len(palettes)} palette strips in '{palette_dir}'"
        f" and {filter_sets} filter set(s) in '{filter_dir}'."
    )

if __name__ == "__main__":
    main()
