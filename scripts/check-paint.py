#!/usr/bin/env python3
"""Живая проверка Краски: контур → заливка → пиксели → системный диалог.

Проверяет эффект, а не отсутствие паники:
  * прямоугольник рисуется в заданных точках;
  * заливка красит только внутренность контура (пиксель снаружи — белый);
  * Ctrl+O поднимает системный диалог (процесс zenity).
"""
import os
import re
import subprocess
import sys
import time

DISPLAY = ":0"
ENV = dict(os.environ, DISPLAY=DISPLAY)


def run(cmd, **kw):
    return subprocess.run(cmd, shell=isinstance(cmd, str), capture_output=True, text=True, env=ENV, **kw)


def window(title: str) -> tuple[str, int, int, int, int]:
    ids = run(["xdotool", "search", "--onlyvisible", "--class", ".*"]).stdout.split()
    for wid in ids:
        name = run(["xdotool", "getwindowname", wid]).stdout.strip()
        if name == title:
            geo = run(["xdotool", "getwindowgeometry", wid]).stdout
            pos = re.search(r"Position: (\d+),(\d+)", geo)
            size = re.search(r"Geometry: (\d+)x(\d+)", geo)
            return wid, int(pos.group(1)), int(pos.group(2)), int(size.group(1)), int(size.group(2))
    raise SystemExit(f"окно «{title}» не найдено")


def read_ppm(path: str):
    data = open(path, "rb").read()
    if not data.startswith(b"P6"):
        raise SystemExit("не PPM — конвертация не удалась")
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


def pixel(pixels: bytes, w: int, x: int, y: int):
    i = (y * w + x) * 3
    return tuple(pixels[i : i + 3])


def white_bbox(w: int, h: int, pixels: bytes):
    """Рамка холста: строки и столбцы с длинными белыми пробегами.

    Просто min/max белых пикселей не годится — белым бывает и текст заголовка.
    """
    def is_white(x: int, y: int) -> bool:
        r, g, b = pixel(pixels, w, x, y)
        return r > 240 and g > 240 and b > 240

    rows = []
    for y in range(h):
        count = sum(1 for x in range(0, w, 2) if is_white(x, y))
        if count * 2 > w * 0.5:
            rows.append(y)
    if not rows:
        raise SystemExit("белый холст не найден")
    top, bottom = min(rows), max(rows)
    cols = []
    for x in range(w):
        count = sum(1 for y in range(top, bottom + 1, 2) if is_white(x, y))
        if count * 2 > (bottom - top) * 0.5:
            cols.append(x)
    left, right = min(cols), max(cols)
    return left, top, right, bottom


def drag(x1, y1, x2, y2, steps=18):
    run(["xdotool", "mousemove", str(x1), str(y1)])
    run(["xdotool", "mousedown", "1"])
    for i in range(1, steps + 1):
        run(["xdotool", "mousemove", str(int(x1 + (x2 - x1) * i / steps)), str(int(y1 + (y2 - y1) * i / steps))])
        time.sleep(0.012)
    run(["xdotool", "mouseup", "1"])
    time.sleep(0.25)


def shot(wid: str, path: str):
    run(["import", "-window", wid, path])
    run(["convert", path, path + ".ppm"])
    return read_ppm(path + ".ppm")


