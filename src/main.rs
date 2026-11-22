// MeikiCtrl — appli manette à la sauce Meikitoow.
//  - thread « manette » : lecture DualSense → correction de dérive → manette Xbox virtuelle
//  - thread « overlay » : manette 3D par-dessus le jeu
//  - thread principal : interface (Tauri) + icône dans la zone de notification
// En debug (`cargo run`), une console affiche les logs. En release, pas de console.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod engine;
mod hidhide;
mod model;
mod output;
mod overlay;
mod pad;
mod profiles;
mod render;
mod state;
mod stats;
mod system;
mod toast;
mod ui;

use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};

use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;

use state::{Settings, Shared};

// ---- Réglages de l'overlay ----
const FPS_ACTIVE: f32 = 60.0; // quand tu joues
const FPS_IDLE: f32 = 30.0; // quand rien ne bouge
const IDLE_AFTER_S: f32 = 3.0;

// Raccourcis globaux : F8 = afficher/cacher l'overlay, F9 = recentrer
const HK_TOGGLE: i32 = 1;
const HK_RECENTER: i32 = 3;

fn main() {
    let settings = Settings::load();
    let elevated = system::is_elevated();

    // Mode administrateur : on se relance avec les droits admin, puis cette instance s'arrête.
    if settings.admin_mode && !elevated && !system::has_arg("--no-elevate") {
        match system::relaunch_elevated() {
            Ok(()) => return,
            Err(e) => eprintln!("Mode administrateur : {e} On continue sans."),
        }
    }

    if !system::single_instance(system::has_arg("--wait-previous")) {
        unsafe {
            MessageBoxW(
                None,
                windows::core::w!("MeikiCtrl est déjà lancé : clique sur son icône à côté de l'horloge."),
                windows::core::w!("MeikiCtrl"),
                MB_ICONINFORMATION,
            );
        }
        return;
    }
    unsafe {
        // Sommeil précis à 1 ms pour un rythme d'images régulier.
        windows::Win32::Media::timeBeginPeriod(1);
    }

    system::set_process_priority(settings.process_priority);
    println!(
        "MeikiCtrl {} — droits administrateur : {}",
        env!("CARGO_PKG_VERSION"),
        if elevated { "oui" } else { "non" }
    );

    // Passage en mode admin : le démarrage auto migre vers la tâche planifiée (lancement admin sans UAC).
    if settings.admin_mode && elevated && system::run_key_enabled() {
        match system::set_autostart(true, true) {
            Ok(()) => println!("Démarrage avec Windows : passé en mode administrateur"),
            Err(e) => eprintln!("Démarrage avec Windows : {e}"),
        }
    }

    let shared = Arc::new(Shared::new(settings));
    if let Ok(mut s) = shared.sys.lock() {
        s.autostart = system::autostart_enabled();
        s.elevated = elevated;
    }
    stats::now(); // démarre l'horloge
    toast::spawn();
    stats::spawn(shared.clone());
    pad::spawn(shared.clone());
    profiles::spawn(shared.clone());
    hidhide::spawn(shared.clone());

    {
        let s = shared.clone();
        std::thread::Builder::new()
            .name("overlay".into())
            .spawn(move || {
                if let Err(e) = overlay_loop(s) {
                    eprintln!("Overlay arrêté : {e:?}");
                }
            })
            .expect("impossible de lancer l'overlay");
    }

    ui::run(shared);
}

