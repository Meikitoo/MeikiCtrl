// Lecture de la DualSense / DualSense Edge en HID brut, sur un thread dédié.
// La lecture est bloquante : 0 % de CPU quand la manette n'envoie rien.

use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use hidapi::HidApi;

use crate::engine::{self, Emulator};
use crate::output::{self, OutputCtl};
use crate::stats;
use crate::state::Shared;

const VID_SONY: u16 = 0x054C;
const PID_DUALSENSE: u16 = 0x0CE6;
const PID_DUALSENSE_EDGE: u16 = 0x0DF2;

// Résolution des capteurs (d'après le pilote Linux hid-playstation)
const GYRO_PER_DEG_S: f32 = 1024.0;
const ACC_PER_G: f32 = 8192.0;

/// Bits des boutons dans `PadState::buttons`.
#[allow(dead_code)]
pub mod btn {
    pub const SQUARE: u32 = 1 << 0;
    pub const CROSS: u32 = 1 << 1;
    pub const CIRCLE: u32 = 1 << 2;
    pub const TRIANGLE: u32 = 1 << 3;
    pub const L1: u32 = 1 << 4;
    pub const R1: u32 = 1 << 5;
    pub const L2: u32 = 1 << 6;
    pub const R2: u32 = 1 << 7;
    pub const CREATE: u32 = 1 << 8;
    pub const OPTIONS: u32 = 1 << 9;
    pub const L3: u32 = 1 << 10;
    pub const R3: u32 = 1 << 11;
    pub const PS: u32 = 1 << 12;
    pub const TOUCH: u32 = 1 << 13;
    pub const MUTE: u32 = 1 << 14;
    pub const UP: u32 = 1 << 15;
    pub const DOWN: u32 = 1 << 16;
    pub const LEFT: u32 = 1 << 17;
    pub const RIGHT: u32 = 1 << 18;
}

#[derive(Clone, Default, Debug)]
pub struct PadState {
    pub connected: bool,
    pub bluetooth: bool,
    pub edge: bool,
    /// Manette Xbox (lue via XInput) au lieu d'une DualSense.
    pub xbox: bool,
    /// Sticks de -1 à 1 (haut = +1 sur l'axe Y).
    pub lx: f32,
    pub ly: f32,
    pub rx: f32,
    pub ry: f32,
    /// Gâchettes de 0 à 1.
    pub l2: f32,
    pub r2: f32,
    pub buttons: u32,
    /// Gyroscope en degrés par seconde.
    pub gyro: [f32; 3],
    /// Accéléromètre en g.
    pub accel: [f32; 3],
    /// Batterie en % (approximatif, par paliers de 10).
    pub battery: u8,
    pub charging: bool,
}

pub fn spawn(shared: Arc<Shared>) {
    std::thread::Builder::new()
        .name("manette".into())
        .spawn(move || run(shared))
        .expect("impossible de lancer le thread manette");
}

