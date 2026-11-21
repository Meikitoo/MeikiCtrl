// Interface Tauri (HTML/CSS/JS dans le dossier `ui/`, aucun Node nécessaire)
// + icône dans la zone de notification.
// Fermer la fenêtre la DÉTRUIT (la WebView s'arrête) : rien ne tourne pour l'interface en jeu.

use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, RunEvent, State, WebviewUrl, WebviewWindowBuilder};

use crate::state::{Live, Settings, Shared, Sys};
use crate::{hidhide, system};

type Sh<'a> = State<'a, Arc<Shared>>;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LiveOut {
    live: Live,
    sys: Sys,
    overlay: bool,
    emulation: bool,
}

/// Infos en direct (interrogé ~30 fois par seconde, seulement quand la fenêtre est ouverte).
#[tauri::command]
fn get_live(state: Sh<'_>) -> LiveOut {
    LiveOut {
        live: state.live.lock().map(|l| l.clone()).unwrap_or_default(),
        sys: state.sys.lock().map(|s| s.clone()).unwrap_or_default(),
        overlay: state.overlay_visible.load(Ordering::Relaxed),
        emulation: state.emulation_on.load(Ordering::Relaxed),
    }
}

#[tauri::command]
fn get_settings(state: Sh<'_>) -> Settings {
    state.settings.lock().map(|s| s.clone()).unwrap_or_default()
}

fn update_settings(state: &Shared, f: impl FnOnce(&mut Settings)) {
    if let Ok(mut s) = state.settings.lock() {
        f(&mut s);
        s.normalize();
        s.save();
    }
    state.apply_active();
}

#[tauri::command]
fn save_settings(state: Sh<'_>, settings: Settings) {
    let mut overlay_changed = false;
    update_settings(&state, |s| {
        // Ces deux-là sont gérés par leurs interrupteurs : on garde les valeurs actuelles.
        let (o, e) = (s.overlay_on, s.emulation_on);
        overlay_changed = s.overlay != settings.overlay;
        *s = settings;
        s.overlay_on = o;
        s.emulation_on = e;
    });
    if overlay_changed {
        state.bump_overlay();
    }
}

#[tauri::command]
fn set_overlay(state: Sh<'_>, on: bool) {
    state.overlay_visible.store(on, Ordering::Relaxed);
    update_settings(&state, |s| s.overlay_on = on);
}

#[tauri::command]
fn set_emulation(state: Sh<'_>, on: bool) {
    state.emulation_on.store(on, Ordering::Relaxed);
    update_settings(&state, |s| s.emulation_on = on);
}

#[tauri::command]
fn recenter(state: Sh<'_>) {
    state.recenter.store(true, Ordering::Relaxed);
}

#[tauri::command]
fn set_autostart(state: Sh<'_>, on: bool) -> Result<bool, String> {
    let admin = state.settings.lock().map(|s| s.admin_mode).unwrap_or(false) && system::is_elevated();
    system::set_autostart(on, admin)?;
    let now = system::autostart_enabled();
    if let Ok(mut s) = state.sys.lock() {
        s.autostart = now;
    }
    Ok(now)
}

/// Priorité du processus, comme le réglage d'OBS : 0 normale, 1 au-dessus, 2 haute.
#[tauri::command]
fn set_process_priority(state: Sh<'_>, level: u8) {
    let level = level.min(2);
    system::set_process_priority(level);
    update_settings(&state, |s| s.process_priority = level);
}

/// Active/coupe le mode administrateur. Activé sans droits admin : MeikiCtrl se relance.
/// Renvoie un message à afficher (ou « restart » si l'appli redémarre).
#[tauri::command]
fn set_admin_mode(app: AppHandle, state: Sh<'_>, on: bool) -> Result<String, String> {
    let elevated = system::is_elevated();
    update_settings(&state, |s| s.admin_mode = on);
    if on && !elevated {
        if let Err(e) = system::relaunch_elevated() {
            update_settings(&state, |s| s.admin_mode = false);
            return Err(e);
        }
        // La nouvelle instance (admin) prend le relais : on se ferme.
        let app = app.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(300));
            app.exit(0);
        });
        return Ok("restart".into());
    }
    if !on {
        // Démarrage auto : on repasse sur la méthode classique (clé Run).
        if system::autostart_enabled() {
            let _ = system::set_autostart(true, false);
        }
        if elevated {
            return Ok("Le mode administrateur sera coupé au prochain lancement de MeikiCtrl.".into());
        }
    }
    Ok(String::new())
}

