// Masque la DualSense aux jeux pendant l'émulation, avec HidHide (Nefarius).
// Les jeux ne voient alors que la manette Xbox virtuelle : plus de doubles entrées.
// HidHide demande les droits administrateur : Windows affiche une demande (UAC) à chaque changement.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use crate::state::Shared;

const CLI: &str = r"C:\Program Files\Nefarius Software Solutions\HidHide\x64\HidHideCLI.exe";

pub fn installed() -> bool {
    Path::new(CLI).exists()
}

/// Lance HidHideCLI en administrateur et attend la fin.
/// En mode administrateur, Windows ne demande plus rien.
fn run_elevated(args: &str) -> Result<(), String> {
    crate::system::run_as_admin(CLI, args, true, false)
}

fn quote(s: &str) -> String {
    format!("\"{s}\"")
}

fn hide(instance: &str) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    // Liste blanche : MeikiCtrl doit continuer à voir la manette.
    run_elevated(&format!(
        "--app-reg {} --dev-hide {} --cloak-on",
        quote(&exe.to_string_lossy()),
        quote(instance)
    ))
}

fn unhide(instance: &str) -> Result<(), String> {
    run_elevated(&format!("--dev-unhide {}", quote(instance)))
}

/// Surveille l'état voulu (émulation + option « masquer ») et applique.
pub fn spawn(shared: Arc<Shared>) {
    std::thread::Builder::new()
        .name("hidhide".into())
        .spawn(move || {
            let present = installed();
            if let Ok(mut s) = shared.sys.lock() {
                s.hidhide_installed = present;
            }
            if !present {
                return;
            }
            // Instance actuellement masquée, et dernier état demandé (pour ne pas
            // relancer une demande UAC en boucle après un refus).
            let mut hidden: Option<String> = None;
            let mut last_want: Option<(bool, Option<String>)> = None;
            loop {
                let option = shared.settings.lock().map(|s| s.hide_controller).unwrap_or(false);
                // En pause pendant un jeu qui bloque l'émulation : la DualSense redevient visible.
                let emu = shared.emulation_wanted();
                let inst = shared.hid_instance.lock().map(|i| i.clone()).unwrap_or(None);
                // Manette débranchée : on ne touche à rien (pas de demande UAC inutile).
                let want = option && emu;
                let key = (want, inst.clone());

                if last_want.as_ref() != Some(&key) {
                    last_want = Some(key);
                    let result = match (want, &hidden, &inst) {
                        (true, None, Some(i)) => {
                            set_busy(&shared, true);
                            let r = hide(i);
                            if r.is_ok() {
                                hidden = Some(i.clone());
                                println!("DualSense masquée aux jeux (HidHide)");
                            }
                            Some(r)
                        }
                        (false, Some(h), _) => {
                            set_busy(&shared, true);
                            let r = unhide(h);
                            if r.is_ok() {
                                hidden = None;
                                println!("DualSense de nouveau visible");
                            }
                            Some(r)
                        }
                        _ => None,
                    };
                    if let Ok(mut s) = shared.sys.lock() {
                        s.hidhide_busy = false;
                        s.hidhide_hidden = hidden.is_some();
                        if let Some(r) = result {
                            s.hidhide_error = r.err();
                        }
                    }
                }
                std::thread::sleep(Duration::from_millis(500));
            }
        })
        .expect("impossible de lancer le service HidHide");
}

fn set_busy(shared: &Shared, busy: bool) {
    if let Ok(mut s) = shared.sys.lock() {
        s.hidhide_busy = busy;
    }
}

/// En quittant : rend la DualSense de nouveau visible (sinon les jeux ne la verraient plus).
pub fn restore(shared: &Shared) {
    let hidden = shared.sys.lock().map(|s| s.hidhide_hidden).unwrap_or(false);
    if !hidden {
        return;
    }
    let inst = shared.hid_instance.lock().map(|i| i.clone()).unwrap_or(None);
    if let Some(i) = inst {
        let _ = unhide(&i);
    } else {
        // Manette débranchée entre-temps : on coupe le masquage global.
        let _ = run_elevated("--cloak-off");
    }
}