fn run(shared: Arc<Shared>) {
    // Priorité haute : même quand le jeu sature le processeur, la manette passe en premier.
    unsafe {
        use windows::Win32::System::Threading::{GetCurrentThread, SetThreadPriority, THREAD_PRIORITY_HIGHEST};
        let _ = SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_HIGHEST);
    }
    let mut last_sig = 0u64;
    let mut api = match HidApi::new() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("hidapi : {e}");
            return;
        }
    };
    let mut emu = Emulator::new();

    loop {
        let _ = api.refresh_devices();
        let found = api
            .device_list()
            .find(|d| {
                d.vendor_id() == VID_SONY
                    && (d.product_id() == PID_DUALSENSE || d.product_id() == PID_DUALSENSE_EDGE)
            })
            .map(|d| (d.path().to_owned(), d.product_id()));

        let Some((path, pid)) = found else {
            // Pas de DualSense : on regarde s'il y a une manette Xbox (pendant ~1 s, puis on revérifie).
            let xbox_active = shared.pad.lock().map(|g| g.xbox && g.connected).unwrap_or(false);
            if !xbox_active {
                set_disconnected(&shared, &mut emu);
            }
            xbox_poll(&shared, Duration::from_secs(1));
            continue;
        };

        let dev = match api.open_path(&path) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("Ouverture de la manette impossible : {e}");
                std::thread::sleep(Duration::from_secs(1));
                continue;
            }
        };

        // En Bluetooth, lire le rapport 0x05 fait passer la manette en mode complet
        // (rapport 0x31 avec gyro). Sans effet en USB.
        let mut feat = [0u8; 64];
        feat[0] = 0x05;
        let _ = dev.get_feature_report(&mut feat);

        if let Ok(mut i) = shared.hid_instance.lock() {
            *i = instance_id(&path.to_string_lossy());
        }

        let edge = pid == PID_DUALSENSE_EDGE;
        println!("Manette connectée : {}", if edge { "DualSense Edge" } else { "DualSense" });

        let mut raw = PadState { connected: true, edge, ..Default::default() };
        let mut buf = [0u8; 128];
        let mut outctl = OutputCtl::new();
        let mut last_out = 0.0f64;
        let mut got_report = false;
        loop {
            // Lumière et gâchettes adaptatives (au plus ~60 fois par seconde),
            // après le premier rapport (pour savoir si c'est de l'USB ou du Bluetooth).
            let t = stats::now();
            if got_report && t - last_out >= 0.016 {
                last_out = t;
                drive_outputs(&shared, &dev, &mut outctl, &raw, t);
            }
            match dev.read_timeout(&mut buf, 250) {
                Ok(0) => {
                    // Pas de nouveau rapport : on garde l'émulation à jour (activation/désactivation).
                    let want = shared.emulation_wanted();
                    let out = shared.pad.lock().map(|g| g.clone()).unwrap_or_default();
                    let profile = shared.profile();
                    let (active, err) = emu.update(want, Some(&out), &profile.remap);
                    if let Ok(mut l) = shared.live.lock() {
                        l.emulation_active = active;
                        l.emulation_error = err;
                    }
                }
                Ok(n) => {
                    if parse(&buf[..n], &mut raw) {
                        got_report = true;
                        let (hz, avg_ms, worst_ms) = match shared.stats.lock() {
                            Ok(mut tr) => {
                                tr.on_report(&raw, stats::now(), true);
                                (tr.hz, tr.avg_ms, tr.worst_ms)
                            }
                            Err(_) => (0.0, 0.0, 0.0),
                        };
                        let profile = shared.profile();
                        let out = engine::process(&raw, &profile);
                        let want = shared.emulation_wanted();
                        let (active, err) = emu.update(want, Some(&out), &profile.remap);
                        if let Ok(mut l) = shared.live.lock() {
                            l.connected = true;
                            l.edge = raw.edge;
                            l.bluetooth = raw.bluetooth;
                            l.battery = raw.battery;
                            l.charging = raw.charging;
                            l.raw_l = [raw.lx, raw.ly];
                            l.raw_r = [raw.rx, raw.ry];
                            l.out_l = [out.lx, out.ly];
                            l.out_r = [out.rx, out.ry];
                            l.l2 = out.l2;
                            l.r2 = out.r2;
                            l.buttons = out.buttons;
                            l.emulation_active = active;
                            l.emulation_error = err;
                            l.hz = hz;
                            l.avg_ms = avg_ms;
                            l.worst_ms = worst_ms;
                        }
                        let sig = input_signature(&out);
                        if sig != last_sig {
                            last_sig = sig;
                            shared.input_seq.fetch_add(1, Ordering::Relaxed);
                        }
                        if let Ok(mut g) = shared.pad.lock() {
                            *g = out;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Manette perdue : {e}");
                    break;
                }
            }
        }
        if let Ok(mut tr) = shared.stats.lock() {
            tr.disconnected();
        }
        set_disconnected(&shared, &mut emu);
        std::thread::sleep(Duration::from_millis(500));
    }
}

