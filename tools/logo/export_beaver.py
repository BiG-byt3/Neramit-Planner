"""Export the generated beaver artwork into Neramit Planner's icon formats.

Run ``python tools/logo/export_beaver.py`` from any working directory. Pillow is
the only dependency. Exports normalize the generated green matte to the exact
BRAND_GREEN from src/theme.rs, resize, and apply a 22% corner radius.
"""

from pathlib import Path
import re
import shutil
from statistics import median

from PIL import Image, ImageDraw, ImageFont


HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent
SOURCE = HERE / "beaver-source.png"
SIZES = (16, 20, 24, 32, 40, 48, 64, 96, 128, 256)
CORNER_RADIUS = 0.22
LANCZOS = Image.Resampling.LANCZOS


def brand_background(source):
    """Enforce the app palette after generation, retaining ink at matte edges."""
    theme = (ROOT / "src" / "theme.rs").read_text(encoding="utf-8")
    match = re.search(r"pub const BRAND_GREEN: Color32 = Color32::from_rgb\(([^)]+)\)", theme)
    if not match:
        raise ValueError("Cannot find BRAND_GREEN in src/theme.rs")
    target = tuple(int(value.strip(), 0) for value in match.group(1).split(","))
    image = source.convert("RGBA")
    # The clear upper strip is background only. Sample the generated matte's
    # actual colour so small model colour/texture differences do not reach exports.
    strip = image.crop((image.width // 4, 0, image.width * 3 // 4, max(1, image.height // 20)))
    samples = [pixel for pixel in strip.get_flattened_data() if pixel[1] > max(pixel[0], pixel[2]) + 20]
    if not samples:
        raise ValueError("Cannot sample the generated green background")
    matte = tuple(round(median(pixel[channel] for pixel in samples)) for channel in range(3))
    chroma = matte[1] - max(matte[0], matte[2])
    pixels = []
    for r, g, b, alpha in image.get_flattened_data():
        difference = g - max(r, b)
        if difference > 20:
            coverage = min(1.0, difference / chroma)
            if coverage >= 0.94:
                r, g, b = target
            else:
                r, g, b = (max(0, min(255, round(value + coverage * (new - old))))
                           for value, new, old in zip((r, g, b), target, matte))
        pixels.append((r, g, b, alpha))
    image.putdata(pixels)
    return image


def archive_previous():
    """Keep the first pre-beaver copies intact on subsequent export runs."""
    backup = HERE / "archive" / "pre-beaver"
    for source in (
        HERE / "neramit-planner-logo-1024.png",
        ROOT / "assets" / "icon.ico",
        ROOT / "assets" / "icon-64.rgba",
        ROOT / "assets" / "logo-256.rgba",
    ):
        destination = backup / source.name
        if source.exists() and not destination.exists():
            backup.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source, destination)


def rounded_master(source):
    size, supersampling = 1024, 4
    canvas_size = size * supersampling
    mask = Image.new("L", (canvas_size, canvas_size), 0)
    ImageDraw.Draw(mask).rounded_rectangle(
        (0, 0, canvas_size - 1, canvas_size - 1),
        radius=round(canvas_size * CORNER_RADIUS),
        fill=255,
    )
    master = source.convert("RGBA").resize((size, size), LANCZOS)
    master.putalpha(mask.resize((size, size), LANCZOS))
    return master


def icon_at(master, size):
    icon = master.resize((size, size), LANCZOS)
    # Lanczos can leave faint ringing outside the mask on very small exports.
    alpha = icon.getchannel("A").point(lambda value: 0 if value < 4 else value)
    icon.putalpha(alpha)
    return icon


def preview(master):
    """Show actual-size exports on light and dark desktop surfaces."""
    sheet = Image.new("RGB", (720, 444))
    draw = ImageDraw.Draw(sheet)
    font = ImageFont.load_default(size=16)
    for origin, background, foreground in (
        (0, "#F4F4F2", "#242824"),
        (360, "#202420", "#F4F4F2"),
    ):
        draw.rectangle((origin, 0, origin + 359, 443), fill=background)
        draw.text((origin + 24, 17), "Neramit Planner / Beaver", fill=foreground, font=font)
        for size, x, y in ((256, 52, 54), (64, 40, 340), (32, 158, 356), (16, 252, 364)):
            icon = icon_at(master, size)
            sheet.paste(icon, (origin + x, y), icon)
            draw.text((origin + x, y + size + 7), f"{size} px", fill=foreground, font=font)
    sheet.save(HERE / "beaver-preview.png")


def main():
    with Image.open(SOURCE) as source:
        if source.width != source.height:
            raise ValueError("The beaver source must be square; automatic cropping is disabled.")
        master = rounded_master(brand_background(source))
    archive_previous()
    assets = ROOT / "assets"
    assets.mkdir(exist_ok=True)
    master.save(HERE / "neramit-planner-logo-1024.png")
    master.save(HERE / "neramit-planner-beaver-1024.png")
    frames = [icon_at(master, size) for size in SIZES]
    master.save(assets / "icon.ico", format="ICO", sizes=[(n, n) for n in SIZES], append_images=frames)
    for size, name in ((64, "icon-64.rgba"), (256, "logo-256.rgba")):
        (assets / name).write_bytes(icon_at(master, size).tobytes())
    preview(master)
    print("Exported beaver master, Windows icon, 64/256 RGBA assets and preview.")


if __name__ == "__main__":
    main()
