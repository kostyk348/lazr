#!/usr/bin/env python3
"""Живая проверка Краски: контур → заливка → пиксели → файл → чтение → диалог.

Проверяется эффект, а не отсутствие паники:
  * заливка красит только внутренность замкнутого контура;
  * запись по пути создаёт файл, который читается обратно (round-trip);
  * чтение по пути возвращает картинку на холст;
  * Ctrl+O поднимает системный диалог (процесс zenity).
"""
import os
import re
import subprocess
import sys
import time

ENV = dict(os.environ, DISPLAY=":0")
TARGET = "/tmp/paint.ppm"  # путь по умолчанию у приложения


def run(cmd, **kw):
    return subprocess.run(cmd, shell=isinstance(cmd, str), capture_output=True, text=True, env=ENV, **kw)


def find_window(title: str) -> str:
    for wid in run(["xdotool", "search", "--onlyvisible", "--class", ".*"]).stdout.split():
        if run(["xdotool", "getwindowname", wid]).stdout.strip() == title:
            return wid
    raise SystemExit(f"окно «{title}» не найдено")


def geometry(wid: str):
    geo = run(["xdotool", "getwindowgeometry", wid]).stdout
    pos = re.search(r"Position: (\d+),(\d+)", geo)
    size = re.search(r"Geometry: (\d+)x(\d+)", geo)
    return int(pos.group(1)), int(pos.group(2)), int(size.group(1)), int(size.group(2))


def read_ppm(path: str):
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


def shot(wid: str, path: str):
    run(["import", "-window", wid, path])
    run(["convert", path, path + ".ppm"])
    return read_ppm(path + ".ppm")


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
        time.sleep(0.011)
    run(["xdotool", "mouseup", "1"])
    time.sleep(0.25)


def main():
    wid = find_window("Краски")
    wx, wy, _, _ = geometry(wid)
    run(["xdotool", "windowactivate", "--sync", wid])
    time.sleep(0.6)

    w, h, pixels = shot(wid, "/tmp/chk-0.png")
    bx1, by1, _, _ = canvas_box(w, h, pixels)
    print(f"холст в окне: {bx1},{by1}")

    # 1. контур и заливка
    run(["xdotool", "key", "5"])
    time.sleep(0.25)
    drag(wx + bx1 + 70, wy + by1 + 60, wx + bx1 + 420, wy + by1 + 280)
    run(["xdotool", "key", "8"])
    time.sleep(0.25)
    run(["xdotool", "mousemove", str(wx + bx1 + 245), str(wy + by1 + 170)])
    run(["xdotool", "click", "1"])
    time.sleep(0.8)
    w1, h1, px1 = shot(wid, "/tmp/chk-1.png")
    inside = pixel(px1, w1, bx1 + 245, by1 + 170)
    outside = pixel(px1, w1, bx1 + 20, by1 + 20)
    ok_fill = inside[0] < 120 and outside[0] > 240
    print(f"внутри {inside} · снаружи {outside} → заливка {'OK' if ok_fill else 'ПРОВАЛ'}")

    # 2. запись по пути
    if os.path.exists(TARGET):
        os.remove(TARGET)
    run(["xdotool", "key", "F5"])
    time.sleep(1.2)
    ok_save = False
    if os.path.exists(TARGET):
        sw, sh, sp = read_ppm(TARGET)
        ok_save = os.path.getsize(TARGET) > 1000 and pixel(sp, sw, 245, 170)[0] < 120
        print(f"записано: {TARGET} · {os.path.getsize(TARGET)} байт · {sw}x{sh} → {'OK' if ok_save else 'ПРОВАЛ'}")
    else:
        print("файл не появился → ПРОВАЛ")

    # 3. очистка и чтение обратно
    run(["xdotool", "key", "ctrl+n"])
    time.sleep(0.7)
    w2, h2, px2 = shot(wid, "/tmp/chk-2.png")
    cleared = pixel(px2, w2, bx1 + 245, by1 + 170)[0] > 240
    run(["xdotool", "key", "F6"])
    time.sleep(1.2)
    w3, h3, px3 = shot(wid, "/tmp/chk-3.png")
    restored = pixel(px3, w3, bx1 + 245, by1 + 170)[0] < 120
    ok_round = cleared and restored
    print(f"очистка {cleared} · чтение вернуло картинку {restored} → round-trip {'OK' if ok_round else 'ПРОВАЛ'}")

    # 4. системный диалог открытия
    run(["xdotool", "key", "F7"])
    time.sleep(3.5)
    open_pids = run(["pgrep", "-x", "zenity"]).stdout.strip()
    ok_dialog = bool(open_pids)
    print(f"диалог открытия: {'поднялся' if ok_dialog else 'НЕ ПОДНЯЛСЯ'}")
    if ok_dialog:
        run(["pkill", "-x", "zenity"])
        time.sleep(0.5)

    shot(wid, "/tmp/chk-final.png")
    print("\nИТОГ:")
    print(f"  заливка контура: {'works' if ok_fill else 'broken'}")
    print(f"  запись файла: {'works' if ok_save else 'broken'}")
    print(f"  чтение файла (round-trip): {'works' if ok_round else 'broken'}")
    print(f"  системный диалог открытия: {'works' if ok_dialog else 'broken'}")
    return 0 if all([ok_fill, ok_save, ok_round, ok_dialog]) else 1


if __name__ == "__main__":
    sys.exit(main())