/// Lit une manette Xbox via XInput pendant `duration` (250 fois par seconde).
/// Sert à l'overlay et à l'interface ; pas d'émulation (c'est déjà une manette Xbox).
fn xbox_poll(shared: &Arc<Shared>, duration: Duration) {
    use windows::Win32::UI::Input::XboxController::{XInputGetState, XINPUT_STATE};
    let start = std::time::Instant::now();
    let mut slot: Option<u32> = None;
    let mut last_sig = 0u64;
    let mut announced = false;
    while start.elapsed() < duration || slot.is_some() {
        let mut state = XINPUT_STATE::default();
        // Cherche la première manette branchée (emplacements 0 à 3).
        let found = match slot {
            Some(i) if unsafe { XInputGetState(i, &mut state) } == 0 => Some(i),
            _ => (0..4u32).find(|&i| unsafe { XInputGetState(i, &mut state) } == 0),
        };
        slot = found;
        let Some(_) = slot else {
            if announced {
                println!("Manette Xbox déconnectée");
                if let Ok(mut g) = shared.pad.lock() {
                    *g = PadState::default();
                }
                if let Ok(mut l) = shared.live.lock() {
                    *l = Default::default();
                }
            }
            // Personne : on rend la main pour revérifier la DualSense.
            if start.elapsed() >= duration {
                return;
            }
            std::thread::sleep(Duration::from_millis(100));
            continue;
        };
        if !announced {
            println!("Manette connectée : Xbox (XInput)");
            announced = true;
        }
        let g = state.Gamepad;
        let wb: u16 = unsafe { std::mem::transmute_copy(&g.wButtons) };
        let map = [
            (0x1000u16, btn::CROSS),
            (0x2000, btn::CIRCLE),
            (0x4000, btn::SQUARE),
            (0x8000, btn::TRIANGLE),
            (0x0100, btn::L1),
            (0x0200, btn::R1),
            (0x0040, btn::L3),
            (0x0080, btn::R3),
            (0x0020, btn::CREATE),
            (0x0010, btn::OPTIONS),
            (0x0001, btn::UP),
            (0x0002, btn::DOWN),
            (0x0004, btn::LEFT),
            (0x0008, btn::RIGHT),
        ];
        let mut buttons = 0u32;
        for (x, d) in map {
            if wb & x != 0 {
                buttons |= d;
            }
        }
        let axis = |v: i16| (v as f32 / 32767.0).clamp(-1.0, 1.0);
        let raw = PadState {
            connected: true,
            xbox: true,
            lx: axis(g.sThumbLX),
            ly: axis(g.sThumbLY),
            rx: axis(g.sThumbRX),
            ry: axis(g.sThumbRY),
            l2: g.bLeftTrigger as f32 / 255.0,
            r2: g.bRightTrigger as f32 / 255.0,
            buttons,
            ..Default::default()
        };
        if let Ok(mut tr) = shared.stats.lock() {
            tr.on_report(&raw, stats::now(), false);
        }
        let profile = shared.profile();
        let out = engine::process(&raw, &profile);
        let sig = input_signature(&out);
        if sig != last_sig {
            last_sig = sig;
            shared.input_seq.fetch_add(1, Ordering::Relaxed);
        }
        if let Ok(mut l) = shared.live.lock() {
            l.connected = true;
            l.xbox = true;
            l.edge = false;
            l.bluetooth = false;
            l.raw_l = [raw.lx, raw.ly];
            l.raw_r = [raw.rx, raw.ry];
            l.out_l = [out.lx, out.ly];
            l.out_r = [out.rx, out.ry];
            l.l2 = out.l2;
            l.r2 = out.r2;
            l.buttons = out.buttons;
            l.emulation_active = false;
            l.emulation_error = None;
        }
        if let Ok(mut p) = shared.pad.lock() {
            *p = out;
        }
        std::thread::sleep(Duration::from_millis(4));
        // Une DualSense branchée entre-temps reprend la main (vérifié toutes les ~2 s).
        if start.elapsed() > Duration::from_secs(2) {
            return;
        }
    }
}

/// Barre lumineuse, LED joueur et gâchettes adaptatives selon les réglages.
/// Rien n'est envoyé pendant un jeu qui pilote la manette lui-même (ex. COD) ni en quittant.
fn drive_outputs(shared: &Arc<Shared>, dev: &hidapi::HidDevice, ctl: &mut OutputCtl, raw: &PadState, t: f64) {
    if shared.quitting.load(Ordering::Relaxed) {
        ctl.release(dev, raw.bluetooth);
        shared.released.store(true, Ordering::Relaxed);
        return;
    }
    let allowed = !shared.emulation_blocked.load(Ordering::Relaxed);
    // Bouton qui déclenche le flash : L2/R2 au-delà de 60 %, sinon le bouton lui-même.
    let flash_button = shared.settings.lock().map(|s| s.lightbar.flash_button).unwrap_or(7);
    let fire = match flash_button {
        6 => raw.l2 > 0.6,
        7 => raw.r2 > 0.6,
        b => raw.buttons & (1u32 << b) != 0,
    };
    let wanted = if allowed {
        let profile = shared.profile();
        shared
            .settings
            .lock()
            .ok()
            .map(|s| output::wanted(&s.lightbar, &profile, s.adaptive_triggers, raw, t, ctl.flash))
    } else {
        None
    };
    ctl.update(dev, raw.bluetooth, allowed, wanted.as_ref(), fire, t);
}

