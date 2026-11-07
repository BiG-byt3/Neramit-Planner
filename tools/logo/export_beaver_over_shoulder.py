"""Install the approved over-shoulder beaver PNG as Neramit Planner's icon.

Run: python tools/logo/export_beaver_over_shoulder.py
Uses the selected 1024 px image verbatim; only the Windows/UI exports are resized.
"""
from pathlib import Path
import shutil
import sys

from PIL import Image

sys.dont_write_bytecode = True
from export_beaver import SIZES, icon_at, preview

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent
SELECTED = HERE / "beaver-poses-20261107" / "02-over-shoulder-1024.png"
BACKUP = HERE / "archive" / "pre-beaver-over-shoulder-20261107"


def main():
    with Image.open(SELECTED) as image:
        if image.size != (1024, 1024) or image.mode != "RGBA":
            raise ValueError("The approved icon must be a 1024 px RGBA PNG.")
        master = image.copy()

    assets = ROOT / "assets"
    for path in (
        HERE / "neramit-planner-logo-1024.png",
        HERE / "neramit-planner-beaver-1024.png",
        assets / "icon.ico",
        assets / "icon-64.rgba",
        assets / "logo-256.rgba",
    ):
        destination = BACKUP / path.name
        if path.exists() and not destination.exists():
            BACKUP.mkdir(parents=True, exist_ok=True)
            shutil.copy2(path, destination)

    # Copy the approved PNG exactly, keeping its palette, framing, and corner alpha.
    shutil.copy2(SELECTED, HERE / "neramit-planner-logo-1024.png")
    shutil.copy2(SELECTED, HERE / "neramit-planner-beaver-1024.png")
    frames = [icon_at(master, size) for size in SIZES]
    master.save(assets / "icon.ico", format="ICO", sizes=[(n, n) for n in SIZES], append_images=frames)
    for size, name in ((64, "icon-64.rgba"), (256, "logo-256.rgba")):
        (assets / name).write_bytes(icon_at(master, size).tobytes())
    preview(master)
    print("Installed the approved over-shoulder beaver: master PNG, 10 ICO frames, 64/256 RGBA, and preview.")


if __name__ == "__main__":
    main()
