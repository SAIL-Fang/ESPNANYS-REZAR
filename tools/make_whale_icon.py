#!/usr/bin/env python3
"""Generate the original SAIL blue-haired whale-chan app icon.

This is an original vector-like raster drawing, not a copy of any DeepSeek
fan art. The character uses a sailor collar, whale-fin ears, a whale-tail
hairpin, and the project's blue palette. Shapes are drawn at 4x and reduced
with Lanczos so the 16/32 px icon keeps a clear silhouette.
"""
from __future__ import annotations

import math
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter, ImageOps

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "assets" / "icon"
OUT.mkdir(parents=True, exist_ok=True)

N = 1024
S = 4
W = N * S
PALETTE = {
    "bg_top": (7, 20, 43, 255),
    "bg_bottom": (12, 49, 86, 255),
    "bg_glow": (36, 128, 190, 255),
    "hair_dark": (18, 50, 99, 255),
    "hair_mid": (39, 105, 188, 255),
    "hair_light": (95, 181, 255, 255),
    "hair_shine": (170, 226, 255, 255),
    "skin": (255, 225, 211, 255),
    "skin_shadow": (241, 177, 164, 255),
    "white": (248, 252, 255, 255),
    "white_shadow": (204, 224, 241, 255),
    "navy": (8, 29, 56, 255),
    "navy_light": (17, 55, 96, 255),
    "cyan": (79, 220, 255, 255),
    "cyan_dark": (24, 139, 186, 255),
    "pink": (255, 151, 167, 255),
    "eye": (5, 30, 63, 255),
    "iris": (31, 151, 239, 255),
    "iris_dark": (10, 83, 163, 255),
    "pupil": (2, 15, 35, 255),
    "mouth": (108, 34, 52, 255),
    "tongue": (255, 129, 145, 255),
}


def u(value: float) -> int:
    return int(round(value * W))


def box(cx: float, cy: float, rx: float, ry: float) -> tuple[int, int, int, int]:
    return (u(cx - rx), u(cy - ry), u(cx + rx), u(cy + ry))


def poly(draw: ImageDraw.ImageDraw, points: list[tuple[float, float]], fill) -> None:
    draw.polygon([(u(x), u(y)) for x, y in points], fill=fill)


def ellipse(draw: ImageDraw.ImageDraw, cx: float, cy: float, rx: float, ry: float, fill) -> None:
    draw.ellipse(box(cx, cy, rx, ry), fill=fill)


def line(
    draw: ImageDraw.ImageDraw,
    points: list[tuple[float, float]],
    fill,
    width: float,
    joint: str = "curve",
) -> None:
    scaled = [(u(x), u(y)) for x, y in points]
    draw.line(scaled, fill=fill, width=u(width), joint=joint)
    radius = u(width) / 2
    for x, y in (scaled[0], scaled[-1]):
        draw.ellipse((x - radius, y - radius, x + radius, y + radius), fill=fill)


def linear_gradient(size: int, top, bottom) -> Image.Image:
    image = Image.new("RGBA", (1, size))
    pixels = image.load()
    for y in range(size):
        t = y / max(1, size - 1)
        pixels[0, y] = tuple(int(top[i] * (1 - t) + bottom[i] * t) for i in range(4))
    return image.resize((size, size), Image.Resampling.BILINEAR)


def rounded_mask(size: int, radius: float) -> Image.Image:
    mask = Image.new("L", (size, size), 0)
    d = ImageDraw.Draw(mask)
    d.rounded_rectangle((0, 0, size - 1, size - 1), radius=int(size * radius), fill=255)
    return mask


canvas = Image.new("RGBA", (W, W), (0, 0, 0, 0))
draw = ImageDraw.Draw(canvas)

# Background tile: deep ocean blue with a soft centre glow and water rings.
bg_mask = rounded_mask(W, 0.215)
bg = linear_gradient(W, PALETTE["bg_top"], PALETTE["bg_bottom"])
canvas.paste(bg, (0, 0), bg_mask)

glow = Image.new("RGBA", (W, W), (0, 0, 0, 0))
gd = ImageDraw.Draw(glow)
for radius, alpha in ((0.52, 38), (0.38, 46), (0.26, 54)):
    gd.ellipse(
        box(0.50, 0.43, radius, radius * 0.90),
        fill=(30, 127, 201, alpha),
    )
glow = glow.filter(ImageFilter.GaussianBlur(u(0.065)))
canvas.alpha_composite(glow)

