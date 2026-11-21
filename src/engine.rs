// Le moteur : correction de dérive, courbes, remappage, puis sortie en manette Xbox 360 virtuelle.

use std::time::{Duration, Instant};

use vigem_client::{Client, TargetId, XButtons, XGamepad, Xbox360Wired};

use crate::pad::{btn, PadState};
use crate::state::Profile;

// Boutons XInput
pub const X_UP: u16 = 0x0001;
pub const X_DOWN: u16 = 0x0002;
pub const X_LEFT: u16 = 0x0004;
pub const X_RIGHT: u16 = 0x0008;
pub const X_START: u16 = 0x0010;
pub const X_BACK: u16 = 0x0020;
pub const X_LTHUMB: u16 = 0x0040;
pub const X_RTHUMB: u16 = 0x0080;
pub const X_LB: u16 = 0x0100;
pub const X_RB: u16 = 0x0200;
pub const X_GUIDE: u16 = 0x0400;
pub const X_A: u16 = 0x1000;
pub const X_B: u16 = 0x2000;
pub const X_X: u16 = 0x4000;
pub const X_Y: u16 = 0x8000;

/// Boutons DualSense remappables, dans l'ordre utilisé par `Profile::remap`
/// (même ordre dans ui/app.js), avec leur cible Xbox par défaut.
pub const SOURCES: [(u32, u16); 17] = [
    (btn::CROSS, X_A),
    (btn::CIRCLE, X_B),
    (btn::SQUARE, X_X),
    (btn::TRIANGLE, X_Y),
    (btn::L1, X_LB),
    (btn::R1, X_RB),
    (btn::L3, X_LTHUMB),
    (btn::R3, X_RTHUMB),
    (btn::CREATE, X_BACK),
    (btn::TOUCH, X_BACK),
    (btn::OPTIONS, X_START),
    (btn::PS, X_GUIDE),
    (btn::UP, X_UP),
    (btn::DOWN, X_DOWN),
    (btn::LEFT, X_LEFT),
    (btn::RIGHT, X_RIGHT),
    (btn::MUTE, 0),
];
pub const SOURCE_COUNT: usize = SOURCES.len();

pub fn default_remap() -> Vec<u16> {
    SOURCES.iter().map(|&(_, x)| x).collect()
}

/// Applique zones mortes, courbes et portée max. Entrée brute → sortie propre.
pub fn process(raw: &PadState, p: &Profile) -> PadState {
    let mut o = raw.clone();
    (o.lx, o.ly) = stick(raw.lx, raw.ly, p.left_deadzone, p.left_outer, p.left_anti_deadzone, p.left_curve);
    (o.rx, o.ry) = stick(raw.rx, raw.ry, p.right_deadzone, p.right_outer, p.right_anti_deadzone, p.right_curve);
    o.l2 = trigger(raw.l2, p.trigger_deadzone, p.trigger_curve, p.hair_trigger);
    o.r2 = trigger(raw.r2, p.trigger_deadzone, p.trigger_curve, p.hair_trigger);
    o
}

/// Zone morte ronde avec remise à l'échelle (pas de « saut »), courbe, puis anti-zone morte.
fn stick(x: f32, y: f32, dz: f32, outer: f32, anti: f32, curve: f32) -> (f32, f32) {
    let m = (x * x + y * y).sqrt();
    if m <= dz || m < 1e-6 {
        return (0.0, 0.0);
    }
    let mut t = ((m - dz) / (outer - dz).max(0.01)).min(1.0);
    t = t.powf(curve.max(0.2));
    t = anti + (1.0 - anti) * t;
    (x / m * t, y / m * t)
}

fn trigger(v: f32, dz: f32, curve: f32, hair: bool) -> f32 {
    if v <= dz {
        0.0
    } else if hair {
        1.0
    } else {
        ((v - dz) / (1.0 - dz).max(0.01)).min(1.0).powf(curve.max(0.2))
    }
}

fn to_xbox(p: &PadState, remap: &[u16]) -> XGamepad {
    let mut raw = 0u16;
    for (i, &(src, _)) in SOURCES.iter().enumerate() {
        if p.buttons & src != 0 {
            raw |= remap.get(i).copied().unwrap_or(0);
        }
    }
    let axis = |v: f32| (v.clamp(-1.0, 1.0) * 32767.0) as i16;
    let trig = |v: f32| (v.clamp(0.0, 1.0) * 255.0) as u8;
    XGamepad {
        buttons: XButtons { raw },
        left_trigger: trig(p.l2),
        right_trigger: trig(p.r2),
        thumb_lx: axis(p.lx),
        thumb_ly: axis(p.ly),
        thumb_rx: axis(p.rx),
        thumb_ry: axis(p.ry),
    }
}

/// Gère la manette Xbox virtuelle : la branche quand il faut, la débranche sinon.
pub struct Emulator {
    target: Option<Xbox360Wired<Client>>,
    error: Option<String>,
    last_try: Option<Instant>,
}

impl Emulator {
    pub fn new() -> Self {
        Self { target: None, error: None, last_try: None }
    }

    /// `want` : l'émulation est demandée. `pad` : état corrigé (None si pas de manette).
    /// Renvoie (active, erreur).
    pub fn update(&mut self, want: bool, pad: Option<&PadState>, remap: &[u16]) -> (bool, Option<String>) {
        match (want, pad) {
            (true, Some(p)) => {
                if self.target.is_none() {
                    let retry = self.last_try.map_or(true, |t| t.elapsed() > Duration::from_secs(3));
                    if retry {
                        self.last_try = Some(Instant::now());
                        match plug() {
                            Ok(t) => {
                                println!("Manette Xbox 360 virtuelle branchée");
                                self.target = Some(t);
                                self.error = None;
                            }
                            Err(e) => {
                                if self.error.as_deref() != Some(e.as_str()) {
                                    eprintln!("Émulation : {e}");
                                }
                                self.error = Some(e);
                            }
                        }
                    }
                }
                if let Some(t) = self.target.as_mut() {
                    if let Err(e) = t.update(&to_xbox(p, remap)) {
                        self.error = Some(format!("La manette virtuelle a été perdue ({e:?})."));
                        self.target = None;
                    }
                }
            }
            _ => {
                if self.target.take().is_some() {
                    println!("Manette Xbox 360 virtuelle débranchée");
                }
                if !want {
                    self.error = None;
                    self.last_try = None;
                }
            }
        }
        (self.target.is_some(), self.error.clone())
    }
}

fn plug() -> Result<Xbox360Wired<Client>, String> {
    let client = Client::connect()
        .map_err(|e| format!("Pilote ViGEmBus introuvable ({e:?}). Installe « ViGEm Bus Driver »."))?;
    let mut target = Xbox360Wired::new(client, TargetId::XBOX360_WIRED);
    target.plugin().map_err(|e| format!("Branchement de la manette virtuelle refusé ({e:?})."))?;
    target.wait_ready().map_err(|e| format!("La manette virtuelle ne répond pas ({e:?})."))?;
    Ok(target)
}