fn overlay_loop(shared: Arc<Shared>) -> windows::core::Result<()> {
    unsafe {
        use windows::Win32::System::Threading::{GetCurrentThread, SetThreadPriority, THREAD_PRIORITY_ABOVE_NORMAL};
        let _ = SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_ABOVE_NORMAL);
    }
    let mut cfg = shared.overlay_cfg();
    let mut rev = shared.overlay_rev.load(Ordering::Relaxed);
    let mut ov = overlay::Overlay::create(&cfg)?;
    let models = model::Models::load();
    if let Ok(mut s) = shared.sys.lock() {
        s.xbox_model = models.xbox.is_some();
    }
    let extra = models.meshes();
    let mut renderer = Some(render::Renderer::new(ov.hwnd, ov.width, ov.height, &extra)?);
    if let (Some(r), Ok(mut s)) = (renderer.as_ref(), shared.sys.lock()) {
        s.gpu_priority = r.gpu_priority.to_string();
    }
    let mut anim = model::Animator::new();

    unsafe {
        let _ = RegisterHotKey(ov.hwnd, HK_TOGGLE, MOD_NOREPEAT, VK_F8.0 as u32);
        let _ = RegisterHotKey(ov.hwnd, HK_RECENTER, MOD_NOREPEAT, VK_F9.0 as u32);
    }
    println!("Overlay prêt — F8 : afficher/cacher · F9 : recentrer");

    let mut shown = true;
    // Diagnostic : images dessinées et temps de rendu max, affichés toutes les 5 s.
    let mut stat_start = Instant::now();
    let mut stat_frames = 0u32;
    let mut stat_worst = Duration::ZERO;
    let mut seen_seq = shared.input_seq.load(Ordering::Relaxed);
    let mut last = Instant::now();
    let mut last_topmost = last;

    loop {
        // 1. Messages Windows (raccourcis)
        unsafe {
            let mut msg = MSG::default();
            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                if msg.message == WM_HOTKEY {
                    match msg.wParam.0 as i32 {
                        HK_TOGGLE => {
                            let v = !shared.overlay_visible.load(Ordering::Relaxed);
                            shared.overlay_visible.store(v, Ordering::Relaxed);
                        }
                        HK_RECENTER => anim.recenter(),
                        _ => {}
                    }
                    continue;
                }
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }

        // 2. Afficher/cacher selon l'interface ou le raccourci
        let want = shared.overlay_visible.load(Ordering::Relaxed);
        if want != shown {
            ov.set_visible(want);
            shown = want;
        }
        let frame_start = Instant::now();
        if !shown {
            // Caché = on ne dessine rien du tout.
            std::thread::sleep(Duration::from_millis(50));
            last = frame_start;
            continue;
        }
        if shared.recenter.swap(false, Ordering::Relaxed) {
            anim.recenter();
        }

        // Réglages de l'overlay modifiés dans l'interface : coin, taille, couleur, opacité.
        let now_rev = shared.overlay_rev.load(Ordering::Relaxed);
        if now_rev != rev {
            rev = now_rev;
            cfg = shared.overlay_cfg();
            if ov.place(&cfg) {
                // Nouvelle taille : on recrée le rendu (l'ancien doit être libéré d'abord).
                drop(renderer.take());
                renderer = Some(render::Renderer::new(ov.hwnd, ov.width, ov.height, &extra)?);
            }
        }
        let look = model::Look { accent: glam::Vec3::from(cfg.accent), opacity: cfg.opacity, skin: cfg.skin };

        // 3. Animation + rendu
        let dt = (frame_start - last).as_secs_f32().min(0.1);
        last = frame_start;
        let snap = shared.pad.lock().map(|g| g.clone()).unwrap_or_default();
        anim.update(&snap, dt);
        let asset = models.pick(cfg.model, snap.xbox);
        let scene = model::build(&anim, &snap, asset, &look);
        if let Some(r) = renderer.as_mut() {
            let t0 = Instant::now();
            r.render(&scene, anim.t)?;
            stat_worst = stat_worst.max(t0.elapsed());
            stat_frames += 1;
            if stat_start.elapsed() >= Duration::from_secs(5) {
                let secs = stat_start.elapsed().as_secs_f32();
                println!(
                    "Overlay : {:.0} i/s dessinées, {} sautées, rendu le plus long {:.1} ms",
                    stat_frames as f32 / secs,
                    r.skipped,
                    stat_worst.as_secs_f32() * 1000.0
                );
                r.skipped = 0;
                stat_frames = 0;
                stat_worst = Duration::ZERO;
                stat_start = Instant::now();
            }
        }

        // Certains jeux repassent devant : on se remet au premier plan de temps en temps.
        if frame_start - last_topmost > Duration::from_secs(2) {
            ov.reassert_topmost();
            last_topmost = frame_start;
        }

        // 4. Rythme : 60 i/s quand tu joues, 30 quand rien ne bouge.
        //    Au repos, une entrée réveille l'overlay tout de suite (pas d'attente de 33 ms).
        let fps = if anim.idle_for() > IDLE_AFTER_S { FPS_IDLE } else { FPS_ACTIVE };
        let can_wake = fps == FPS_IDLE;
        let deadline = frame_start + Duration::from_secs_f32(1.0 / fps);
        let earliest = frame_start + Duration::from_millis(8);
        loop {
            let now = Instant::now();
            if now >= deadline {
                break;
            }
            if can_wake && now >= earliest && shared.input_seq.load(Ordering::Relaxed) != seen_seq {
                break;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        seen_seq = shared.input_seq.load(Ordering::Relaxed);
    }
}
