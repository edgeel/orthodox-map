# /// script
# requires-python = ">=3.10"
# dependencies = ["quickjs"]
# ///
"""JavaScript checks without a browser: QuickJS (from PyPI) compiles assets/app.js and assets/sw.js,
runs the pure progress block of app.js against old and new saved data, and, given a single-file
export, compiles its inline script and parses its inline data.

    uv run tools/jscheck.py [dist/orthodox-map.html]
"""

import json
import re
import sys
from pathlib import Path

import quickjs

ROOT = Path(__file__).resolve().parent.parent


def compile_js(name: str, src: str) -> None:
    ctx = quickjs.Context()
    ctx.set("src", src)
    try:
        ctx.eval("new Function(src); 0")
    except quickjs.JSException as e:
        sys.exit(f"{name}: синтаксическая ошибка JavaScript: {e}")


def pure_block(src: str) -> str:
    m = re.search(r"// pure:start\n(.*?)// pure:end", src, re.S)
    if not m:
        sys.exit("assets/app.js: нет блока // pure:start … // pure:end")
    return m.group(1)


PURE_TESTS = r"""
const eq = (a, b, what) => {
  if (JSON.stringify(a) !== JSON.stringify(b)) throw new Error(what + ": " + JSON.stringify(a) + " != " + JSON.stringify(b));
};
// Saves from before "want" load unchanged.
eq(cleanProgress({ a: "reading", b: "done" }), { a: "reading", b: "done" }, "old save");
// The third state is kept; junk and unknown states are dropped.
eq(cleanProgress({ a: "want", b: "done", c: "maybe", d: 5, e: null }), { a: "want", b: "done" }, "new save");
eq(cleanProgress(null), {}, "null");
eq(cleanProgress("done"), {}, "string");
eq(cleanProgress(["want"]), {}, "array");
// An export file: { app, version, saved, progress }.
const exported = { app: "orthodox-map", version: 1, saved: "2026-11-16", progress: { x: "want", y: "reading" } };
eq(cleanProgress(exported.progress || exported), { x: "want", y: "reading" }, "export file");
// "Мои отметки" filter.
eq(statusMatches("want", "want"), true, "want/want");
eq(statusMatches("done", "want"), false, "done/want");
eq(statusMatches(undefined, "want"), false, "none/want");
eq(statusMatches(undefined, ""), true, "none/all");
eq(statusMatches("reading", ""), true, "reading/all");
eq(statusMatches(undefined, "none"), true, "none/none");
eq(statusMatches("want", "none"), false, "want/none");
eq(STATES, ["want", "reading", "done"], "states");
"ok";
"""


def main() -> None:
    app = (ROOT / "assets/app.js").read_text(encoding="utf-8")
    compile_js("assets/app.js", app)
    compile_js("assets/sw.js", (ROOT / "assets/sw.js").read_text(encoding="utf-8"))

    ctx = quickjs.Context()
    try:
        ctx.eval(pure_block(app) + PURE_TESTS)
    except quickjs.JSException as e:
        sys.exit(f"assets/app.js, блок pure: {e}")

    for arg in sys.argv[1:]:
        html = Path(arg).read_text(encoding="utf-8")
        data = re.search(r'<script type="application/json" id="omap-data">(.*?)</script>', html, re.S)
        if not data:
            sys.exit(f"{arg}: нет встроенных данных #omap-data")
        topics = json.loads(data.group(1))["topics"]
        scripts = re.findall(r"<script>(.*?)</script>", html, re.S)
        if not any("pure:start" in s for s in scripts):
            sys.exit(f"{arg}: нет встроенного app.js")
        for i, s in enumerate(scripts):
            compile_js(f"{arg}, <script> №{i + 1}", s)
        print(f"{arg}: тем {len(topics)}, скриптов {len(scripts)}, JS компилируется")

    print("ok: app.js и sw.js компилируются, статусы прогресса проверены (QuickJS)")


if __name__ == "__main__":
    main()
