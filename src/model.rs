// La manette 3D provisoire (formes simples) et toutes ses animations.
// Plus tard, ce fichier chargera ton modèle Blender (glTF) à la place des formes.

use glam::{EulerRot, Mat4, Quat, Vec3};

use crate::pad::{btn, PadState};

// ---- Réglages visuels ----
pub const RED: Vec3 = Vec3::new(1.0, 0.12, 0.18);
const BODY: Vec3 = Vec3::new(0.02, 0.025, 0.035);
const TRIM: Vec3 = Vec3::new(0.05, 0.055, 0.065);
const BUTTON: Vec3 = Vec3::new(0.09, 0.095, 0.11);

const STICK_TILT_DEG: f32 = 32.0;
/// Décalage visible du chapeau du stick (vue de dessus, la bascule seule se voit peu).
const STICK_SHIFT: f32 = 0.045;
const TRIGGER_DEG: f32 = 24.0;
/// Mode gyro : la manette 3D suit les mouvements de ta vraie manette.
/// Désactivé : elle reste posée et « respire » juste doucement.
const FOLLOW_MOTION: bool = false;
/// Force du suivi quand le mode gyro est activé (0 = rien, 1 = 1 pour 1).
const MOTION_STRENGTH: f32 = 0.7;
/// Sens des axes du capteur. Si la manette 3D penche à l'envers, inverse un signe.
const ACCEL_SIGN: Vec3 = Vec3::new(1.0, 1.0, -1.0);
const YAW_SIGN: f32 = -1.0;

#[derive(Clone, Copy)]
pub enum MeshId {
    Sphere = 0,
    Cylinder = 1,
    Cube = 2,
}

// ---- Modèle 3D chargé depuis assets/manette.mkm (voir tools/convert_glb.py) ----
const ROLE_SHELL: u32 = 1;
const ROLE_STICK_L: u32 = 2;
const ROLE_STICK_R: u32 = 3;
const ROLE_BUTTON: u32 = 4;
const ROLE_TRIGGER_L: u32 = 5;
const ROLE_TRIGGER_R: u32 = 6;
const ROLE_LIGHTBAR: u32 = 7;
/// Croix directionnelle d'une seule pièce (Xbox) : elle bascule vers la direction pressée.
const ROLE_DPAD: u32 = 8;
const DPAD_TILT_DEG: f32 = 9.0;
/// Les 3 formes simples occupent les maillages 0 à 2 ; les pièces du modèle suivent.
pub const ASSET_MESH_BASE: usize = 3;

#[allow(dead_code)]
pub struct AssetPart {
    pub name: String,
    pub role: u32,
    pub bit: u32,
    pub color: Vec3,
    pub pivot: Vec3,
    pub press: Vec3,
    pub verts: Vec<[f32; 6]>,
    pub indices: Vec<u32>,
}

pub struct Asset {
    pub parts: Vec<AssetPart>,
}

/// Cherche le modèle à côté du projet ou de l'exécutable (pour le remplacer facilement),
/// sinon prend celui intégré à l'exe.
pub fn load_asset(file: &str, embedded: Option<&'static [u8]>) -> Option<Asset> {
    let mut paths = vec![std::path::PathBuf::from("assets").join(file)];
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            paths.push(dir.join("assets").join(file));
            paths.push(dir.join(file));
        }
    }
    for path in paths {
        if let Ok(bytes) = std::fs::read(&path) {
            match parse_asset(&bytes) {
                Some(a) => {
                    println!("Modèle 3D chargé : {} ({} pièces)", path.display(), a.parts.len());
                    return Some(a);
                }
                None => eprintln!("Modèle 3D illisible : {}", path.display()),
            }
        }
    }
    // Sinon : le modèle intégré dans l'exécutable (version installable, sans fichier à côté).
    let a = parse_asset(embedded?)?;
    println!("Modèle 3D intégré chargé : {file} ({} pièces)", a.parts.len());
    Some(a)
}

