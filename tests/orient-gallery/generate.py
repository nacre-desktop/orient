"""Original procedural gallery artwork, dedicated to CC0; no private wallpaper."""

from pathlib import Path
from PIL import Image, ImageDraw
import math


def images(folder):
    folder = Path(folder)
    folder.mkdir(parents=True, exist_ok=True)
    descriptions = {
        "warm-room": ("543320", "28160e", "e69b43", "ce677f"),
        "night-city": ("13254a", "07112b", "3766aa", "ec66b9"),
        "coast-day": ("dfe9ee", "dde4da", "3c8baf", "e7bf6b"),
        "mist-forest": ("879e9a", "253f38", "687e74", "a3b0a4"),
        "neon-scene": ("202044", "0b0c24", "3e57bf", "e33393"),
        "monochrome": ("bbbbbb", "151515", "777777", "dddddd"),
    }
    for name, colors in descriptions.items():
        image = Image.new("RGB", (256, 144))
        pixels = image.load()
        sky, ground, subject, detail = [tuple(int(c[k : k + 2], 16) for k in (0, 2, 4)) for c in colors]
        for y in range(144):
            for x in range(256):
                base = sky if y < 83 else ground
                shade = 0.83 + 0.15 * y / 144 + 0.02 * math.sin((x + y) * 0.07)
                pixels[x, y] = tuple(round(v * shade) for v in base)
        draw = ImageDraw.Draw(image)
        draw.polygon([(86, 118), (94, 54), (124, 31), (150, 57), (158, 118)], fill=subject)
        for x in range(98, 149, 7):
            draw.line((x, 65, x + 3, 110), fill=ground, width=2)
        draw.ellipse((158, 47, 178, 67), fill=detail)
        draw.line((168, 0, 168, 47), fill=detail, width=2)
        draw.rectangle((0, 128, 255, 143), fill=ground)
        image.save(folder / (name + ".png"))
    base = Image.new("RGB", (128, 80), "#14253e")
    frames = []
    for x in (24, 43, 62, 81):
        item = base.copy()
        ImageDraw.Draw(item).ellipse((x, 25, x + 12, 37), fill="#efc585")
        frames.append(item)
    frames[0].save(
        folder / "moving-lantern.gif",
        save_all=True,
        append_images=frames[1:],
        duration=200,
        loop=0,
    )
    return [folder / (name + ".png") for name in descriptions]


if __name__ == "__main__":
    images(Path(__file__).parent / "images")
