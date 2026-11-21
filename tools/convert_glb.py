"""Convertit le modèle Sketchfab « PS5 Controller » (Taohid Animation, CC-BY-4.0)
en `assets/manette.mkm`, le format compact lu par MeikiCtrl.

Usage :  python tools/convert_glb.py manette.glb assets/manette.mkm
Il faut Python 3 et numpy.

Le format .mkm (petit-boutiste) :
  "MKM1", u32 nb_pièces
  pour chaque pièce :
    u8 longueur + nom (utf-8)
    u32 rôle, u32 bit_bouton
    f32x3 couleur, f32x3 pivot, f32x3 enfoncement
    u32 nb_sommets, u32 nb_indices
    sommets : f32x3 position + f32x3 normale
    indices : u32
Tout est déjà dans le repère de l'appli : X droite, Y face des boutons, Z vers les gâchettes.
"""
import json
import struct
import sys

import numpy as np

# Rôles (doivent correspondre à model.rs)
STATIC, SHELL, STICK_L, STICK_R, BUTTON, TRIGGER_L, TRIGGER_R, LIGHTBAR = range(8)

# Bits des boutons (doivent correspondre à pad.rs)
SQUARE, CROSS, CIRCLE, TRIANGLE = 1 << 0, 1 << 1, 1 << 2, 1 << 3
L1, R1 = 1 << 4, 1 << 5
PS, MUTE = 1 << 12, 1 << 14
UP, DOWN, LEFT, RIGHT = 1 << 15, 1 << 16, 1 << 17, 1 << 18

SCALE = 0.57  # ramène la manette à ±1,5 unité de large
PRESS = 0.025  # profondeur d'enfoncement d'un bouton (unités appli)

# Couleurs Meikitoow (linéaires)
DARK = (0.018, 0.02, 0.026)
DARKER = (0.008, 0.009, 0.012)
CAP = (0.05, 0.055, 0.065)

# nœud glTF -> (nom, rôle, bit, couleur ou None pour garder l'originale)
PARTS = {
    5: ("dos", STATIC, 0, DARKER),
    10: ("coque", SHELL, 0, DARK),
    12: ("centre", STATIC, 0, DARKER),
    46: ("barre_lumineuse", LIGHTBAR, 0, (0.0, 0.85, 1.0)),
    47: ("leds_joueur", LIGHTBAR, 0, (0.0, 0.85, 1.0)),
    44: ("ps", BUTTON, PS, CAP),
    61: ("ps_logo", BUTTON, PS, CAP),
    49: ("micro", BUTTON, MUTE, CAP),
    66: ("triangle", BUTTON, TRIANGLE, CAP),
    68: ("carre", BUTTON, SQUARE, CAP),
    70: ("croix", BUTTON, CROSS, CAP),
    72: ("rond", BUTTON, CIRCLE, CAP),
}
DPAD = {
    UP: ("haut", [74, 76, 78]),
    RIGHT: ("droite", [80, 82, 84]),
    DOWN: ("bas", [86, 88, 90]),
    LEFT: ("gauche", [92, 94, 96, 98]),
}
STICK_NODES = [16, 14, 63]  # chapeau, jupe, anneau : chaque nœud contient les 2 sticks
SHOULDER_NODE = 59  # L1, R1, L2, R2 dans un seul nœud


def load(path):
    d = open(path, "rb").read()
    ln = struct.unpack("<I", d[12:16])[0]
    j = json.loads(d[20 : 20 + ln])
    off = 20 + ln
    bl = struct.unpack("<I", d[off : off + 4])[0]
    return j, d[off + 8 : off + 8 + bl]


def node_matrix(nd):
    if "matrix" in nd:
        return np.array(nd["matrix"], dtype=float).reshape(4, 4).T
    m = np.eye(4)
    x, y, z, w = nd.get("rotation", [0, 0, 0, 1])
    r = np.array(
        [
            [1 - 2 * (y * y + z * z), 2 * (x * y - z * w), 2 * (x * z + y * w)],
            [2 * (x * y + z * w), 1 - 2 * (x * x + z * z), 2 * (y * z - x * w)],
            [2 * (x * z - y * w), 2 * (y * z + x * w), 1 - 2 * (x * x + y * y)],
        ]
    )
    m[:3, :3] = r * np.array(nd.get("scale", [1, 1, 1]))
    m[:3, 3] = nd.get("translation", [0, 0, 0])
    return m


