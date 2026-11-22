// Santé de la manette et sessions de jeu :
//  - fréquence de la manette (rapports par seconde) et régularité ;
//  - dérive au repos des sticks, jour par jour ;
//  - rebonds de boutons (un appui qui « double » tout seul) ;
//  - compteur d'appuis par bouton ;
//  - sessions de jeu (durée, jeu, appuis), bilan à la fin et rappel de pause.
// Enregistré dans %APPDATA%\MeikiCtrl\stats.json (une fois par minute au plus).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::pad::PadState;
use crate::state::Shared;
use crate::toast;

pub const NBTN: usize = 19;
/// Deux appuis séparés par moins que ça = rebond mécanique, pas un humain.
const BOUNCE_S: f64 = 0.015;
/// Session terminée après ce temps sans rien toucher.
const SESSION_IDLE_S: f64 = 300.0;
/// Sessions plus courtes ignorées.
const SESSION_MIN_S: f64 = 120.0;
/// L2/R2 (bits 6 et 7) basculent au seuil analogique : pas de détection de rebond dessus.
const NO_BOUNCE: u32 = (1 << 6) | (1 << 7);

static START: OnceLock<Instant> = OnceLock::new();

/// Secondes depuis le lancement de MeikiCtrl.
pub fn now() -> f64 {
    START.get_or_init(Instant::now).elapsed().as_secs_f64()
}

pub fn unix_now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct DriftDay {
    /// Jour (nombre de jours depuis le 1er janvier 1970).
    pub day: u32,
    pub left: f32,
    pub right: f32,
    pub n_left: u32,
    pub n_right: u32,
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SessionRec {
    /// Début (secondes Unix).
    pub start: u64,
    pub minutes: f32,
    pub game: String,
    pub presses: u64,
    /// Boutons les plus utilisés : (bit, appuis).
    pub top: Vec<(u8, u64)>,
    pub hz: f32,
    pub bounces: u32,
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Store {
    pub presses: Vec<u64>,
    pub bounces: Vec<u32>,
    pub drift: Vec<DriftDay>,
    pub sessions: Vec<SessionRec>,
    pub play_minutes: f64,
}

impl Store {
    fn path() -> Option<PathBuf> {
        std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join("MeikiCtrl").join("stats.json"))
    }

    fn load() -> Self {
        let mut s: Store = Self::path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        s.presses.resize(NBTN, 0);
        s.bounces.resize(NBTN, 0);
        s
    }

    fn save(&self) {
        if let Some(p) = Self::path() {
            if let Some(dir) = p.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            if let Ok(json) = serde_json::to_string(self) {
                let _ = std::fs::write(p, json);
            }
        }
    }
}

struct Current {
    start_unix: u64,
    start: f64,
    presses: [u64; NBTN],
    bounces: u32,
    games: HashMap<String, u32>,
    hz_sum: f64,
    hz_n: u32,
    reminders: u32,
}

/// Session en cours, pour l'interface.
#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveSession {
    pub minutes: f32,
    pub presses: u64,
    pub game: String,
}

struct Stick {
    anchor: [f32; 2],
    since: f64,
    last_sample: f64,
}

pub struct Tracker {
    pub store: Store,
    dirty: bool,
    prev_buttons: u32,
    release_at: [f64; NBTN],
    // fréquence
    win_start: f64,
    win_count: u32,
    win_sum: f64,
    win_max: f64,
    last_report: f64,
    pub hz: f32,
    pub avg_ms: f32,
    pub worst_ms: f32,
    // dérive
    sticks: [Stick; 2],
    // session
    cur: Option<Current>,
    last_input: f64,
}

impl Tracker {
    pub fn new() -> Self {
        let st = || Stick { anchor: [0.0; 2], since: 0.0, last_sample: 0.0 };
        Self {
            store: Store::load(),
            dirty: false,
            prev_buttons: 0,
            release_at: [-1.0; NBTN],
            win_start: 0.0,
            win_count: 0,
            win_sum: 0.0,
            win_max: 0.0,
            last_report: 0.0,
            hz: 0.0,
            avg_ms: 0.0,
            worst_ms: 0.0,
            sticks: [st(), st()],
            cur: None,
            last_input: 0.0,
        }
    }

    pub fn reset(&mut self) {
        self.store = Store::default();
        self.store.presses.resize(NBTN, 0);
        self.store.bounces.resize(NBTN, 0);
        self.dirty = true;
    }

    /// Manette débranchée : plus de fréquence à afficher.
    pub fn disconnected(&mut self) {
        self.hz = 0.0;
        self.avg_ms = 0.0;
        self.worst_ms = 0.0;
        self.last_report = 0.0;
        self.win_count = 0;
        self.prev_buttons = 0;
    }