/// Les modèles disponibles : DualSense (toujours, intégré) et Xbox (si assets/xbox.mkm existe).
pub struct Models {
    pub dualsense: Option<Asset>,
    pub xbox: Option<Asset>,
}

impl Models {
    pub fn load() -> Self {
        Self {
            dualsense: load_asset("manette.mkm", Some(EMBEDDED_MODEL)),
            xbox: load_asset("xbox.mkm", Some(EMBEDDED_XBOX)),
        }
    }

    /// Toutes les pièces à envoyer au GPU, dans l'ordre : DualSense puis Xbox.
    pub fn meshes(&self) -> Vec<(&[[f32; 6]], &[u32])> {
        [self.dualsense.as_ref(), self.xbox.as_ref()]
            .into_iter()
            .flatten()
            .flat_map(|a| a.parts.iter().map(|p| (p.verts.as_slice(), p.indices.as_slice())))
            .collect()
    }

    /// Modèle à afficher + indice de son premier maillage.
    /// `model` : 0 = auto (selon la manette branchée), 1 = DualSense, 2 = Xbox.
    pub fn pick(&self, model: u8, pad_is_xbox: bool) -> Option<(&Asset, usize)> {
        let ds_count = self.dualsense.as_ref().map_or(0, |a| a.parts.len());
        let want_xbox = model == 2 || (model == 0 && pad_is_xbox);
        match (&self.xbox, &self.dualsense) {
            (Some(x), _) if want_xbox => Some((x, ASSET_MESH_BASE + ds_count)),
            (_, Some(d)) => Some((d, ASSET_MESH_BASE)),
            (Some(x), None) => Some((x, ASSET_MESH_BASE)),
            (None, None) => None,
        }
    }
}

/// Le modèle est aussi intégré à l'exe : `MeikiCtrl.exe` fonctionne seul.
static EMBEDDED_MODEL: &[u8] = include_bytes!("../assets/manette.mkm");
static EMBEDDED_XBOX: &[u8] = include_bytes!("../assets/xbox.mkm");

struct Reader<'a> {
    b: &'a [u8],
    o: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        let s = self.b.get(self.o..self.o + n)?;
        self.o += n;
        Some(s)
    }
    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }
    fn f32(&mut self) -> Option<f32> {
        Some(f32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }
    fn v3(&mut self) -> Option<Vec3> {
        Some(Vec3::new(self.f32()?, self.f32()?, self.f32()?))
    }
}

fn parse_asset(b: &[u8]) -> Option<Asset> {
    let mut r = Reader { b, o: 0 };
    if r.take(4)? != &b"MKM1"[..] {
        return None;
    }
    let n = r.u32()? as usize;
    let mut parts = Vec::with_capacity(n);
    for _ in 0..n {
        let len = r.take(1)?[0] as usize;
        let name = String::from_utf8_lossy(r.take(len)?).into_owned();
        let role = r.u32()?;
        let bit = r.u32()?;
        let color = r.v3()?;
        let pivot = r.v3()?;
        let press = r.v3()?;
        let nv = r.u32()? as usize;
        let ni = r.u32()? as usize;
        let verts = r
            .take(nv * 24)?
            .chunks_exact(24)
            .map(|c| {
                let mut a = [0f32; 6];
                for (k, x) in a.iter_mut().enumerate() {
                    *x = f32::from_le_bytes(c[k * 4..k * 4 + 4].try_into().unwrap());
                }
                a
            })
            .collect();
        let indices = r
            .take(ni * 4)?
            .chunks_exact(4)
            .map(|c| u32::from_le_bytes(c.try_into().unwrap()))
            .collect();
        parts.push(AssetPart { name, role, bit, color, pivot, press, verts, indices });
    }
    Some(Asset { parts })
}

pub struct Draw {
    pub mesh: usize,
    pub world: Mat4,
    pub color: Vec3,
    pub emissive: Vec3,
    pub glow: f32,
    pub lightning: bool,
}