/// Calibrage de la dérive : stick lâché pendant 2 s, on mesure le bruit max,
/// et on règle la zone morte du profil juste au-dessus.
#[tauri::command]
async fn calibrate(state: Sh<'_>, stick: String, profile: usize) -> Result<f32, String> {
    let shared = state.inner().clone();
    if !shared.live.lock().map(|l| l.connected).unwrap_or(false) {
        return Err("Aucune manette connectée.".into());
    }
    let left = stick == "left";
    let mut max = 0f32;
    for _ in 0..40 {
        std::thread::sleep(Duration::from_millis(50));
        let l = shared.live.lock().map(|l| l.clone()).unwrap_or_default();
        let [x, y] = if left { l.raw_l } else { l.raw_r };
        max = max.max((x * x + y * y).sqrt());
    }
    if max > 0.45 {
        return Err("Le stick a bougé pendant la mesure : lâche-le complètement et recommence.".into());
    }
    let dz = (max * 1.2 + 0.02).clamp(0.03, 0.35);
    update_settings(&shared, |s| {
        if let Some(p) = s.profiles.get_mut(profile) {
            if left {
                p.left_deadzone = dz;
            } else {
                p.right_deadzone = dz;
            }
        }
    });
    Ok(dz)
}

fn open_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
        return;
    }
    if let Err(e) = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
        .title("MeikiCtrl")
        .inner_size(1180.0, 760.0)
        .min_inner_size(960.0, 620.0)
        .build()
    {
        eprintln!("Ouverture de la fenêtre impossible : {e}");
    }
}

/// Quitter proprement : rendre la DualSense visible si elle était masquée, puis fermer.
fn quit(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        let st = app.state::<Arc<Shared>>();
        hidhide::restore(&st);
        app.exit(0);
    });
}

fn toggle(st: &Shared, overlay: bool) {
    if overlay {
        let v = !st.overlay_visible.load(Ordering::Relaxed);
        st.overlay_visible.store(v, Ordering::Relaxed);
        update_settings(st, |s| s.overlay_on = v);
    } else {
        let v = !st.emulation_on.load(Ordering::Relaxed);
        st.emulation_on.store(v, Ordering::Relaxed);
        update_settings(st, |s| s.emulation_on = v);
    }
}

pub fn run(shared: Arc<Shared>) {
    let app = tauri::Builder::default()
        .manage(shared)
        .invoke_handler(tauri::generate_handler![
            get_live,
            get_settings,
            save_settings,
            set_overlay,
            set_emulation,
            recenter,
            set_autostart,
            set_process_priority,
            set_admin_mode,
            calibrate
        ])
        .setup(|app| {
            let open = MenuItem::with_id(app, "open", "Ouvrir MeikiCtrl", true, None::<&str>)?;
            let overlay = MenuItem::with_id(app, "overlay", "Overlay : afficher / cacher (F8)", true, None::<&str>)?;
            let emu = MenuItem::with_id(app, "emu", "Émulation Xbox : activer / couper", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quitter MeikiCtrl", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open, &overlay, &emu, &quit_item])?;

            let mut tray = TrayIconBuilder::with_id("meikictrl")
                .tooltip("MeikiCtrl")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| {
                    let st = app.state::<Arc<Shared>>();
                    match event.id.as_ref() {
                        "open" => open_main(app),
                        "overlay" => toggle(&st, true),
                        "emu" => toggle(&st, false),
                        "quit" => quit(app),
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        open_main(tray.app_handle());
                    }
                });
            if let Some(icon) = app.default_window_icon() {
                tray = tray.icon(icon.clone());
            }
            tray.build(app)?;

            // Lancé avec Windows : on reste discret dans la zone de notification.
            if !system::start_minimized() {
                open_main(app.handle());
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("impossible de démarrer l'interface");

    app.run(|_app, event| {
        // Fermer la fenêtre ne quitte pas l'appli : elle continue dans la zone de notification.
        // Seul « Quitter » (exit avec un code) ferme vraiment.
        if let RunEvent::ExitRequested { api, code, .. } = event {
            if code.is_none() {
                api.prevent_exit();
            }
        }
    });
}
