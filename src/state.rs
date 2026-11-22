// État partagé entre le moteur (thread manette), l'overlay, les services et l'interface.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use crate::engine::{default_remap, SOURCE_COUNT};
use crate::pad::PadState;

/// Un profil = tous les réglages de la manette pour un jeu.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Profile {
    pub name: String,
    /// Nom de l'exécutable du jeu (ex. « cod.exe »). Vide = pas de bascule auto.
    pub exe: String,
    /// Zone morte (0 à 0,35) : en dessous, le stick compte comme lâché.
    pub left_deadzone: f32,
    pub right_deadzone: f32,
    /// Portée max (0,8 à 1) : au-delà, le stick compte comme poussé à fond.
    pub left_outer: f32,
    pub right_outer: f32,
    /// Anti-zone morte (0 à 0,3) : compense la zone morte propre au jeu.
    pub left_anti_deadzone: f32,
    pub right_anti_deadzone: f32,
    /// Courbe de réponse (exposant) : 1 = linéaire, >1 = plus précis au centre.
    pub left_curve: f32,
    pub right_curve: f32,
    pub trigger_deadzone: f32,
    pub trigger_curve: f32,
    /// Gâchette rapide : le moindre appui = appui complet.
    pub hair_trigger: bool,
    /// Ce jeu bloque les manettes virtuelles (ex. COD) : tant qu'il est lancé,
    /// l'émulation est en pause et la DualSense reste visible.
    pub block_emulation: bool,
    /// Pour chaque bouton DualSense (ordre `engine::SOURCES`) : bouton Xbox envoyé (0 = aucun).
    pub remap: Vec<u16>,
    /// Couleur de la barre lumineuse pour ce jeu (mode « couleur du profil »).
    pub light_color: [f32; 3],
    /// Gâchettes adaptatives : 0 = aucun effet, 1 = résistance, 2 = clic d'arme.
    pub trigger_l: u8,
    pub trigger_r: u8,
    /// Force des gâchettes adaptatives (0,05 à 1).
    pub trigger_strength: f32,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            name: "Par défaut".into(),
            exe: String::new(),
            left_deadzone: 0.08,
            right_deadzone: 0.08,
            left_outer: 0.98,
            right_outer: 0.98,
            left_anti_deadzone: 0.0,
            right_anti_deadzone: 0.0,
            left_curve: 1.0,
            right_curve: 1.0,
            trigger_deadzone: 0.02,
            trigger_curve: 1.0,
            hair_trigger: false,
            block_emulation: false,
            remap: default_remap(),
            light_color: [1.0, 1.0, 1.0],
            trigger_l: 0,
            trigger_r: 0,
            trigger_strength: 0.6,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct OverlayCfg {
    /// 0 = bas droite, 1 = bas gauche, 2 = haut droite, 3 = haut gauche
    pub corner: u8,
    /// Taille (0,6 à 1,6)
    pub scale: f32,
    /// Opacité (0,3 à 1)
    pub opacity: f32,
    /// Couleur des éclairs et des lueurs (RVB 0 à 1)
    pub accent: [f32; 3],
    /// Modèle affiché : 0 = automatique (selon la manette branchée), 1 = DualSense, 2 = Xbox
    pub model: u8,
    /// Habillage de la coque : 0 = éclairs, 1 = noir mat, 2 = blanc, 3 = couleur.
    pub skin: u8,
}

impl Default for OverlayCfg {
    fn default() -> Self {
        Self { corner: 0, scale: 1.0, opacity: 1.0, accent: [0.0, 0.85, 1.0], model: 0, skin: 0 }
    }
}

/// Barre lumineuse et LED joueur de la DualSense.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct LightCfg {
    /// 0 = ne pas toucher, 1 = couleur fixe, 2 = batterie, 3 = couleur du profil,
    /// 4 = couleur du joueur, 5 = respiration, 6 = RGB (toutes les couleurs qui défilent).
    pub mode: u8,
    pub color: [f32; 3],
    /// Luminosité (0,05 à 1).
    pub brightness: f32,
    /// Éclair blanc quand tu appuies sur le bouton de tir choisi.
    pub flash_on_fire: bool,
    /// Bouton qui déclenche le flash (numéro de bit, voir `pad::btn`) : 7 = R2 par défaut.
    pub flash_button: u8,
    /// Numéro de joueur (LED sous le pavé) : 0 = éteintes, 1 à 4.
    pub player: u8,
    /// Vitesse des effets RGB et respiration (0,05 à 1).
    pub speed: f32,
}

impl Default for LightCfg {
    fn default() -> Self {
        Self { mode: 2, color: [1.0, 1.0, 1.0], brightness: 0.8, flash_on_fire: false, flash_button: 7, player: 1, speed: 0.5 }
    }
}