    /// Appelé pour chaque rapport de la manette (valeurs brutes, avant correction).
    /// `rate` : vrai rapport matériel (DualSense) → compte pour la fréquence.
    pub fn on_report(&mut self, p: &PadState, t: f64, rate: bool) {
        // --- Fréquence ---
        if rate {
            if self.last_report > 0.0 {
                let dt = t - self.last_report;
                if dt < 0.5 {
                    self.win_count += 1;
                    self.win_sum += dt;
                    self.win_max = self.win_max.max(dt);
                }
            } else {
                self.win_start = t;
            }
            self.last_report = t;
            let span = t - self.win_start;
            if span >= 1.0 {
                if self.win_count > 0 {
                    self.hz = (self.win_count as f64 / span) as f32;
                    self.avg_ms = (self.win_sum / self.win_count as f64 * 1000.0) as f32;
                    self.worst_ms = (self.win_max * 1000.0) as f32;
                    if let Some(c) = self.cur.as_mut() {
                        c.hz_sum += self.hz as f64;
                        c.hz_n += 1;
                    }
                }
                self.win_start = t;
                self.win_count = 0;
                self.win_sum = 0.0;
                self.win_max = 0.0;
            }
        }

        // --- Appuis et rebonds ---
        let pressed = p.buttons & !self.prev_buttons;
        let released = self.prev_buttons & !p.buttons;
        self.prev_buttons = p.buttons;
        for i in 0..NBTN {
            let bit = 1u32 << i;
            if released & bit != 0 {
                self.release_at[i] = t;
            }
            if pressed & bit != 0 {
                self.store.presses[i] += 1;
                let bounce = bit & NO_BOUNCE == 0 && self.release_at[i] >= 0.0 && t - self.release_at[i] < BOUNCE_S;
                if bounce {
                    self.store.bounces[i] += 1;
                }
                if let Some(c) = self.cur.as_mut() {
                    c.presses[i] += 1;
                    if bounce {
                        c.bounces += 1;
                    }
                }
                self.dirty = true;
            }
        }

        // --- Activité → session ---
        let mag = |x: f32, y: f32| (x * x + y * y).sqrt();
        let active = p.buttons != 0 || p.l2 > 0.2 || p.r2 > 0.2 || mag(p.lx, p.ly) > 0.5 || mag(p.rx, p.ry) > 0.5;
        if active {
            self.last_input = t;
            if self.cur.is_none() {
                self.cur = Some(Current {
                    start_unix: unix_now(),
                    start: t,
                    presses: [0; NBTN],
                    bounces: 0,
                    games: HashMap::new(),
                    hz_sum: 0.0,
                    hz_n: 0,
                    reminders: 0,
                });
            }
        }

        // --- Dérive au repos : stick immobile depuis 2 s, près du centre ---
        let day = (unix_now() / 86_400) as u32;
        for (k, (x, y)) in [(p.lx, p.ly), (p.rx, p.ry)].into_iter().enumerate() {
            let s = &mut self.sticks[k];
            if (x - s.anchor[0]).abs() > 0.02 || (y - s.anchor[1]).abs() > 0.02 {
                s.anchor = [x, y];
                s.since = t;
                continue;
            }
            if t - s.since > 2.0 && mag(x, y) < 0.4 && t - s.last_sample > 1.0 {
                s.last_sample = t;
                let m = mag(x, y);
                if self.store.drift.last().map(|d| d.day) != Some(day) {
                    self.store.drift.push(DriftDay { day, ..Default::default() });
                    let n = self.store.drift.len();
                    if n > 60 {
                        self.store.drift.drain(..n - 60);
                    }
                }
                let d = self.store.drift.last_mut().unwrap();
                if k == 0 {
                    d.n_left += 1;
                    d.left += (m - d.left) / d.n_left as f32;
                } else {
                    d.n_right += 1;
                    d.right += (m - d.right) / d.n_right as f32;
                }
            }
        }
    }

    pub fn live_session(&self) -> Option<LiveSession> {
        self.cur.as_ref().map(|c| LiveSession {
            minutes: ((self.last_input - c.start) / 60.0) as f32,
            presses: c.presses.iter().sum(),
            game: top_game(&c.games),
        })
    }

    /// Clôt la session si elle est finie. Renvoie le bilan.
    fn end_session(&mut self, force: bool, t: f64) -> Option<SessionRec> {
        let idle = t - self.last_input;
        if !force && idle < SESSION_IDLE_S {
            return None;
        }
        let c = self.cur.take()?;
        let secs = self.last_input - c.start;
        if secs < SESSION_MIN_S {
            return None;
        }
        let mut top: Vec<(u8, u64)> = c.presses.iter().enumerate().filter(|(_, n)| **n > 0).map(|(i, n)| (i as u8, *n)).collect();
        top.sort_by(|a, b| b.1.cmp(&a.1));
        top.truncate(3);
        let rec = SessionRec {
            start: c.start_unix,
            minutes: (secs / 60.0) as f32,
            game: top_game(&c.games),
            presses: c.presses.iter().sum(),
            top,
            hz: if c.hz_n > 0 { (c.hz_sum / c.hz_n as f64) as f32 } else { 0.0 },
            bounces: c.bounces,
        };
        self.store.play_minutes += rec.minutes as f64;
        self.store.sessions.push(rec.clone());
        let n = self.store.sessions.len();
        if n > 50 {
            self.store.sessions.drain(..n - 50);
        }
        self.dirty = true;
        Some(rec)
    }
}