impl Draw {
    fn glow(&mut self, color: Vec3, amount: f32) -> &mut Self {
        self.emissive = color;
        self.glow = amount.clamp(0.0, 1.5);
        self
    }
    fn lightning(&mut self) -> &mut Self {
        self.lightning = true;
        self
    }
}

pub struct Scene {
    pub draws: Vec<Draw>,
    pub root: Mat4,
    pub cam_pos: Vec3,
    pub cam_target: Vec3,
    pub fov_y: f32,
    /// Intensité des éclairs (0 à 1).
    pub energy: f32,
    /// Couleur des éclairs et des lueurs.
    pub accent: Vec3,
    /// Opacité de l'overlay (0,2 à 1).
    pub opacity: f32,
}

pub struct Animator {
    pub t: f32,
    tilt: Quat,
    yaw: f32,
    g_ref: Option<Vec3>,
    g_acc: Vec3,
    g_n: u32,
    press: [f32; 32],
    last_sig: u64,
    last_active: f32,
    energy: f32,
}

impl Animator {
    pub fn new() -> Self {
        Self {
            t: 0.0,
            tilt: Quat::IDENTITY,
            yaw: 0.0,
            g_ref: None,
            g_acc: Vec3::ZERO,
            g_n: 0,
            press: [0.0; 32],
            last_sig: 0,
            last_active: 0.0,
            energy: 0.3,
        }
    }

    /// Prend la position actuelle de la manette comme position « droite ».
    pub fn recenter(&mut self) {
        self.g_ref = None;
        self.g_acc = Vec3::ZERO;
        self.g_n = 0;
        self.yaw = 0.0;
        println!("Recentrage");
    }

    pub fn idle_for(&self) -> f32 {
        self.t - self.last_active
    }

    fn press(&self, bit: u32) -> f32 {
        self.press[bit.trailing_zeros() as usize]
    }

    pub fn update(&mut self, p: &PadState, dt: f32) {
        self.t += dt;

        // Activité : un bouton, un stick, une gâchette ou un mouvement.
        let sig = signature(p);
        if sig != self.last_sig || (FOLLOW_MOTION && Vec3::from(p.gyro).length() > 10.0) {
            self.last_sig = sig;
            self.last_active = self.t;
        }

        // Appuis lissés (enfoncement + lumière qui s'allume en douceur).
        let k = 1.0 - (-dt * 35.0).exp();
        for (i, v) in self.press.iter_mut().enumerate() {
            let target = if p.buttons & (1 << i) != 0 { 1.0 } else { 0.0 };
            *v += (target - *v) * k;
        }

        // Énergie des éclairs : à fond quand tu joues, calme au repos.
        let target_e = if !p.connected {
            0.15
        } else if self.idle_for() < 0.4 {
            1.0
        } else {
            0.45
        };
        self.energy += (target_e - self.energy) * (1.0 - (-dt * 4.0).exp());

        if FOLLOW_MOTION && p.connected {
            // Inclinaison : direction de la gravité (stable, ne dérive pas).
            let g = Vec3::from(p.accel) * ACCEL_SIGN;
            if g.length() > 0.5 {
                let g = g.normalize();
                match self.g_ref {
                    None => {
                        self.g_acc += g;
                        self.g_n += 1;
                        if self.g_n >= 30 {
                            self.g_ref = Some(self.g_acc.normalize());
                        }
                    }
                    Some(r) => {
                        let q = Quat::IDENTITY.slerp(Quat::from_rotation_arc(g, r), MOTION_STRENGTH);
                        self.tilt = self.tilt.slerp(q, 1.0 - (-dt * 12.0).exp());
                    }
                }
            }
            // Rotation gauche/droite : gyroscope, avec retour doux au centre.
            self.yaw += YAW_SIGN * p.gyro[1].to_radians() * dt * MOTION_STRENGTH;
        } else {
            self.tilt = self.tilt.slerp(Quat::IDENTITY, 1.0 - (-dt * 3.0).exp());
        }
        self.yaw = (self.yaw * (-dt * 1.2).exp()).clamp(-0.6, 0.6);
    }