/// Réglages sauvegardés dans %APPDATA%\MeikiCtrl\settings.json
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    /// Dernier état connu, réappliqué au démarrage.
    pub overlay_on: bool,
    pub emulation_on: bool,
    /// Masquer la DualSense aux jeux pendant l'émulation (HidHide).
    pub hide_controller: bool,
    /// Bascule automatique de profil selon le jeu au premier plan.
    pub auto_profiles: bool,
    /// Profil utilisé quand aucun jeu reconnu n'est au premier plan.
    pub default_profile: usize,
    pub profiles: Vec<Profile>,
    pub overlay: OverlayCfg,
    /// Mode administrateur : MeikiCtrl se relance avec les droits admin
    /// (priorité GPU temps réel garantie, plus de demande UAC pour HidHide).
    pub admin_mode: bool,
    /// Priorité du processus, comme OBS : 0 = normale, 1 = au-dessus de la normale, 2 = haute.
    pub process_priority: u8,
    pub lightbar: LightCfg,
    /// Interrupteur général des gâchettes adaptatives.
    pub adaptive_triggers: bool,
    /// Langue : « fr », « en » (vide = celle de Windows).
    pub lang: String,
    /// Assistant de premier lancement terminé.
    pub first_run_done: bool,
    /// Notifications à l'écran (pause, bilan, batterie).
    pub toasts: bool,
    pub break_reminder: bool,
    pub break_minutes: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            overlay_on: true,
            emulation_on: false,
            hide_controller: false,
            auto_profiles: true,
            default_profile: 0,
            profiles: vec![Profile::default()],
            overlay: OverlayCfg::default(),
            admin_mode: false,
            process_priority: 1,
            lightbar: LightCfg::default(),
            adaptive_triggers: true,
            lang: String::new(),
            first_run_done: false,
            toasts: true,
            break_reminder: true,
            break_minutes: 120,
        }
    }
}

impl Settings {
    fn path() -> Option<PathBuf> {
        std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join("MeikiCtrl").join("settings.json"))
    }

    pub fn load() -> Self {
        let mut s: Settings = Self::path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        s.normalize();
        s
    }

    /// Remet les valeurs dans des bornes sûres (fichier ancien ou modifié à la main).
    pub fn normalize(&mut self) {
        if self.profiles.is_empty() {
            self.profiles.push(Profile::default());
        }
        if self.default_profile >= self.profiles.len() {
            self.default_profile = 0;
        }
        let def = default_remap();
        for p in &mut self.profiles {
            if p.remap.len() != SOURCE_COUNT {
                p.remap = def.clone();
            }
            p.left_deadzone = p.left_deadzone.clamp(0.0, 0.35);
            p.right_deadzone = p.right_deadzone.clamp(0.0, 0.35);
            p.left_outer = p.left_outer.clamp(0.5, 1.0);
            p.right_outer = p.right_outer.clamp(0.5, 1.0);
            p.left_anti_deadzone = p.left_anti_deadzone.clamp(0.0, 0.4);
            p.right_anti_deadzone = p.right_anti_deadzone.clamp(0.0, 0.4);
            p.left_curve = p.left_curve.clamp(0.3, 3.0);
            p.right_curve = p.right_curve.clamp(0.3, 3.0);
            p.trigger_deadzone = p.trigger_deadzone.clamp(0.0, 0.5);
            p.trigger_curve = p.trigger_curve.clamp(0.3, 3.0);
            p.exe = p.exe.trim().to_lowercase();
            p.trigger_l = p.trigger_l.min(2);
            p.trigger_r = p.trigger_r.min(2);
            p.trigger_strength = p.trigger_strength.clamp(0.05, 1.0);
            p.light_color = p.light_color.map(|x| x.clamp(0.0, 1.0));
        }
        let l = &mut self.lightbar;
        l.mode = l.mode.min(6);
        l.speed = l.speed.clamp(0.05, 1.0);
        if l.flash_button > 18 {
            l.flash_button = 7;
        }
        l.player = l.player.min(4);
        l.brightness = l.brightness.clamp(0.05, 1.0);
        l.color = l.color.map(|x| x.clamp(0.0, 1.0));
        self.break_minutes = self.break_minutes.clamp(15, 600);
        if self.lang != "fr" && self.lang != "en" {
            self.lang.clear();
        }
        self.process_priority = self.process_priority.min(2);
        let o = &mut self.overlay;
        o.corner = o.corner.min(3);
        o.model = o.model.min(2);
        o.skin = o.skin.min(3);
        o.scale = o.scale.clamp(0.5, 2.0);
        o.opacity = o.opacity.clamp(0.2, 1.0);
    }

    pub fn save(&self) {
        if let Some(p) = Self::path() {
            if let Some(dir) = p.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            if let Ok(json) = serde_json::to_string_pretty(self) {
                if let Err(e) = std::fs::write(&p, json) {
                    eprintln!("Sauvegarde des réglages impossible : {e}");
                }
            }
        }
    }
}