# Subtle waves near the bottom.
wave = Image.new("RGBA", (W, W), (0, 0, 0, 0))
wd = ImageDraw.Draw(wave)
for row, (y, alpha, width) in enumerate(
    ((0.805, 32, 0.006), (0.845, 25, 0.004), (0.885, 20, 0.003))
):
    points = []
    for i in range(41):
        x = i / 40
        points.append((x, y + math.sin((x * 2.1 + row * 0.42) * math.pi) * 0.012))
    line(wd, points, (112, 213, 255, alpha), width)
canvas.alpha_composite(wave)

# Rounded inner rim.
rim = Image.new("RGBA", (W, W), (0, 0, 0, 0))
rd = ImageDraw.Draw(rim)
rd.rounded_rectangle(
    (u(0.012), u(0.012), u(0.988), u(0.988)),
    radius=u(0.205),
    outline=(121, 196, 239, 54),
    width=u(0.004),
)
canvas.alpha_composite(rim)

# Soft drop shadow behind the character.
shadow = Image.new("RGBA", (W, W), (0, 0, 0, 0))
sd = ImageDraw.Draw(shadow)
sd.ellipse(box(0.50, 0.55, 0.35, 0.36), fill=(0, 8, 21, 95))
shadow = shadow.filter(ImageFilter.GaussianBlur(u(0.055)))
canvas.alpha_composite(shadow)

# Back hair: a broad silhouette rather than many strands, so it reads at 16 px.
ellipse(draw, 0.50, 0.46, 0.31, 0.34, PALETTE["hair_dark"])
poly(
    draw,
    [
        (0.22, 0.47),
        (0.28, 0.66),
        (0.21, 0.81),
        (0.31, 0.91),
        (0.42, 0.83),
        (0.45, 0.65),
        (0.40, 0.49),
    ],
    PALETTE["hair_dark"],
)
poly(
    draw,
    [
        (0.78, 0.47),
        (0.72, 0.66),
        (0.79, 0.81),
        (0.69, 0.91),
        (0.58, 0.83),
        (0.55, 0.65),
        (0.60, 0.49),
    ],
    PALETTE["hair_dark"],
)
# Side hair masses, mid blue.
ellipse(draw, 0.30, 0.54, 0.105, 0.16, PALETTE["hair_mid"])
ellipse(draw, 0.70, 0.54, 0.105, 0.16, PALETTE["hair_mid"])

# Whale-fin ears behind the face.
for mirror in (False, True):
    sx = 1 if mirror else -1
    cx = 0.50 - sx * 0.285
    fin = [
        (cx, 0.43),
        (cx - sx * 0.16, 0.37),
        (cx - sx * 0.22, 0.47),
        (cx - sx * 0.13, 0.61),
        (cx - sx * 0.02, 0.54),
    ]
    poly(draw, fin, PALETTE["navy_light"])
    inner = [
        (cx - sx * 0.015, 0.44),
        (cx - sx * 0.14, 0.40),
        (cx - sx * 0.16, 0.48),
        (cx - sx * 0.09, 0.57),
    ]
    poly(draw, inner, PALETTE["hair_mid"])
    line(
        draw,
        [
            (cx - sx * 0.10, 0.43),
            (cx - sx * 0.13, 0.50),
            (cx - sx * 0.10, 0.56),
        ],
        PALETTE["cyan_dark"],
        0.007,
    )

# Body and neck.
ellipse(draw, 0.50, 0.815, 0.125, 0.09, PALETTE["skin_shadow"])
ellipse(draw, 0.50, 0.795, 0.105, 0.075, PALETTE["skin"])
poly(
    draw,
    [
        (0.28, 1.02),
        (0.29, 0.87),
        (0.38, 0.78),
        (0.50, 0.82),
        (0.62, 0.78),
        (0.71, 0.87),
        (0.72, 1.02),
    ],
    PALETTE["navy"],
)
# Shoulders.
ellipse(draw, 0.34, 0.905, 0.12, 0.12, PALETTE["navy_light"])
ellipse(draw, 0.66, 0.905, 0.12, 0.12, PALETTE["navy_light"])

# Sailor collar: two bright wings and a V opening.
poly(draw, [(0.50, 0.79), (0.29, 0.86), (0.37, 0.96), (0.49, 0.88)], PALETTE["white"])
poly(draw, [(0.50, 0.79), (0.71, 0.86), (0.63, 0.96), (0.51, 0.88)], PALETTE["white"])
poly(draw, [(0.50, 0.79), (0.41, 0.88), (0.50, 0.95), (0.59, 0.88)], PALETTE["navy_light"])
line(draw, [(0.31, 0.87), (0.37, 0.92)], PALETTE["white_shadow"], 0.008)
line(draw, [(0.69, 0.87), (0.63, 0.92)], PALETTE["white_shadow"], 0.008)