    fn root(&self) -> Mat4 {
        // Léger balancement permanent : la manette « respire ».
        let t = self.t;
        let sway = Quat::from_euler(
            EulerRot::YXZ,
            (t * 0.55).sin() * 0.06,
            (t * 0.8).sin() * 0.035,
            (t * 0.4).sin() * 0.02,
        );
        Mat4::from_quat(Quat::from_rotation_y(self.yaw) * self.tilt * sway)
    }
}

fn signature(p: &PadState) -> u64 {
    let q = |v: f32| ((v * 16.0).round() as i64 & 0xFF) as u64;
    p.buttons as u64
        ^ (q(p.lx) << 20)
        ^ (q(p.ly) << 28)
        ^ (q(p.rx) << 36)
        ^ (q(p.ry) << 44)
        ^ (q(p.l2) << 52)
        ^ (q(p.r2) << 58)
}

fn v(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x, y, z)
}

fn trs(t: Vec3, r: Quat, s: Vec3) -> Mat4 {
    Mat4::from_scale_rotation_translation(s, r, t)
}

struct Builder {
    root: Mat4,
    draws: Vec<Draw>,
}

impl Builder {
    fn add(&mut self, mesh: MeshId, local: Mat4, color: Vec3) -> &mut Draw {
        self.add_mesh(mesh as usize, local, color)
    }

    fn add_mesh(&mut self, mesh: usize, local: Mat4, color: Vec3) -> &mut Draw {
        self.draws.push(Draw {
            mesh,
            world: self.root * local,
            color,
            emissive: Vec3::ZERO,
            glow: 0.0,
            lightning: false,
        });
        self.draws.last_mut().unwrap()
    }
}

fn camera(draws: Vec<Draw>, root: Mat4, energy: f32, look: &Look) -> Scene {
    Scene {
        accent: look.accent,
        opacity: look.opacity,
        draws,
        root,
        // Caméra en hauteur : vue de dessus, sans déformation.
        cam_pos: v(0.0, 5.6, -2.95),
        cam_target: v(0.0, 0.0, -0.05),
        fov_y: 26f32.to_radians(),
        energy,
    }
}

/// Pivote une pièce autour de son pivot.
fn about(pivot: Vec3, rot: Quat) -> Mat4 {
    Mat4::from_translation(pivot) * Mat4::from_quat(rot) * Mat4::from_translation(-pivot)
}

/// Apparence choisie dans l'interface.
pub struct Look {
    pub accent: Vec3,
    pub opacity: f32,
    /// Habillage de la coque : 0 = éclairs, 1 = noir mat, 2 = blanc, 3 = couleur.
    pub skin: u8,
}

pub fn build(a: &Animator, p: &PadState, asset: Option<(&Asset, usize)>, look: &Look) -> Scene {
    match asset {
        Some((asset, base)) => build_asset(a, p, asset, base, look),
        None => build_basic(a, p, look),
    }
}

