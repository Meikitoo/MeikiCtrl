// Profils et jeux, vérifiés une fois par seconde :
//  - bascule automatique de profil selon le jeu au premier plan ;
//  - pause de l'émulation quand un jeu qui bloque les manettes virtuelles (ex. COD) est lancé.
// On lit la liste des programmes (sans ouvrir le processus du jeu) : ça marche même avec un anti-triche.

use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use windows::Win32::Foundation::CloseHandle;
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

use crate::state::Shared;

/// Tous les programmes en cours : (identifiant, nom de l'exe en minuscules).
fn processes() -> Vec<(u32, String)> {
    let mut list = Vec::with_capacity(400);
    unsafe {
        let Ok(snap) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else {
            return list;
        };
        let mut entry = PROCESSENTRY32W { dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32, ..Default::default() };
        if Process32FirstW(snap, &mut entry).is_ok() {
            loop {
                let len = entry.szExeFile.iter().position(|&c| c == 0).unwrap_or(entry.szExeFile.len());
                let name = String::from_utf16_lossy(&entry.szExeFile[..len]).to_lowercase();
                list.push((entry.th32ProcessID, name));
                if Process32NextW(snap, &mut entry).is_err() {
                    break;
                }
            }
        }
        let _ = CloseHandle(snap);
    }
    list
}

fn foreground_pid() -> Option<u32> {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() {
            return None;
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        (pid != 0).then_some(pid)
    }
}

pub fn spawn(shared: Arc<Shared>) {
    std::thread::Builder::new()
        .name("profils".into())
        .spawn(move || {
            let me = std::env::current_exe()
                .ok()
                .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_lowercase()))
                .unwrap_or_default();
            // Programmes à ignorer : MeikiCtrl lui-même et le bureau Windows.
            let ignored = [me.as_str(), "explorer.exe", "searchhost.exe", "shellexperiencehost.exe"];
            let mut game = String::new();
            let mut last_block: Option<String> = None;
            loop {
                let procs = processes();
                if let Some(pid) = foreground_pid() {
                    if let Some((_, exe)) = procs.iter().find(|(p, _)| *p == pid) {
                        if !ignored.contains(&exe.as_str()) {
                            game = exe.clone();
                        }
                    }
                }

                let (idx, by_game, block) = match shared.settings.lock() {
                    Ok(s) => {
                        let found = if s.auto_profiles && !game.is_empty() {
                            s.profiles.iter().position(|p| !p.exe.is_empty() && p.exe == game)
                        } else {
                            None
                        };
                        // Un jeu « qui bloque l'émulation » est lancé (même pas au premier plan) ?
                        let block = s
                            .profiles
                            .iter()
                            .find(|p| p.block_emulation && !p.exe.is_empty() && procs.iter().any(|(_, n)| *n == p.exe))
                            .map(|p| p.name.clone());
                        match found {
                            Some(i) => (i, true, block),
                            None => (s.default_profile, false, block),
                        }
                    }
                    Err(_) => (0, false, None),
                };

                shared.emulation_blocked.store(block.is_some(), Ordering::Relaxed);
                if block != last_block {
                    match &block {
                        Some(n) => println!("Émulation en pause : « {n} » bloque les manettes virtuelles"),
                        None => println!("Émulation : reprise"),
                    }
                    last_block = block.clone();
                }

                let changed = match shared.sys.lock() {
                    Ok(mut s) => {
                        let changed = s.active_profile != idx;
                        s.active_profile = idx;
                        s.active_by_game = by_game;
                        s.last_game_exe = game.clone();
                        s.emulation_blocked_by = block;
                        changed
                    }
                    Err(_) => false,
                };
                if changed {
                    let name = shared
                        .settings
                        .lock()
                        .ok()
                        .and_then(|s| s.profiles.get(idx).map(|p| p.name.clone()))
                        .unwrap_or_default();
                    println!("Profil actif : {name}");
                }
                shared.apply_active();
                std::thread::sleep(Duration::from_secs(1));
            }
        })
        .expect("impossible de lancer le service des profils");
}
