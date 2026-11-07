"""Write assets/licenses/THIRD-PARTY-LICENSES.txt: every crate linked into the Windows build
(from `cargo metadata`) with its licence, followed by the full licence and notice texts the
crates ship (identical texts printed once, with the crates they belong to).

usage: python tools/licenses.py   (run from the project folder with the cargo environment set)
"""
import json
import os
import re
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, "assets", "licenses", "THIRD-PARTY-LICENSES.txt")
NAMES = re.compile(r"^(licen[cs]e|copying|notice|copyright|authors|ofl|ufl|unlicense)([-_.].*)?$|.*licen[cs]e.*\.txt$", re.I)
FONT_TEXTS = re.compile(r".*\.txt$", re.I)

MIT = """Permission is hereby granted, free of charge, to any person obtaining a copy of this software
and associated documentation files (the "Software"), to deal in the Software without restriction,
including without limitation the rights to use, copy, modify, merge, publish, distribute,
sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all copies or
substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT
NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES
OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE."""


def main():
    meta = json.loads(subprocess.check_output(
        ["cargo", "metadata", "--format-version", "1", "--filter-platform", "x86_64-pc-windows-gnu"], cwd=ROOT))
    own = meta["resolve"]["root"]
    pk = {p["id"]: p for p in meta["packages"]}
    crates = sorted((pk[n["id"]] for n in meta["resolve"]["nodes"] if n["id"] != own), key=lambda p: (p["name"], p["version"]))
    version = pk[own]["version"]

    texts = {}  # text -> [crate labels]
    missing = []
    for p in crates:
        label = f'{p["name"]} {p["version"]}'
        d = os.path.dirname(p["manifest_path"])
        found = []
        for base, dirs, files in os.walk(d):
            depth = os.path.relpath(base, d).count(os.sep)
            if depth > 1 or os.path.basename(base) in ("src", "tests", "examples", "benches", "target"):
                dirs[:] = []
                if base != d and os.path.basename(base) != "fonts":
                    continue
            for f in sorted(files):
                in_fonts = os.path.basename(base) == "fonts"
                if NAMES.match(f) or (in_fonts and FONT_TEXTS.match(f)):
                    try:
                        t = open(os.path.join(base, f), encoding="utf-8", errors="replace").read().strip()
                    except OSError:
                        continue
                    if t:
                        found.append(t)
        if not found and "MIT" in (p.get("license") or ""):
            authors = ", ".join(a.split("<")[0].strip() for a in p.get("authors") or []) or f"the {p['name']} authors"
            found.append(f"MIT License\n\nCopyright (c) {authors}\n\n{MIT}")
        if not found:
            missing.append(label)
        for t in found:
            texts.setdefault(t, []).append(label)

    with open(OUT, "w", encoding="utf-8-sig", newline="\n") as f:
        f.write(f"Neramit Planner {version} - third-party components\n\n")
        f.write("Neramit Planner is built with the following open-source Rust crates (name, version, licence).\n")
        f.write("Where a crate offers a choice of licences, Neramit Planner uses it under the first permissive one\n")
        f.write("(MIT or Apache-2.0). The full licence and notice texts follow the list.\n\n")
        for p in crates:
            f.write(f'{p["name"] + " v" + p["version"]:<46}{p.get("license") or "(see crate)"}\n')
        f.write("\nFonts: the user interface embeds Noto Sans and Noto Sans Thai (SIL Open Font License 1.1, see\n")
        f.write("NotoSans-NotoSansThai-OFL.txt). The Noto Sans Thai files are modified (pre-shaped mark glyphs added)\n")
        f.write("and are redistributed under the same OFL 1.1. Windows system fonts offered in the Font box (Arial,\n")
        f.write("Calibri, Segoe UI, Tahoma, ...) are read from the user's Windows Fonts folder at run time and are\n")
        f.write("not distributed.\n")
        if missing:
            f.write("\nBuild-time crates without a licence file in their package (they only generate code while\n"
                    "compiling and are not part of the program; licence as declared above): " + ", ".join(missing) + "\n")
        f.write("\n\n================================================================ LICENCE TEXTS\n")
        for t, labels in sorted(texts.items(), key=lambda kv: kv[1][0]):
            f.write("\n" + "-" * 100 + "\n")
            f.write("Applies to: " + ", ".join(labels) + "\n\n")
            f.write(t + "\n")
    print(f"{len(crates)} crates, {len(texts)} distinct texts, {os.path.getsize(OUT)} bytes, missing: {missing}")


if __name__ == "__main__":
    sys.exit(main())