/// Le vrai modèle 3D, pièce par pièce.
fn build_asset(a: &Animator, p: &PadState, asset: &Asset, base: usize, look: &Look) -> Scene {
    let accent = look.accent;
    let root = a.root();
    let mut b = Builder { root, draws: Vec::with_capacity(asset.parts.len()) };
    let tilt = STICK_TILT_DEG.to_radians();

    for (i, part) in asset.parts.iter().enumerate() {
        let mesh = base + i;
        match part.role {
            ROLE_SHELL => match look.skin {
                1 => {
                    b.add_mesh(mesh, Mat4::IDENTITY, Vec3::splat(0.035));
                }
                2 => {
                    b.add_mesh(mesh, Mat4::IDENTITY, Vec3::splat(0.82));
                }
                3 => {
                    b.add_mesh(mesh, Mat4::IDENTITY, accent * 0.5);
                }
                _ => {
                    b.add_mesh(mesh, Mat4::IDENTITY, part.color).lightning();
                }
            },
            ROLE_STICK_L | ROLE_STICK_R => {
                let (sx, sy, bit) = if part.role == ROLE_STICK_L {
                    (p.lx, p.ly, btn::L3)
                } else {
                    (p.rx, p.ry, btn::R3)
                };
                let rot = Quat::from_rotation_x(sy * tilt) * Quat::from_rotation_z(-sx * tilt);
                let press = a.press(bit);
                let mag = (sx * sx + sy * sy).sqrt().min(1.0);
                let shift = v(sx * STICK_SHIFT, -press * 0.03, sy * STICK_SHIFT);
                let m = Mat4::from_translation(shift) * about(part.pivot, rot);
                b.add_mesh(mesh, m, part.color).glow(accent, press + mag * 0.55);
            }
            ROLE_BUTTON if part.bit != 0 => {
                let pr = a.press(part.bit);
                let col = if part.bit == btn::PS || part.bit == btn::MUTE { RED } else { accent };
                b.add_mesh(mesh, Mat4::from_translation(part.press * pr), part.color).glow(col, pr);
            }
            ROLE_TRIGGER_L | ROLE_TRIGGER_R => {
                let val = if part.role == ROLE_TRIGGER_L { p.l2 } else { p.r2 };
                let m = about(part.pivot, Quat::from_rotation_x(val * TRIGGER_DEG.to_radians()));
                b.add_mesh(mesh, m, part.color).glow(accent, val * 0.9);
            }
            ROLE_DPAD => {
                let (up, down) = (a.press(btn::UP), a.press(btn::DOWN));
                let (left, right) = (a.press(btn::LEFT), a.press(btn::RIGHT));
                let t = DPAD_TILT_DEG.to_radians();
                let rot = Quat::from_rotation_x((up - down) * t) * Quat::from_rotation_z((left - right) * t);
                let any = up.max(down).max(left).max(right);
                b.add_mesh(mesh, about(part.pivot, rot), part.color).glow(accent, any);
            }
            ROLE_LIGHTBAR => {
                b.add_mesh(mesh, Mat4::IDENTITY, part.color * 0.2).glow(accent, 0.35 + 0.65 * a.energy);
            }
            _ => {
                b.add_mesh(mesh, Mat4::IDENTITY, part.color);
            }
        }
    }
    camera(b.draws, root, a.energy, look)
}

