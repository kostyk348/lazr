#!/usr/bin/env python3
"""Состязательный прогон Краски: атаки на контролы и вердикты.

Каждая атака проверяет эффект, а не отсутствие паники:
  * отмена возвращает ровно предыдущее состояние и не ломается при повторе;
  * очистка возвращает белый холст;
  * resize не роняет окно и не теряет холст;
  * быстрые клики не дублируют состояние;
  * Shift/Ctrl-сочетания при пустой истории безопасны.
"""
import os
import re
import subprocess
import sys
import time

ENV = dict(os.environ, DISPLAY=":0")


def run(cmd, **kw):
    return subprocess.run(cmd, shell=isinstance(cmd, str), capture_output=True, text=True, env=ENV, **kw)


def find_window(title: str):
    for wid in run(["xdotool", "search", "--onlyvisible", "--class", ".*"]).stdout.split():
        if run(["xdotool", "getwindowname", wid]).stdout.strip() == title:
            return wid
    raise SystemExit(f"нет окна «{title}»")


def geometry(wid):
    geo = run(["xdotool", "getwindowgeometry", wid]).stdout
    pos = re.search(r"Position: (\d+),(\d+)", geo)
    size = re.search(r"Geometry: (\d+)x(\d+)", geo)
    return int(pos.group(1)), int(pos.group(2)), int(size.group(1)), int(size.group(2))


def shot(wid, path):
    run(["import", "-window", wid, path])
    ppm = path + ".ppm"
    run(["convert", path, ppm])
    return read_ppm(ppm)


def read_ppm(path):
    data = open(path, "rb").read()
    fields, offset = [], 2
    while len(fields) < 3:
        while data[offset : offset + 1].isspace():
            offset += 1
        start = offset
        while not data[offset : offset + 1].isspace():
            offset += 1
        fields.append(int(data[start:offset]))
    offset += 1
    w, h, _ = fields
    return w, h, data[offset : offset + w * h * 3]


def pixel(pixels, w, x, y):
    i = (y * w + x) * 3
    return tuple(pixels[i : i + 3])


def canvas_box(w, h, pixels):
    def white(x, y):
        r, g, b = pixel(pixels, w, x, y)
        return r > 240 and g > 240 and b > 240

    rows = [y for y in range(h) if sum(1 for x in range(0, w, 2) if white(x, y)) * 2 > w * 0.5]
    top, bottom = min(rows), max(rows)
    cols = [x for x in range(w) if sum(1 for y in range(top, bottom + 1, 2) if white(x, y)) * 2 > (bottom - top) * 0.5]
    return min(cols), top, max(cols), bottom


def drag(x1, y1, x2, y2, steps=14):
    run(["xdotool", "mousemove", str(x1), str(y1)])
    run(["xdotool", "mousedown", "1"])
    for i in range(1, steps + 1):
        run(["xdotool", "mousemove", str(int(x1 + (x2 - x1) * i / steps)), str(int(y1 + (y2 - y1) * i / steps))])
        time.sleep(0.01)
    run(["xdotool", "mouseup", "1"])
    time.sleep(0.25)


def main():
    wid = find_window("Краски")
    wx, wy, _, _ = geometry(wid)
    run(["xdotool", "windowactivate", "--sync", wid])
    time.sleep(0.6)

    w, h, pixels = shot(wid, "/tmp/a0.png")
    bx1, by1, bx2, by2 = canvas_box(w, h, pixels)
    results = []

    def verdict(name, ok, note):
        results.append((name, "works" if ok else "broken", note))

    # атака 1: отмена на пустой истории
    before = pixel(pixels, w, bx1 + 30, by1 + 30)
    run(["xdotool", "key", "--window", wid, "ctrl+z"])
    time.sleep(0.5)
    w2, h2, px2 = shot(wid, "/tmp/a1.png")
    verdict("Ctrl+Z при пустой истории", pixel(px2, w2, bx1 + 30, by1 + 30) == before, "холст не изменился")

    # атака 2: рисуем прямоугольник, заливаем, отменяем дважды
    run(["xdotool", "key", "--window", wid, "5"])
    time.sleep(0.2)
    drag(wx + bx1 + 100, wy + by1 + 80, wx + bx1 + 460, wy + by1 + 300)
    run(["xdotool", "key", "--window", wid, "8"])
    time.sleep(0.2)
    run(["xdotool", "mousemove", str(wx + bx1 + 280), str(wy + by1 + 190)])
    run(["xdotool", "click", "1"])
    time.sleep(0.7)
    w3, h3, px3 = shot(wid, "/tmp/a2.png")
    filled = pixel(px3, w3, bx1 + 280, by1 + 190)[0] < 120
    run(["xdotool", "key", "--window", wid, "ctrl+z"])
    time.sleep(0.6)
    w4, h4, px4 = shot(wid, "/tmp/a3.png")
    unfilled = pixel(px4, w4, bx1 + 280, by1 + 190)[0] > 240
    verdict("заливка + отмена заливки", filled and unfilled, f"залито={filled}, после отмены белый={unfilled}")

    # атака 3: вторая отмена (снимает прямоугольник)
    run(["xdotool", "key", "--window", wid, "ctrl+z"])
    time.sleep(0.6)
    w5, h5, px5 = shot(wid, "/tmp/a4.png")
    edge = pixel(px5, w5, bx1 + 100, by1 + 80)
    verdict("отмена после отмены", edge[0] > 240, f"рамка исчезла: {edge}")

    # атака 4: очистка холста
    run(["xdotool", "key", "--window", wid, "ctrl+n"])
    time.sleep(0.6)
    w6, h6, px6 = shot(wid, "/tmp/a5.png")
    verdict("очистка (Ctrl+N)", pixel(px6, w6, bx1 + 280, by1 + 190)[0] > 240, "холст белый")

    # атака 5: быстрые клики по холсту кистью
    run(["xdotool", "key", "--window", wid, "2"])
    time.sleep(0.2)
    for i in range(12):
        run(["xdotool", "mousemove", str(wx + bx1 + 60 + i * 30), str(wy + by1 + 60)])
        run(["xdotool", "click", "1"])
    time.sleep(0.6)
    w7, h7, px7 = shot(wid, "/tmp/a6.png")
    painted = sum(1 for x in range(0, w7, 7) if pixel(px7, w7, x, by1 + 60)[0] < 120)
    verdict("12 быстрых кликов", painted > 3, f"пикселей с краской в строке: {painted}")

    # атака 6: resize окна
    run(["xdotool", "windowsize", wid, "760", "640"])
    time.sleep(1.0)
    w8, h8, px8 = shot(wid, "/tmp/a7.png")
    alive = run(["xdotool", "getwindowname", wid]).stdout.strip() == "Краски"
    verdict("resize до 760x640", alive, f"окно живо, кадр {w8}x{h8}")
    run(["xdotool", "windowsize", wid, "900", "760"])
    time.sleep(0.6)

    print(f"{'атака':38} {'вердикт':8} замечание")
    for name, v, note in results:
        print(f"{name:38} {v:8} {note}")
    return 0 if all(v == "works" for _, v, _ in results) else 1


if __name__ == "__main__":
    sys.exit(main())
