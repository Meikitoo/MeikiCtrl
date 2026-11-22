// Sorties de la DualSense : barre lumineuse, LED joueur et gâchettes adaptatives.
// Pas de vibrations (volontairement). Rapport de sortie d'après le pilote Linux hid-playstation :
//  - USB : rapport 0x02 (48 octets)
//  - Bluetooth : rapport 0x31 (78 octets, avec CRC32)

use hidapi::HidDevice;

use crate::pad::PadState;
use crate::state::{LightCfg, Profile};

/// Couleurs « joueur » (comme sur PS5) : bleu, rouge, vert, rose.
pub const PLAYER_COLORS: [[f32; 3]; 4] = [[0.0, 0.25, 1.0], [1.0, 0.05, 0.1], [0.05, 1.0, 0.2], [1.0, 0.2, 0.7]];
/// LED blanches sous le pavé tactile (5 LED) pour les joueurs 1 à 4.
const PLAYER_LEDS: [u8; 4] = [0b00100, 0b01010, 0b10101, 0b11011];

/// Effet d'une gâchette adaptative (11 octets).
/// 0 = aucun, 1 = résistance, 2 = clic d'arme (point dur puis relâchement).
pub fn trigger_fx(mode: u8, strength: f32) -> [u8; 11] {
    let s = (strength.clamp(0.05, 1.0) * 255.0) as u8;
    let mut fx = [0u8; 11];
    match mode {
        1 => {
            fx[0] = 0x01; // résistance continue
            fx[1] = 0x10; // à partir du début de course
            fx[2] = s;
        }
        2 => {
            fx[0] = 0x02; // « arme » : point dur entre début et fin
            fx[1] = 0x50;
            fx[2] = 0xA0;
            fx[3] = s;
        }
        _ => fx[0] = 0x05, // libre
    }
    fx
}

/// Teinte (0 à 1) → couleur saturée.
fn hue(h: f32) -> [f32; 3] {
    let h = h.rem_euclid(1.0) * 6.0;
    let x = 1.0 - ((h % 2.0) - 1.0).abs();
    match h as u32 {
        0 => [1.0, x, 0.0],
        1 => [x, 1.0, 0.0],
        2 => [0.0, 1.0, x],
        3 => [0.0, x, 1.0],
        4 => [x, 0.0, 1.0],
        _ => [1.0, 0.0, x],
    }
}

fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
}

/// Couleur de la batterie : vert → jaune → rouge (clignote quand elle est presque vide).
fn battery_color(p: &PadState, t: f64) -> ([f32; 3], f32) {
    let b = p.battery as f32;
    let green = [0.1, 1.0, 0.25];
    let yellow = [1.0, 0.75, 0.0];
    let red = [1.0, 0.03, 0.03];
    let c = if b >= 60.0 {
        green
    } else if b >= 30.0 {
        mix(yellow, green, (b - 30.0) / 30.0)
    } else {
        mix(red, yellow, (b / 30.0).max(0.0))
    };
    let pulse = if p.charging {
        0.45 + 0.55 * (0.5 + 0.5 * (t * 2.0).sin() as f32)
    } else if b <= 15.0 {
        if (t * 2.0).fract() < 0.5 { 1.0 } else { 0.15 }
    } else {
        1.0
    };
    (c, pulse)
}

/// Ce qu'on veut envoyer à la manette.
#[derive(Clone, PartialEq)]
pub struct Wanted {
    pub light: Option<[u8; 3]>,
    pub player: u8,
    pub trig_l: [u8; 11],
    pub trig_r: [u8; 11],
}

impl Wanted {
    /// Rien à piloter : on laisse la manette tranquille.
    fn idle(&self) -> bool {
        self.light.is_none() && self.player == 0 && self.trig_l[0] == 0x05 && self.trig_r[0] == 0x05
    }
}

pub fn wanted(cfg: &LightCfg, prof: &Profile, triggers_on: bool, p: &PadState, t: f64, flash: f32) -> Wanted {
    let player_idx = cfg.player.clamp(1, 4) as usize - 1;
    // Vitesse : un tour de couleurs en 12 s (lent) à 1 s (rapide) ; respiration de 8 s à 1,5 s.
    let sp = cfg.speed.clamp(0.05, 1.0) as f64;
    let rgb_period = 12.0 - 11.0 * sp;
    let breath_period = 8.0 - 6.5 * sp;
    let (base, k) = match cfg.mode {
        1 => (cfg.color, 1.0),
        2 => battery_color(p, t),
        3 => (prof.light_color, 1.0),
        4 => (PLAYER_COLORS[player_idx], 1.0),
        5 => (cfg.color, 0.2 + 0.8 * (0.5 + 0.5 * (t * std::f64::consts::TAU / breath_period).sin() as f32)),
        6 => (hue((t / rgb_period).fract() as f32), 1.0),
        _ => ([0.0; 3], 0.0),
    };
    let light = if cfg.mode == 0 {
        None
    } else {
        let mut c = base.map(|x| x * k);
        if cfg.flash_on_fire && flash > 0.01 {
            c = mix(c, [1.0, 1.0, 1.0], flash);
        }
        let br = cfg.brightness.clamp(0.05, 1.0);
        Some(c.map(|x| (x.clamp(0.0, 1.0) * br * 255.0) as u8))
    };
    let player = if cfg.player >= 1 { PLAYER_LEDS[player_idx] } else { 0 };
    let (ml, mr) = if triggers_on { (prof.trigger_l, prof.trigger_r) } else { (0, 0) };
    Wanted {
        light,
        player,
        trig_l: trigger_fx(ml, prof.trigger_strength),
        trig_r: trigger_fx(mr, prof.trigger_strength),
    }
}