// Manette provisoire (formes simples), si aucun modèle n'est trouvé.
// Repère de la manette : X = droite, Y = haut (face des boutons), Z = vers les gâchettes.
fn build_basic(a: &Animator, p: &PadState, look: &Look) -> Scene {
    let accent = look.accent;
    use MeshId::*;
    let root = a.root();
    let mut b = Builder { root, draws: Vec::with_capacity(48) };
    let id = Quat::IDENTITY;

    // --- Coque ---
    b.add(Sphere, trs(v(0.0, 0.0, 0.15), id, v(1.55, 0.28, 0.7)), BODY).lightning();
    for side in [-1.0f32, 1.0] {
        b.add(
            Sphere,
            trs(v(side * 1.0, -0.04, -0.36), Quat::from_rotation_y(-side * 0.38), v(0.38, 0.27, 0.76)),
            BODY,
        )
        .lightning();
    }

    // --- Pavé tactile + barres lumineuses ---
    b.add(Cube, trs(v(0.0, 0.255, 0.33), id, v(0.95, 0.05, 0.48)), TRIM)
        .glow(accent, a.press(btn::TOUCH) * 0.8);
    for side in [-1.0f32, 1.0] {
        b.add(Cube, trs(v(side * 0.5, 0.25, 0.33), id, v(0.035, 0.04, 0.44)), TRIM)
            .glow(accent, 0.35 + 0.65 * a.energy);
    }

    // --- Sticks ---
    let tilt = STICK_TILT_DEG.to_radians();
    for (sx, sy, bit, px) in [(p.lx, p.ly, btn::L3, -0.42f32), (p.rx, p.ry, btn::R3, 0.42)] {
        let base = v(px, 0.2, -0.18);
        let rot = Quat::from_rotation_x(sy * tilt) * Quat::from_rotation_z(-sx * tilt);
        let press = a.press(bit);
        let mag = (sx * sx + sy * sy).sqrt().min(1.0);
        let shaft_h = 0.17 - press * 0.04;
        b.add(Cylinder, trs(base - v(0.0, 0.03, 0.0), id, v(0.27, 0.035, 0.27)), TRIM)
            .glow(accent, mag * 0.8);
        b.add(Cylinder, trs(base, rot, v(0.075, shaft_h, 0.075)), BUTTON);
        b.add(Cylinder, trs(base + rot * v(0.0, shaft_h, 0.0), rot, v(0.21, 0.07, 0.21)), BUTTON)
            .glow(accent, press * 0.9 + mag * 0.25);
    }

    // --- Boutons de façade ---
    let face = v(1.02, 0.17, 0.18);
    for (bit, off) in [
        (btn::TRIANGLE, v(0.0, 0.0, 0.2)),
        (btn::CROSS, v(0.0, 0.0, -0.2)),
        (btn::SQUARE, v(-0.2, 0.0, 0.0)),
        (btn::CIRCLE, v(0.2, 0.0, 0.0)),
    ] {
        let pr = a.press(bit);
        b.add(Cylinder, trs(face + off - v(0.0, pr * 0.03, 0.0), id, v(0.085, 0.07, 0.085)), BUTTON)
            .glow(accent, pr);
    }

    // --- Croix directionnelle ---
    let dpad = v(-1.0, 0.19, 0.18);
    for (bit, off, size) in [
        (btn::UP, v(0.0, 0.0, 0.12), v(0.1, 0.05, 0.13)),
        (btn::DOWN, v(0.0, 0.0, -0.12), v(0.1, 0.05, 0.13)),
        (btn::LEFT, v(-0.12, 0.0, 0.0), v(0.13, 0.05, 0.1)),
        (btn::RIGHT, v(0.12, 0.0, 0.0), v(0.13, 0.05, 0.1)),
    ] {
        let pr = a.press(bit);
        b.add(Cube, trs(dpad + off - v(0.0, pr * 0.025, 0.0), id, size), BUTTON).glow(accent, pr);
    }

    // --- L1 / R1 ---
    for (bit, side) in [(btn::L1, -1.0f32), (btn::R1, 1.0)] {
        let pr = a.press(bit);
        b.add(Cube, trs(v(side * 0.95, 0.1 - pr * 0.03, 0.62), id, v(0.48, 0.09, 0.12)), BUTTON)
            .glow(accent, pr);
    }

    // --- L2 / R2 (pivotent vers le bas selon la pression) ---
    for (val, side) in [(p.l2, -1.0f32), (p.r2, 1.0)] {
        let pivot = v(side * 0.95, 0.08, 0.68);
        let local = Mat4::from_translation(pivot)
            * Mat4::from_rotation_x(-val * TRIGGER_DEG.to_radians())
            * Mat4::from_translation(v(0.0, -0.06, 0.08))
            * Mat4::from_scale(v(0.42, 0.16, 0.16));
        b.add(Cube, local, BUTTON).glow(accent, val);
    }

    // --- PS, Create, Options, Micro ---
    b.add(Cylinder, trs(v(0.0, 0.2, -0.22), id, v(0.07, 0.05, 0.07)), BUTTON)
        .glow(RED, a.press(btn::PS));
    for (bit, side) in [(btn::CREATE, -1.0f32), (btn::OPTIONS, 1.0)] {
        b.add(Cube, trs(v(side * 0.6, 0.24, 0.55), id, v(0.06, 0.04, 0.12)), BUTTON)
            .glow(accent, a.press(bit));
    }
    b.add(Cube, trs(v(0.0, 0.19, -0.38), id, v(0.14, 0.03, 0.05)), BUTTON)
        .glow(RED, a.press(btn::MUTE));

    camera(b.draws, root, a.energy, look)
}