def main(src, dst):
    j, binary = load(src)

    def accessor(i):
        a = j["accessors"][i]
        bv = j["bufferViews"][a["bufferView"]]
        o = bv.get("byteOffset", 0) + a.get("byteOffset", 0)
        if a["componentType"] == 5126:
            ncomp = {"VEC3": 3, "VEC2": 2, "VEC4": 4}[a["type"]]
            stride = bv.get("byteStride", ncomp * 4) // 4
            arr = np.frombuffer(binary, np.float32, a["count"] * stride, o).reshape(a["count"], stride)
            return arr[:, :ncomp].astype(float)
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

    # Modèle : X droite, Y vers les gâchettes, Z face avant.
    # Appli  : X droite, Y face avant, Z vers les gâchettes.
    swap = np.array([[1, 0, 0, 0], [0, 0, 1, 0], [0, 1, 0, 0], [0, 0, 0, 1]], dtype=float)
    to_app = np.diag([SCALE, SCALE, SCALE, 1.0]) @ swap

    def mesh_of(n):
        nd = j["nodes"][n]
        p = j["meshes"][nd["mesh"]]["primitives"][0]
        m = to_app @ world(n)
        pos = accessor(p["attributes"]["POSITION"])
        nrm = accessor(p["attributes"]["NORMAL"])
        pos = (m @ np.c_[pos, np.ones(len(pos))].T).T[:, :3]
        nm = np.linalg.inv(m[:3, :3]).T
        nrm = (nm @ nrm.T).T
        nrm /= np.linalg.norm(nrm, axis=1, keepdims=True) + 1e-12
        idx = accessor(p["indices"]).reshape(-1, 3) if "indices" in p else np.arange(len(pos)).reshape(-1, 3)
        mat = p.get("material")
        color = (0.05, 0.05, 0.05)
        if mat is not None:
            pbr = j["materials"][mat].get("pbrMetallicRoughness", {})
            if "baseColorFactor" in pbr and "baseColorTexture" not in pbr:
                color = tuple(pbr["baseColorFactor"][:3])
        return pos, nrm, idx, color

    def components(pos, idx):
        """Sépare un maillage en morceaux non connectés."""
        _, inv = np.unique(np.round(pos, 5), axis=0, return_inverse=True)
        inv = inv.ravel()
        par = np.arange(inv.max() + 1)

        def find(a):
            while par[a] != a:
                par[a] = par[par[a]]
                a = par[a]
            return a

        for t in idx:
            a = find(inv[t[0]])
            for k in (1, 2):
                b = find(inv[t[k]])
                par[b] = a
        roots = np.array([find(inv[t[0]]) for t in idx])
        return [idx[roots == r] for r in np.unique(roots)]

    parts = []

    def add(name, role, bit, color, tris_list, pivot=(0, 0, 0), press=(0, 0, 0)):
        # tris_list : liste de (pos, nrm, triangles)
        verts, inds = [], []
        base = 0
        for pos, nrm, tris in tris_list:
            used = np.unique(tris.ravel())
            remap = {int(u): k for k, u in enumerate(used)}
            verts.append(np.c_[pos[used], nrm[used]])
            inds.append(np.vectorize(remap.get)(tris).ravel() + base)
            base += len(used)
        parts.append((name, role, bit, color, pivot, press, np.vstack(verts), np.concatenate(inds)))

    used_nodes = set()

    # Pièces simples
    for n, (name, role, bit, color) in PARTS.items():
        pos, nrm, idx, orig = mesh_of(n)
        press = (0, -PRESS, 0) if role == BUTTON else (0, 0, 0)
        add(name, role, bit, color or orig, [(pos, nrm, idx)], press=press)
        used_nodes.add(n)

    # Croix directionnelle
    for bit, (name, nodes) in DPAD.items():
        lst = []
        for n in nodes:
            pos, nrm, idx, _ = mesh_of(n)
            lst.append((pos, nrm, idx))
            used_nodes.add(n)
        add("dpad_" + name, BUTTON, bit, CAP, lst, press=(0, -PRESS, 0))

    # Sticks : chaque nœud est coupé en gauche / droite
    for side, role in ((-1, STICK_L), (1, STICK_R)):
        lst = []
        lo = None
        for n in STICK_NODES:
            pos, nrm, idx, _ = mesh_of(n)
            for comp in components(pos, idx):
                c = pos[comp.ravel()].mean(0)
                if np.sign(c[0]) == side:
                    lst.append((pos, nrm, comp))
                    m = pos[comp.ravel()][:, 1].min()
                    lo = m if lo is None else min(lo, m)
        allp = np.vstack([p[t.ravel()] for p, _, t in lst])
        cx, cz = allp[:, 0].mean(), allp[:, 2].mean()
        # Pivot sous le chapeau, à l'intérieur de la manette : bascule naturelle.
        pivot = (cx, lo - 0.12, cz)
        add("stick_" + ("g" if side < 0 else "d"), role, 0, CAP, lst, pivot=pivot)
    used_nodes.update(STICK_NODES)

    # Gâchettes : 4 morceaux (L1, R1 devant ; L2, R2 derrière)
    pos, nrm, idx, _ = mesh_of(SHOULDER_NODE)
    for comp in components(pos, idx):
        p = pos[comp.ravel()]
        c = p.mean(0)
        left = c[0] < 0
        is_trigger = c[1] < 0  # en dessous de la face = gâchette L2/R2
        if is_trigger:
            # Charnière : en haut, côté gâchettes (Z max), au niveau de la face.
            pivot = (c[0], p[:, 1].max(), p[:, 2].max())
            add("l2" if left else "r2", TRIGGER_L if left else TRIGGER_R, 0, CAP, [(pos, nrm, comp)], pivot=pivot)
        else:
            add("l1" if left else "r1", BUTTON, L1 if left else R1, CAP, [(pos, nrm, comp)], press=(0, -PRESS, 0))
    used_nodes.add(SHOULDER_NODE)

    # Tout le reste : décor fixe, couleur d'origine (assombrie)
    for n, nd in enumerate(j["nodes"]):
        if "mesh" in nd and n not in used_nodes:
            pos, nrm, idx, orig = mesh_of(n)
            color = tuple(min(c, 0.08) for c in orig)
            add(f"decor_{n}", STATIC, 0, color, [(pos, nrm, idx)])

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

    tris = sum(len(p[7]) // 3 for p in parts)
    print(f"{len(parts)} pièces, {tris} triangles -> {dst}")
    for p in parts:
        if p[1] != STATIC:
            print(f"  {p[0]:16} rôle={p[1]} pivot={np.round(p[4], 3)}")


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