fn top_game(games: &HashMap<String, u32>) -> String {
    games.iter().max_by_key(|(_, n)| **n).map(|(g, _)| g.clone()).unwrap_or_default()
}

pub fn fmt_minutes(m: f32) -> String {
    let m = m.round() as u32;
    match (m / 60, m % 60) {
        (0, mm) => format!("{mm} min"),
        (h, 0) => format!("{h} h"),
        (h, mm) => format!("{h} h {mm:02}"),
    }
}

fn thousands(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push('\u{202F}');
        }
        out.push(c);
    }
    out
}

/// Service d'une seconde : jeu en cours, fin de session, rappel de pause, batterie faible, sauvegarde.
pub fn spawn(shared: Arc<Shared>) {
    std::thread::Builder::new()
        .name("stats".into())
        .spawn(move || {
            let mut last_save = now();
            let mut battery_warned = false;
            loop {
                std::thread::sleep(Duration::from_secs(1));
                let t = now();
                let (toasts, reminder, minutes_cfg, en) = shared
                    .settings
                    .lock()
                    .map(|s| (s.toasts, s.break_reminder, s.break_minutes.max(15), s.lang == "en"))
                    .unwrap_or((true, true, 120, false));
                let game = shared.sys.lock().map(|s| s.last_game_exe.clone()).unwrap_or_default();
                let live = shared.live.lock().map(|l| l.clone()).unwrap_or_default();

                let mut ended = None;
                let mut remind = None;
                let mut to_save = None;
                if let Ok(mut tr) = shared.stats.lock() {
                    let active_now = t - tr.last_input < 5.0;
                    if let Some(c) = tr.cur.as_mut() {
                        if active_now && !game.is_empty() {
                            *c.games.entry(game.clone()).or_insert(0) += 1;
                        }
                    }
                    // Rappel de pause : toutes les N minutes de jeu continu.
                    let last_input = tr.last_input;
                    if let Some(c) = tr.cur.as_mut() {
                        let played = (last_input - c.start) / 60.0;
                        if reminder && active_now && played >= (minutes_cfg as f64) * (c.reminders + 1) as f64 {
                            c.reminders += 1;
                            remind = Some(played as f32);
                        }
                    }
                    ended = tr.end_session(false, t);
                    if tr.dirty && (ended.is_some() || t - last_save > 60.0) {
                        tr.dirty = false;
                        to_save = Some(tr.store.clone());
                    }
                }
                if let Some(store) = to_save {
                    store.save();
                    last_save = t;
                }

                if toasts {
                    if let Some(m) = remind {
                        let d = fmt_minutes(m);
                        if en {
                            toast::show(&format!("{d} of play"), "Time for a little break? Stretch, drink some water.");
                        } else {
                            toast::show(&format!("Ça fait {d} que tu joues"), "Une petite pause ? Étire-toi, bois un verre d'eau.");
                        }
                    }
                    if let Some(rec) = ended {
                        let d = fmt_minutes(rec.minutes);
                        let game = if rec.game.is_empty() { String::new() } else { format!(" · {}", rec.game) };
                        if en {
                            toast::show("Session recap", &format!("{d}{game} · {} presses", thousands(rec.presses)));
                        } else {
                            toast::show("Bilan de ta session", &format!("{d}{game} · {} appuis", thousands(rec.presses)));
                        }
                    }
                    // Batterie faible (DualSense, pas en charge).
                    if live.connected && !live.xbox && !live.charging && live.battery > 0 && live.battery <= 15 {
                        if !battery_warned {
                            battery_warned = true;
                            if en {
                                toast::show("Battery low", &format!("{} % left: plug in your controller soon.", live.battery));
                            } else {
                                toast::show("Batterie faible", &format!("Plus que {} % : pense à brancher ta manette.", live.battery));
                            }
                        }
                    } else if live.charging || live.battery >= 25 {
                        battery_warned = false;
                    }
                }
                if shared.quitting.load(Ordering::Relaxed) {
                    if let Ok(mut tr) = shared.stats.lock() {
                        let _ = tr.end_session(true, t);
                        tr.store.save();
                    }
                    break;
                }
            }
        })
        .expect("impossible de lancer le service des stats");
}

/// Sauvegarde immédiate (en quittant).
pub fn save_now(shared: &Shared) {
    if let Ok(mut tr) = shared.stats.lock() {
        let t = now();
        let _ = tr.end_session(true, t);
        tr.store.save();
    }
}