/// Infos manette en direct pour l'interface (écrites par le thread manette).
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Live {
    pub connected: bool,
    pub edge: bool,
    pub xbox: bool,
    pub bluetooth: bool,
    pub battery: u8,
    pub charging: bool,
    /// Sticks bruts (avant correction).
    pub raw_l: [f32; 2],
    pub raw_r: [f32; 2],
    /// Sticks corrigés (ce que voit le jeu).
    pub out_l: [f32; 2],
    pub out_r: [f32; 2],
    pub l2: f32,
    pub r2: f32,
    pub buttons: u32,
    pub emulation_active: bool,
    pub emulation_error: Option<String>,
    /// Fréquence de la manette (rapports par seconde) et intervalle moyen / pire (ms).
    pub hz: f32,
    pub avg_ms: f32,
    pub worst_ms: f32,
}

/// État des services (profils, HidHide, démarrage auto).
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Sys {
    pub active_profile: usize,
    pub active_by_game: bool,
    /// Dernier jeu (programme) vu au premier plan, hors MeikiCtrl.
    pub last_game_exe: String,
    pub hidhide_installed: bool,
    pub hidhide_hidden: bool,
    pub hidhide_busy: bool,
    pub hidhide_error: Option<String>,
    pub autostart: bool,
    /// MeikiCtrl tourne avec les droits administrateur.
    pub elevated: bool,
    /// Priorité GPU obtenue pour l'overlay (« temps réel », « haute » ou « normale »).
    pub gpu_priority: String,
    /// Le modèle 3D Xbox est disponible (assets/xbox.mkm).
    pub xbox_model: bool,
    /// Profil dont le jeu bloque l'émulation, s'il est lancé.
    pub emulation_blocked_by: Option<String>,
}

pub struct Shared {
    /// État corrigé, utilisé par l'overlay.
    pub pad: Mutex<PadState>,
    pub live: Mutex<Live>,
    pub sys: Mutex<Sys>,
    pub settings: Mutex<Settings>,
    /// Profil actif, lu par le moteur à chaque rapport (copie partagée, sans attente).
    pub engine: Mutex<Arc<Profile>>,
    /// Chemin d'instance Windows de la manette (pour HidHide).
    pub hid_instance: Mutex<Option<String>>,
    pub overlay_visible: AtomicBool,
    pub emulation_on: AtomicBool,
    /// Un jeu qui bloque l'émulation est lancé : émulation et masquage en pause.
    pub emulation_blocked: AtomicBool,
    pub recenter: AtomicBool,
    /// Incrémenté à chaque changement des réglages de l'overlay.
    pub overlay_rev: AtomicU32,
    /// Incrémenté à chaque changement d'entrée (bouton, stick, gâchette) : réveille l'overlay.
    pub input_seq: AtomicU64,
    /// Santé de la manette et sessions.
    pub stats: Mutex<crate::stats::Tracker>,
    /// MeikiCtrl se ferme : la manette rend la main (gâchettes libres, LED d'origine).
    pub quitting: AtomicBool,
    /// La manette a bien rendu la main (lumière / gâchettes).
    pub released: AtomicBool,
}

impl Shared {
    pub fn new(settings: Settings) -> Self {
        let first = settings.profiles[settings.default_profile].clone();
        Self {
            pad: Mutex::new(PadState::default()),
            live: Mutex::new(Live::default()),
            sys: Mutex::new(Sys { active_profile: settings.default_profile, ..Default::default() }),
            engine: Mutex::new(Arc::new(first)),
            hid_instance: Mutex::new(None),
            overlay_visible: AtomicBool::new(settings.overlay_on),
            emulation_on: AtomicBool::new(settings.emulation_on),
            emulation_blocked: AtomicBool::new(false),
            recenter: AtomicBool::new(false),
            overlay_rev: AtomicU32::new(0),
            input_seq: AtomicU64::new(0),
            stats: Mutex::new(crate::stats::Tracker::new()),
            quitting: AtomicBool::new(false),
            released: AtomicBool::new(false),
            settings: Mutex::new(settings),
        }
    }

    /// Profil courant pour le moteur.
    pub fn profile(&self) -> Arc<Profile> {
        self.engine.lock().map(|g| g.clone()).unwrap_or_default()
    }

    /// Recharge le profil actif dans le moteur (après une modification des réglages).
    pub fn apply_active(&self) {
        let idx = self.sys.lock().map(|s| s.active_profile).unwrap_or(0);
        if let Ok(s) = self.settings.lock() {
            let p = s.profiles.get(idx).or_else(|| s.profiles.first()).cloned().unwrap_or_default();
            if let Ok(mut e) = self.engine.lock() {
                if **e != p {
                    *e = Arc::new(p);
                }
            }
        }
    }

    pub fn overlay_cfg(&self) -> OverlayCfg {
        self.settings.lock().map(|s| s.overlay).unwrap_or_default()
    }

    pub fn bump_overlay(&self) {
        self.overlay_rev.fetch_add(1, Ordering::Relaxed);
    }
}

impl Shared {
    /// Interface en anglais ?
    pub fn en(&self) -> bool {
        self.settings.lock().map(|s| s.lang == "en").unwrap_or(false)
    }

    /// Émulation réellement voulue : demandée ET pas en pause à cause d'un jeu.
    pub fn emulation_wanted(&self) -> bool {
        self.emulation_on.load(Ordering::Relaxed) && !self.emulation_blocked.load(Ordering::Relaxed)
    }
}