fn set_disconnected(shared: &Arc<Shared>, emu: &mut Emulator) {
    let want = shared.emulation_wanted();
    let (active, err) = emu.update(want, None, &[]);
    if let Ok(mut i) = shared.hid_instance.lock() {
        *i = None;
    }
    if let Ok(mut g) = shared.pad.lock() {
        if g.connected {
            println!("Manette déconnectée");
        }
        *g = PadState::default();
    }
    if let Ok(mut l) = shared.live.lock() {
        *l = Default::default();
        l.emulation_active = active;
        l.emulation_error = err;
    }
}

/// Empreinte des entrées (boutons + sticks/gâchettes arrondis) pour détecter un changement.
fn input_signature(p: &PadState) -> u64 {
    let q = |v: f32| ((v * 64.0).round() as i64 & 0xFF) as u64;
    (p.buttons as u64)
        ^ (q(p.lx) << 20)
        ^ (q(p.ly) << 28)
        ^ (q(p.rx) << 36)
        ^ (q(p.ry) << 44)
        ^ (q(p.l2) << 52)
        ^ (q(p.r2) << 58)
}

/// Chemin HID Windows → identifiant d'instance (pour HidHide).
/// `\\?\HID#VID_054C&PID_0CE6&MI_03#7&abc&0&0000#{guid}` → `HID\VID_054C&PID_0CE6&MI_03\7&abc&0&0000`
fn instance_id(path: &str) -> Option<String> {
    let p = path.strip_prefix(r"\\?\").unwrap_or(path);
    let end = p.rfind("#{")?;
    Some(p[..end].replace('#', "\\").to_uppercase())
}

/// Décode un rapport d'entrée. USB : 0x01 (données dès l'octet 1).
/// Bluetooth complet : 0x31 (données dès l'octet 2).
fn parse(buf: &[u8], st: &mut PadState) -> bool {
    let d = match buf.first() {
        Some(0x01) if buf.len() >= 28 => {
            st.bluetooth = false;
            &buf[1..]
        }
        Some(0x31) if buf.len() >= 29 => {
            st.bluetooth = true;
            &buf[2..]
        }
        _ => return false,
    };

    // Valeurs brutes : la zone morte est appliquée ensuite par le moteur (réglable).
    let axis = |v: u8| ((v as f32 - 127.5) / 127.5).clamp(-1.0, 1.0);
    st.lx = axis(d[0]);
    st.ly = -axis(d[1]);
    st.rx = axis(d[2]);
    st.ry = -axis(d[3]);
    st.l2 = d[4] as f32 / 255.0;
    st.r2 = d[5] as f32 / 255.0;

    let b0 = d[7];
    let b1 = d[8];
    let b2 = d[9];
    let mut buttons = ((b0 >> 4) & 0x0F) as u32 | (b1 as u32) << 4 | ((b2 & 0x07) as u32) << 12;
    buttons |= match b0 & 0x0F {
        0 => btn::UP,
        1 => btn::UP | btn::RIGHT,
        2 => btn::RIGHT,
        3 => btn::DOWN | btn::RIGHT,
        4 => btn::DOWN,
        5 => btn::DOWN | btn::LEFT,
        6 => btn::LEFT,
        7 => btn::UP | btn::LEFT,
        _ => 0,
    };
    st.buttons = buttons;

    let i16_at = |o: usize| i16::from_le_bytes([d[o], d[o + 1]]) as f32;
    st.gyro = [
        i16_at(15) / GYRO_PER_DEG_S,
        i16_at(17) / GYRO_PER_DEG_S,
        i16_at(19) / GYRO_PER_DEG_S,
    ];
    st.accel = [i16_at(21) / ACC_PER_G, i16_at(23) / ACC_PER_G, i16_at(25) / ACC_PER_G];

    // Octet d'état : batterie (4 bits bas) et charge (4 bits hauts).
    if let Some(&status) = d.get(52) {
        let level = status & 0x0F;
        let charge = status >> 4;
        st.battery = if charge == 2 { 100 } else { (level as u32 * 10 + 5).min(100) as u8 };
        st.charging = charge == 1;
    }
    true
}