def main():
    wid, wx, wy, ww, wh = window("Краски")
    run(["xdotool", "windowactivate", "--sync", wid])
    time.sleep(0.8)

    w, h, pixels = shot(wid, "/tmp/adv-0.png")
    bx1, by1, bx2, by2 = white_bbox(w, h, pixels)
    print(f"окно {ww}x{wh} @ {wx},{wy}; холст в окне: {bx1},{by1}..{bx2},{by2}")
    if wx != 0 or wy != 0:
        pass

    # клавиши: 1 карандаш, 2 кисть, 3 ластик, 4 линия, 5 прямоугольник,
    # 6 овал, 7 текст, 8 заливка
    run(["xdotool", "key", "--window", wid, "5"])
    time.sleep(0.2)
    ax, ay = wx + bx1 + 70, wy + by1 + 60
    bx, by = wx + bx1 + 420, wy + by1 + 280
    drag(ax, ay, bx, by)
    w, h, pixels = shot(wid, "/tmp/adv-1.png")
    print("после рамки: центр", pixel(pixels, w, (bx1 + 245), (by1 + 170)))

    # 8 — заливка, кликаем внутрь рамки
    run(["xdotool", "key", "--window", wid, "8"])
    time.sleep(0.2)
    run(["xdotool", "mousemove", str(wx + bx1 + 245), str(wy + by1 + 170)])
    run(["xdotool", "click", "1"])
    time.sleep(0.8)
    w, h, pixels = shot(wid, "/tmp/adv-2.png")

    inside = pixel(pixels, w, bx1 + 245, by1 + 170)
    outside = pixel(pixels, w, bx1 + 20, by1 + 20)
    print("внутри контура:", inside)
    print("снаружи контура:", outside)

    ok_fill = inside[0] < 120 and outside[0] > 240 and outside[1] > 240
    print("заливка внутри и только внутри:", "OK" if ok_fill else "ПРОВАЛ")

    # Открытие: Ctrl+O должен поднять системный диалог прямо сейчас
    run(["xdotool", "key", "--window", wid, "ctrl+o"])
    time.sleep(2.0)
    open_pids = run(["pgrep", "-x", "zenity"]).stdout.strip()
    ok_open = bool(open_pids)
    print("диалог открытия (процесс):", open_pids.replace("\n", " ") if ok_open else "НЕ ПОДНЯЛСЯ")
    if ok_open:
        run(["pkill", "-x", "zenity"])
        time.sleep(0.8)

    # Сохранение: zenity --save, затем проверяем файл
    target = "/tmp/paint.ppm"  # путь по умолчанию у приложения
    if os.path.exists(target):
        os.remove(target)
    run(["xdotool", "key", "--window", wid, "ctrl+s"])
    time.sleep(2.0)
    save_dialog = run("pgrep -x zenity || true").stdout.strip()
    ok_save = False
    if save_dialog:
        time.sleep(0.6)
        # имя предзаполнено через --filename, поэтому достаточно нажать OK
        dialog_ids = [w for w in run(["xdotool", "search", "--onlyvisible", "--class", ".*"]).stdout.split()
                      if run(["xdotool", "getwindowname", w]).stdout.strip() in ("Сохранить", "Save File")]
        if dialog_ids:
            geo = run(["xdotool", "getwindowgeometry", dialog_ids[-1]]).stdout
            pos = re.search(r"Position: (\d+),(\d+)", geo)
            run(["xdotool", "mousemove", str(int(pos.group(1)) + 963), str(int(pos.group(2)) + 715)])
            time.sleep(0.3)
            run(["xdotool", "click", "1"])
        time.sleep(2.5)
        if os.path.exists(target):
            size = os.path.getsize(target)
            sw, sh, sp = read_ppm(target)
            ok_save = size > 1000 and pixel(sp, sw, 245, 170)[0] < 120
            print(f"сохранено: {target}, {size} байт, {sw}x{sh}, центр {pixel(sp, sw, 245, 170)}")
        else:
            print("файл не появился:", target)
    else:
        print("диалог сохранения не поднялся")

    # Ctrl+O — системный диалог
    run(["xdotool", "key", "--window", wid, "ctrl+o"])
    time.sleep(2.0)
    dialogs = run("pgrep -x zenity || true").stdout.strip()
    print("системный диалог:", dialogs.splitlines()[0] if dialogs else "НЕ ПОДНЯЛСЯ")
    ok_dialog = bool(dialogs)
    if ok_dialog:
        run(["pkill", "-x", "zenity"])
    time.sleep(0.6)
    shot(wid, "/tmp/adv-3.png")

    print("\nИТОГ:")
    print(f"  заливка контура: {'works' if ok_fill else 'broken'}")
    print(f"  системный диалог открытия: {'works' if ok_open else 'broken'}")
    print(f"  сохранение файла: {'works' if ok_save else 'broken'}")
    return 0 if (ok_fill and ok_open and ok_save) else 1


if __name__ == "__main__":
    sys.exit(main())
