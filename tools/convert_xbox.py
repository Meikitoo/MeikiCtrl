"""Convertit « Xbox Controller-Black » (ArtStudio640, CC-BY-4.0) en `assets/xbox.mkm`.

Usage :  python tools/convert_xbox.py xbox.glb assets/xbox.mkm   (Python 3 + numpy)

Le modèle est un seul maillage : on le découpe en morceaux non reliés (286),
puis on reconnaît chaque pièce à sa position (sticks, ABXY, croix, gâchettes…).
Repère du modèle : y = face avant, -x = haut (gâchettes), -z = droite.
Repère de l'appli : X = droite, Y = face avant, Z = haut. Même format .mkm que la DualSense.
"""
import os
import struct
import sys

import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from convert_glb import load, node_matrix  # noqa: E402

STATIC, SHELL, STICK_L, STICK_R, BUTTON, TRIGGER_L, TRIGGER_R, LIGHTBAR, DPAD = range(9)
SQUARE, CROSS, CIRCLE, TRIANGLE = 1 << 0, 1 << 1, 1 << 2, 1 << 3
L1, R1, CREATE, OPTIONS, PS = 1 << 4, 1 << 5, 1 << 8, 1 << 9, 1 << 12

SCALE = 3.5  # largeur ≈ 2,9 unités, comme la DualSense
CENTER_Z = 0.412
PRESS = 0.025

DARK = (0.018, 0.02, 0.026)
DARKER = (0.008, 0.009, 0.012)
CAP = (0.05, 0.055, 0.065)