# Neck ribbon / cyan bow.
ellipse(draw, 0.44, 0.875, 0.045, 0.034, PALETTE["cyan"])
ellipse(draw, 0.56, 0.875, 0.045, 0.034, PALETTE["cyan"])
ellipse(draw, 0.50, 0.875, 0.025, 0.027, PALETTE["cyan_dark"])
line(draw, [(0.50, 0.89), (0.47, 0.95)], PALETTE["cyan"], 0.010)
line(draw, [(0.50, 0.89), (0.53, 0.95)], PALETTE["cyan"], 0.010)

# Whale-tail chest pin.
poly(draw, [(0.50, 0.94), (0.46, 0.90), (0.44, 0.955)], PALETTE["cyan"])
poly(draw, [(0.50, 0.94), (0.54, 0.90), (0.56, 0.955)], PALETTE["cyan"])
ellipse(draw, 0.50, 0.945, 0.013, 0.013, PALETTE["white"])

# Face.
ellipse(draw, 0.50, 0.515, 0.205, 0.225, PALETTE["skin"])
# Soft cheek shadow.
ellipse(draw, 0.50, 0.60, 0.16, 0.10, PALETTE["skin_shadow"])

# Hair fringe: centre-parted locks, leaving the face open.
poly(
    draw,
    [
        (0.30, 0.39),
        (0.40, 0.29),
        (0.49, 0.34),
        (0.45, 0.48),
        (0.38, 0.53),
        (0.32, 0.50),
    ],
    PALETTE["hair_mid"],
)
poly(
    draw,
    [
        (0.70, 0.39),
        (0.60, 0.29),
        (0.51, 0.34),
        (0.55, 0.48),
        (0.62, 0.53),
        (0.68, 0.50),
    ],
    PALETTE["hair_mid"],
)
poly(draw, [(0.40, 0.29), (0.50, 0.22), (0.60, 0.29), (0.55, 0.38), (0.45, 0.38)], PALETTE["hair_dark"])
# Long side locks.
poly(
    draw,
    [
        (0.30, 0.43),
        (0.24, 0.67),
        (0.28, 0.86),
        (0.35, 0.77),
        (0.36, 0.53),
    ],
    PALETTE["hair_dark"],
)
poly(
    draw,
    [
        (0.70, 0.43),
        (0.76, 0.67),
        (0.72, 0.86),
        (0.65, 0.77),
        (0.64, 0.53),
    ],
    PALETTE["hair_dark"],
)
poly(
    draw,
    [
        (0.24, 0.60),
        (0.19, 0.77),
        (0.23, 0.88),
        (0.29, 0.78),
        (0.31, 0.63),
    ],
    PALETTE["hair_mid"],
)
poly(
    draw,
    [
        (0.76, 0.60),
        (0.81, 0.77),
        (0.77, 0.88),
        (0.71, 0.78),
        (0.69, 0.63),
    ],
    PALETTE["hair_mid"],
)
# Highlight streaks.
line(draw, [(0.36, 0.35), (0.32, 0.49), (0.34, 0.64)], PALETTE["hair_light"], 0.012)
line(draw, [(0.64, 0.35), (0.68, 0.49), (0.66, 0.64)], PALETTE["hair_light"], 0.012)
line(draw, [(0.44, 0.26), (0.48, 0.30)], PALETTE["hair_shine"], 0.009)
# A small centre fringe keeps the forehead from reading as a bald gap.
poly(
    draw,
    [
        (0.45, 0.285),
        (0.50, 0.315),
        (0.55, 0.285),
        (0.57, 0.355),
        (0.51, 0.405),
        (0.49, 0.405),
        (0.43, 0.355),
    ],
    PALETTE["hair_mid"],
)
line(draw, [(0.48, 0.31), (0.50, 0.38)], PALETTE["hair_light"], 0.007)

# Ahoge.
line(
    draw,
    [(0.51, 0.22), (0.535, 0.15), (0.57, 0.10), (0.55, 0.055), (0.49, 0.035)],
    PALETTE["hair_dark"],
    0.020,
)
line(draw, [(0.51, 0.22), (0.535, 0.15), (0.565, 0.11)], PALETTE["hair_light"], 0.007)

