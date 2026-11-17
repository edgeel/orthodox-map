# /// script
# requires-python = ">=3.11"
# dependencies = ["playwright"]
# ///
"""Screenshots of a built dist/ in headless Chromium, and a check that pages log no JS errors.

    uv run tools/screenshots.py [dist] [out-dir] [prefix]

Needs Chromium once: `uv run --with playwright python -m playwright install chromium-headless-shell`.
"""

import functools
import http.server
import sys
import threading
from pathlib import Path

from playwright.sync_api import sync_playwright

dist = Path(sys.argv[1] if len(sys.argv) > 1 else "dist").resolve()
out = Path(sys.argv[2] if len(sys.argv) > 2 else "shots").resolve()
prefix = sys.argv[3] if len(sys.argv) > 3 else "orthodox-map-preview"
out.mkdir(parents=True, exist_ok=True)

handler = functools.partial(http.server.SimpleHTTPRequestHandler, directory=str(dist))
handler.log_message = lambda *a: None
srv = http.server.ThreadingHTTPServer(("127.0.0.1", 0), handler)
threading.Thread(target=srv.serve_forever, daemon=True).start()
base = f"http://127.0.0.1:{srv.server_address[1]}/"

errors: list[str] = []


def page(browser, w, h, scheme="light", mobile=False):
    ctx = browser.new_context(viewport={"width": w, "height": h}, color_scheme=scheme,
                              device_scale_factor=2 if mobile else 1.5, is_mobile=mobile, has_touch=mobile)
    p = ctx.new_page()
    p.on("pageerror", lambda e: errors.append(f"pageerror: {e}"))
    p.on("console", lambda m: m.type == "error" and errors.append(f"console: {m.text}"))
    return p


with sync_playwright() as pw:
    b = pw.chromium.launch()

    p = page(b, 1440, 900)
    p.goto(base)
    p.wait_for_timeout(500)
    p.screenshot(path=out / f"{prefix}-map.png")

    p.goto(base + "#topic/evangelie")
    p.wait_for_selector("#panel:not([hidden]) .card")
    p.wait_for_timeout(400)
    p.click(".status button[data-st=done]")
    p.wait_for_timeout(200)
    p.screenshot(path=out / f"{prefix}-card.png")
    assert p.locator('.node[data-id="evangelie"].st-done').count() == 1, "progress mark not painted"

    p.fill("#q", "корепанов")
    p.wait_for_timeout(200)
    shown = p.locator(".node:not(.dim)").count()
    assert 0 < shown < p.locator(".node").count(), f"search did not filter ({shown})"

    p = page(b, 1440, 900, scheme="dark")
    p.goto(base + "#route/otcy")
    p.wait_for_timeout(600)
    p.screenshot(path=out / f"{prefix}-route-dark.png")

    p = page(b, 390, 844, mobile=True)
    p.goto(base)
    p.wait_for_timeout(400)
    p.screenshot(path=out / f"{prefix}-phone-list.png")
    p.click('.li-title[data-topic="simvol-very"]')
    p.wait_for_selector("#panel:not([hidden]) .card")
    p.wait_for_timeout(300)
    p.screenshot(path=out / f"{prefix}-phone-card.png")

    p = page(b, 1100, 900)
    p.goto(base + "topic/kappadokijcy/")
    p.wait_for_timeout(300)
    p.screenshot(path=out / f"{prefix}-topic-page.png", full_page=True)

    b.close()

srv.shutdown()
if errors:
    print("\n".join(errors))
    sys.exit(1)
print(f"ok: screenshots in {out}")