def main(src, dst):
    j, binary = load(src)

    def accessor(i):
        a = j["accessors"][i]
        bv = j["bufferViews"][a["bufferView"]]
        o = bv.get("byteOffset", 0) + a.get("byteOffset", 0)
        if a["componentType"] == 5126:
            n = {"VEC3": 3, "VEC2": 2, "VEC4": 4}[a["type"]]
            stride = bv.get("byteStride", n * 4) // 4
            return np.frombuffer(binary, np.float32, a["count"] * stride, o).reshape(a["count"], stride)[:, :n].astype(float)
        dt = {5121: np.uint8, 5123: np.uint16, 5125: np.uint32}[a["componentType"]]
        return np.frombuffer(binary, dt, a["count"], o).astype(np.int64)

    parent = {}
    for i, nd in enumerate(j["nodes"]):
        for c in nd.get("children", []):
            parent[c] = i

    def world(n):
        chain = []
        while n is not None:
            chain.append(n)
            n = parent.get(n)
        m = np.eye(4)
        for k in reversed(chain):
            m = m @ node_matrix(j["nodes"][k])
        return m

    mesh_node = next(i for i, nd in enumerate(j["nodes"]) if "mesh" in nd)
    prim = j["meshes"][j["nodes"][mesh_node]["mesh"]]["primitives"][0]
    m = world(mesh_node)
    pos = accessor(prim["attributes"]["POSITION"])
    nrm = accessor(prim["attributes"]["NORMAL"])
    pos = (m @ np.c_[pos, np.ones(len(pos))].T).T[:, :3]
    nrm = (np.linalg.inv(m[:3, :3]).T @ nrm.T).T
    tris = accessor(prim["indices"]).reshape(-1, 3)

    # Repère modèle → appli
    to_app = np.array([[0, 0, -1], [0, 1, 0], [-1, 0, 0]], dtype=float)
    app_pos = (pos - np.array([0, 0, CENTER_Z])) @ to_app.T * SCALE
    app_nrm = nrm @ to_app.T
    app_nrm /= np.linalg.norm(app_nrm, axis=1, keepdims=True) + 1e-12

    # Morceaux non reliés (connectivité des indices)
    par = np.arange(len(pos))

    def find(a):
        while par[a] != a:
            par[a] = par[par[a]]
            a = par[a]
        return a

    for t in tris:
        a = find(t[0])
        par[find(t[1])] = a
        par[find(t[2])] = a
    roots = np.array([find(t[0]) for t in tris])
    comps = [tris[roots == r] for r in np.unique(roots)]

    # Repères (en coordonnées appli) des pièces, déduits de la géométrie du modèle.
    def app(x, y, z):
        return np.array([CENTER_Z - z, y, -x]) * SCALE

    STICK_L_C = app(0.035, 0, 0.617)
    STICK_R_C = app(0.135, 0, 0.307)
    DPAD_C = app(0.12, 0, 0.52)
    FACE = {  # bouton : centre
        TRIANGLE: app(-0.022, 0, 0.203),  # Y (haut)
        CROSS: app(0.063, 0, 0.204),  # A (bas)
        CIRCLE: app(0.014, 0, 0.148),  # B (droite)
        SQUARE: app(0.024, 0, 0.259),  # X (gauche)
        CREATE: app(0.02, 0, 0.471),  # View
        OPTIONS: app(0.022, 0, 0.353),  # Menu
        PS: app(-0.049, 0, 0.412),  # Xbox
    }
    BUMPERS = {L1: app(-0.114, 0, 0.614), R1: app(-0.114, 0, 0.21)}
    TRIGGERS = {TRIGGER_L: app(-0.161, 0, 0.646), TRIGGER_R: app(-0.161, 0, 0.178)}

    groups = {}  # clé -> liste de triangles

    def add(key, comp):
        groups.setdefault(key, []).append(comp)

    def dist_xz(c, target):
        return np.hypot(c[0] - target[0], c[2] - target[2])

    for comp in comps:
        p = app_pos[comp.ravel()]
        c = (p.min(0) + p.max(0)) / 2
        size = p.max(0) - p.min(0)
        front = c[1]  # hauteur au-dessus de la face (Y appli)
        big = size.max() > 0.9
        if big:
            # Grandes surfaces : coque avant (éclairs) ou dos/poignées.
            add("coque" if c[1] > -0.05 else "dos", comp)
            continue
        # Le chapeau du stick (pas la bague fixe autour, plus large).
        stick_piece = front > 0.42 and size.max() < 0.36
        if dist_xz(c, STICK_L_C) < 0.25 and stick_piece:
            add("stick_g", comp)
            continue
        if dist_xz(c, STICK_R_C) < 0.25 and stick_piece:
            add("stick_d", comp)
            continue
        if dist_xz(c, DPAD_C) < 0.24 and front > 0.2:
            add("dpad", comp)
            continue
        hit = None
        for bit, t in FACE.items():
            if dist_xz(c, t) < 0.12 and front > 0.4:
                hit = bit
        if hit is not None:
            add(("btn", hit), comp)
            continue
        for bit, t in BUMPERS.items():
            if dist_xz(c, t) < 0.45 and size.max() > 0.2 and front > 0.55:
                hit = bit
        if hit is not None:
            add(("btn", hit), comp)
            continue
        for role, t in TRIGGERS.items():
            if dist_xz(c, t) < 0.35 and size.max() > 0.3:
                hit = role
        if hit is not None:
            add(("trig", hit), comp)
            continue
        add("decor", comp)

    parts = []

    def emit(name, role, bit, color, comps_, pivot=(0, 0, 0), press=(0, 0, 0)):
        t = np.concatenate(comps_)
        used = np.unique(t.ravel())
        remap = np.full(len(app_pos), -1, dtype=np.int64)
        remap[used] = np.arange(len(used))
        verts = np.c_[app_pos[used], app_nrm[used]]
        parts.append((name, role, bit, color, tuple(pivot), tuple(press), verts, remap[t].ravel()))

    names = {TRIANGLE: "y", CROSS: "a", CIRCLE: "b", SQUARE: "x", CREATE: "view", OPTIONS: "menu", PS: "xbox", L1: "lb", R1: "rb"}
    for key, cs in groups.items():
        allp = app_pos[np.concatenate(cs).ravel()]
        lo, hi = allp.min(0), allp.max(0)
        mid = (lo + hi) / 2
        if key == "coque":
            emit("coque", SHELL, 0, DARK, cs)
        elif key == "dos":
            emit("dos", STATIC, 0, DARKER, cs)
        elif key in ("stick_g", "stick_d"):
            pivot = (mid[0], lo[1] - 0.12, mid[2])
            emit(key, STICK_L if key == "stick_g" else STICK_R, 0, CAP, cs, pivot=pivot)
        elif key == "dpad":
            emit("croix", DPAD, 0, CAP, cs, pivot=(mid[0], lo[1], mid[2]))
        elif key == "decor":
            emit("decor", STATIC, 0, DARKER, cs)
        elif key[0] == "btn":
            bit = key[1]
            press = (0, 0, -PRESS) if bit in (L1, R1) else (0, -PRESS, 0)
            emit(names[bit], BUTTON, bit, CAP, cs, press=press)
        elif key[0] == "trig":
            role = key[1]
            # Charnière : en haut (Z max) et vers la face (Y max) de la gâchette.
            emit("lt" if role == TRIGGER_L else "rt", role, 0, CAP, cs, pivot=(mid[0], hi[1], hi[2]))

    with open(dst, "wb") as f:
        f.write(b"MKM1")
        f.write(struct.pack("<I", len(parts)))
        for name, role, bit, color, pivot, press, verts, inds in parts:
            nb = name.encode()
            f.write(struct.pack("<B", len(nb)) + nb)
            f.write(struct.pack("<II", role, bit))
            f.write(struct.pack("<3f3f3f", *color, *pivot, *press))
            f.write(struct.pack("<II", len(verts), len(inds)))
            f.write(verts.astype("<f4").tobytes())
            f.write(inds.astype("<u4").tobytes())

    print(f"{len(parts)} pièces, {sum(len(p[7]) // 3 for p in parts)} triangles -> {dst}")
    for name, role, bit, color, pivot, press, verts, inds in parts:
        print(f"  {name:10} rôle={role} triangles={len(inds) // 3}")


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