# Eyes: large enough to survive 16 px, with two catchlights each.
for mirror in (False, True):
    cx = 0.405 if not mirror else 0.595
    ellipse(draw, cx, 0.505, 0.060, 0.077, PALETTE["white"])
    ellipse(draw, cx, 0.512, 0.044, 0.062, PALETTE["iris_dark"])
    ellipse(draw, cx, 0.518, 0.035, 0.053, PALETTE["iris"])
    ellipse(draw, cx, 0.520, 0.019, 0.035, PALETTE["pupil"])
    ellipse(draw, cx - 0.012, 0.493, 0.014, 0.018, PALETTE["white"])
    ellipse(draw, cx + 0.014, 0.535, 0.008, 0.011, PALETTE["cyan"])
    # upper lash
    line(draw, [(cx - 0.055, 0.47), (cx, 0.45), (cx + 0.055, 0.47)], PALETTE["eye"], 0.016)
    # lower lid
    line(draw, [(cx - 0.045, 0.575), (cx, 0.588), (cx + 0.045, 0.575)], PALETTE["skin_shadow"], 0.006)

# Brows.
line(draw, [(0.355, 0.425), (0.405, 0.415), (0.445, 0.43)], PALETTE["eye"], 0.012)
line(draw, [(0.645, 0.425), (0.595, 0.415), (0.555, 0.43)], PALETTE["eye"], 0.012)

# Nose and mouth.
ellipse(draw, 0.500, 0.575, 0.010, 0.007, PALETTE["skin_shadow"])
ellipse(draw, 0.500, 0.618, 0.038, 0.021, PALETTE["mouth"])
ellipse(draw, 0.500, 0.628, 0.028, 0.011, PALETTE["tongue"])
line(draw, [(0.468, 0.610), (0.500, 0.601), (0.532, 0.610)], PALETTE["skin_shadow"], 0.005)

# Blush.
for cx in (0.335, 0.665):
    blush = Image.new("RGBA", (W, W), (0, 0, 0, 0))
    bd = ImageDraw.Draw(blush)
    bd.ellipse(box(cx, 0.585, 0.052, 0.025), fill=(255, 145, 164, 110))
    blush = blush.filter(ImageFilter.GaussianBlur(u(0.012)))
    canvas.alpha_composite(blush)

# Small cyan water droplets, a whale-girl cue.
for cx, cy, r in ((0.72, 0.105, 0.017), (0.755, 0.155, 0.011), (0.695, 0.165, 0.008)):
    ellipse(draw, cx, cy, r, r * 1.15, PALETTE["cyan"])
    ellipse(draw, cx - r * 0.3, cy - r * 0.35, r * 0.28, r * 0.28, PALETTE["white"])

# Whale-tail hairpin on the left.
poly(
    draw,
    [(0.245, 0.325), (0.195, 0.285), (0.205, 0.365), (0.242, 0.342)],
    PALETTE["white"],
)
poly(
    draw,
    [(0.245, 0.325), (0.295, 0.285), (0.285, 0.365), (0.248, 0.342)],
    PALETTE["white"],
)
ellipse(draw, 0.245, 0.332, 0.018, 0.018, PALETTE["cyan"])

# Soft top-light rim on the hair.
rim_light = Image.new("RGBA", (W, W), (0, 0, 0, 0))
rl = ImageDraw.Draw(rim_light)
line(rl, [(0.22, 0.34), (0.31, 0.23), (0.47, 0.18)], (168, 229, 255, 90), 0.020)
line(rl, [(0.53, 0.18), (0.69, 0.23), (0.78, 0.34)], (168, 229, 255, 90), 0.020)
rim_light = rim_light.filter(ImageFilter.GaussianBlur(u(0.010)))
canvas.alpha_composite(rim_light)

# Final rounded crop so the icon has transparent corners.
final_mask = rounded_mask(W, 0.215)
canvas.putalpha(Image.composite(canvas.getchannel("A"), Image.new("L", (W, W), 0), final_mask))
canvas = canvas.resize((N, N), Image.Resampling.LANCZOS)
canvas.save(OUT / "whale-chan-1024.png")

# Useful small previews for the README/UI.
for size in (16, 32, 48, 128, 256, 512):
    canvas.resize((size, size), Image.Resampling.LANCZOS).save(OUT / f"whale-chan-{size}.png")

print(OUT / "whale-chan-1024.png")
