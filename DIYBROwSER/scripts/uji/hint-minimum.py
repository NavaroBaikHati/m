#!/usr/bin/env python3
"""Cetak ukuran minimum (WM_NORMAL_HINTS) sebuah window X11.

Dipakai untuk menguji bug resize: GtkFixed yang memakai size_request seukuran
jendela membuat ukuran minimum jendela ikut membesar, sehingga tiling
compositor memaksa ukuran dan isi tab terpotong.

Pakai: DISPLAY=:98 scripts/uji/hint-minimum.py <window-id>
"""
import ctypes
import sys

PMinSize = 1 << 4


class XSizeHints(ctypes.Structure):
    _fields_ = [
        ("flags", ctypes.c_long),
        ("x", ctypes.c_int),
        ("y", ctypes.c_int),
        ("width", ctypes.c_int),
        ("height", ctypes.c_int),
        ("min_width", ctypes.c_int),
        ("min_height", ctypes.c_int),
        ("max_width", ctypes.c_int),
        ("max_height", ctypes.c_int),
        ("width_inc", ctypes.c_int),
        ("height_inc", ctypes.c_int),
        ("min_aspect_x", ctypes.c_int),
        ("min_aspect_y", ctypes.c_int),
        ("max_aspect_x", ctypes.c_int),
        ("max_aspect_y", ctypes.c_int),
        ("base_width", ctypes.c_int),
        ("base_height", ctypes.c_int),
        ("win_gravity", ctypes.c_int),
    ]


def main() -> int:
    if len(sys.argv) < 2:
        print("pakai: hint-minimum.py <window-id>")
        return 2
    x11 = ctypes.CDLL("libX11.so.6")
    x11.XOpenDisplay.restype = ctypes.c_void_p
    layar = x11.XOpenDisplay(None)
    if not layar:
        print("tidak bisa membuka display")
        return 1
    hints = XSizeHints()
    x11.XGetWMNormalHints(
        ctypes.c_void_p(layar),
        ctypes.c_ulong(int(sys.argv[1], 0)),
        ctypes.byref(hints),
        ctypes.byref(ctypes.c_long()),
    )
    if hints.flags & PMinSize:
        print(f"minimum={hints.min_width}x{hints.min_height}")
    else:
        print("minimum=tidak diset")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