// ---------- Construction et envoi des rapports ----------

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 { (crc >> 1) ^ 0xEDB8_8320 } else { crc >> 1 };
        }
    }
    !crc
}

enum Kind<'a> {
    /// Éteint l'animation bleue du firmware pour pouvoir piloter la lumière.
    Setup,
    /// Rend la main : gâchettes libres, LED gérées par la manette.
    Release,
    Set(&'a Wanted),
}

fn build(kind: Kind, bt: bool, seq: &mut u8) -> Vec<u8> {
    let mut c = [0u8; 47];
    match kind {
        Kind::Setup => {
            c[38] = 0x02; // autoriser le réglage de la barre
            c[41] = 0x02; // extinction de l'animation de démarrage
        }
        Kind::Release => {
            c[0] = 0x0C; // gâchettes droite + gauche
            c[10] = 0x05;
            c[21] = 0x05;
            c[1] = 0x08; // rendre les LED au firmware
        }
        Kind::Set(w) => {
            c[0] = 0x0C;
            c[10..21].copy_from_slice(&w.trig_r);
            c[21..32].copy_from_slice(&w.trig_l);
            if let Some(rgb) = w.light {
                c[1] |= 0x04;
                c[44..47].copy_from_slice(&rgb);
            }
            c[1] |= 0x10; // LED joueur (0 = éteintes)
            c[43] = w.player;
        }
    }
    if bt {
        let mut buf = vec![0u8; 78];
        buf[0] = 0x31;
        buf[1] = *seq << 4;
        buf[2] = 0x10;
        buf[3..50].copy_from_slice(&c);
        *seq = (*seq + 1) & 0x0F;
        let mut data = Vec::with_capacity(75);
        data.push(0xA2);
        data.extend_from_slice(&buf[..74]);
        buf[74..78].copy_from_slice(&crc32(&data).to_le_bytes());
        buf
    } else {
        let mut buf = vec![0u8; 48];
        buf[0] = 0x02;
        buf[1..48].copy_from_slice(&c);
        buf
    }
}

/// Pilote les sorties d'une manette connectée. Envoie seulement quand ça change
/// (au plus ~60 fois par seconde), et une piqûre de rappel toutes les 3 s.
pub struct OutputCtl {
    seq: u8,
    setup_done: bool,
    controlling: bool,
    last: Option<Vec<u8>>,
    last_send: f64,
    pub flash: f32,
    prev_fire: bool,
    last_t: f64,
}

impl OutputCtl {
    pub fn new() -> Self {
        Self { seq: 0, setup_done: false, controlling: false, last: None, last_send: -10.0, flash: 0.0, prev_fire: false, last_t: 0.0 }
    }

    /// `allowed` : false quand un jeu pilote la manette lui-même (ex. COD) ou en quittant.
    pub fn update(&mut self, dev: &HidDevice, bt: bool, allowed: bool, w: Option<&Wanted>, fire: bool, t: f64) {
        // Flash au tir : bouton de tir enfoncé → éclair blanc qui retombe vite.
        let dt = (t - self.last_t).clamp(0.0, 0.1) as f32;
        self.last_t = t;
        if fire && !self.prev_fire {
            self.flash = 1.0;
        }
        self.prev_fire = fire;
        self.flash *= (-dt * 9.0).exp();

        let want = w.filter(|w| allowed && !w.idle());
        let Some(w) = want else {
            if self.controlling {
                let _ = dev.write(&build(Kind::Release, bt, &mut self.seq));
                self.controlling = false;
                self.last = None;
                self.setup_done = false;
            }
            return;
        };
        if t - self.last_send < 0.016 {
            return;
        }
        if !self.setup_done {
            let _ = dev.write(&build(Kind::Setup, bt, &mut self.seq));
            self.setup_done = true;
            self.last_send = t;
            return;
        }
        // Comparaison sans le compteur Bluetooth (octet 1) ni le CRC.
        let mut seq_probe = self.seq;
        let report = build(Kind::Set(w), bt, &mut seq_probe);
        let key: Vec<u8> = report.iter().enumerate().filter(|(i, _)| !bt || (*i != 1 && *i < 74)).map(|(_, b)| *b).collect();
        let changed = self.last.as_ref() != Some(&key);
        if changed || t - self.last_send > 3.0 {
            let report = build(Kind::Set(w), bt, &mut self.seq);
            if dev.write(&report).is_ok() {
                self.controlling = true;
                self.last = Some(key);
                self.last_send = t;
            }
        }
    }

    /// En quittant : gâchettes libres et LED rendues à la manette.
    pub fn release(&mut self, dev: &HidDevice, bt: bool) {
        if self.controlling {
            let _ = dev.write(&build(Kind::Release, bt, &mut self.seq));
            self.controlling = false;
        }
    }
}
