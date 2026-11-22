// Système : mode administrateur, priorité du processus, démarrage avec Windows, instance unique.

use std::os::windows::process::CommandExt;
use std::process::Command;
use std::time::Duration;

use windows::core::{w, HSTRING, PCWSTR};
use windows::Win32::Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, HANDLE, TRUE};
use windows::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
use windows::Win32::System::Threading::{
    CreateMutexW, GetCurrentProcess, GetExitCodeProcess, OpenProcessToken, SetPriorityClass,
    WaitForSingleObject, ABOVE_NORMAL_PRIORITY_CLASS, HIGH_PRIORITY_CLASS, NORMAL_PRIORITY_CLASS,
};
use windows::Win32::UI::Shell::{ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW};
use windows::Win32::UI::WindowsAndMessaging::{SW_HIDE, SW_SHOWNORMAL};

const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
const NAME: &str = "MeikiCtrl";
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub fn has_arg(arg: &str) -> bool {
    std::env::args().any(|a| a == arg)
}

pub fn start_minimized() -> bool {
    has_arg("--minimized")
}

// ---------- Droits administrateur ----------

/// MeikiCtrl tourne-t-il avec les droits administrateur ?
pub fn is_elevated() -> bool {
    unsafe {
        let mut token = HANDLE::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() {
            return false;
        }
        let mut elevation = TOKEN_ELEVATION::default();
        let mut len = 0u32;
        let ok = GetTokenInformation(
            token,
            TokenElevation,
            Some(&mut elevation as *mut TOKEN_ELEVATION as *mut _),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut len,
        )
        .is_ok();
        let _ = CloseHandle(token);
        ok && elevation.TokenIsElevated != 0
    }
}

/// Lance un programme en administrateur (demande UAC si on ne l'est pas déjà).
/// `wait` : attendre la fin et renvoyer une erreur si le code de sortie n'est pas 0.
pub fn run_as_admin(file: &str, params: &str, wait: bool, visible: bool) -> Result<(), String> {
    unsafe {
        let file = HSTRING::from(file);
        let params = HSTRING::from(params);
        let mut info = SHELLEXECUTEINFOW {
            cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
            fMask: SEE_MASK_NOCLOSEPROCESS,
            lpVerb: w!("runas"),
            lpFile: PCWSTR(file.as_ptr()),
            lpParameters: PCWSTR(params.as_ptr()),
            nShow: if visible { SW_SHOWNORMAL.0 } else { SW_HIDE.0 },
            ..Default::default()
        };
        ShellExecuteExW(&mut info).map_err(|_| "Autorisation administrateur refusée.".to_string())?;
        if info.hProcess.is_invalid() {
            return if wait { Err("Le programme n'a pas pu être lancé.".into()) } else { Ok(()) };
        }
        let mut code = 0u32;
        if wait {
            WaitForSingleObject(info.hProcess, 30_000);
            code = 1;
            let _ = GetExitCodeProcess(info.hProcess, &mut code);
        }
        let _ = CloseHandle(info.hProcess);
        if code == 0 {
            Ok(())
        } else {
            Err(format!("Le programme a renvoyé une erreur (code {code})."))
        }
    }
}

/// Relance MeikiCtrl en administrateur. L'instance actuelle doit ensuite se fermer.
pub fn relaunch_elevated() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let mut args: Vec<String> = std::env::args().skip(1).filter(|a| a != "--wait-previous").collect();
    args.push("--wait-previous".into());
    run_as_admin(&exe.to_string_lossy(), &args.join(" "), false, true)
}

/// Ouvre un lien dans le navigateur par défaut (seulement des liens https).
pub fn open_url(url: &str) -> Result<(), String> {
    if !url.starts_with("https://") {
        return Err("Lien refusé.".into());
    }
    unsafe {
        let file = HSTRING::from(url);
        let mut info = SHELLEXECUTEINFOW {
            cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
            lpVerb: w!("open"),
            lpFile: PCWSTR(file.as_ptr()),
            nShow: SW_SHOWNORMAL.0,
            ..Default::default()
        };
        ShellExecuteExW(&mut info).map_err(|e| e.to_string())
    }
}

/// Le pilote ViGEmBus (manette Xbox virtuelle) est-il installé ?
pub fn vigem_installed() -> bool {
    quiet("reg", &["query", r"HKLM\SYSTEM\CurrentControlSet\Services\ViGEmBus"])
}

// ---------- Priorité du processus (comme OBS) ----------

/// 0 = normale, 1 = au-dessus de la normale, 2 = haute
pub fn set_process_priority(level: u8) {
    let class = match level {
        0 => NORMAL_PRIORITY_CLASS,
        2 => HIGH_PRIORITY_CLASS,
        _ => ABOVE_NORMAL_PRIORITY_CLASS,
    };
    unsafe {
        if let Err(e) = SetPriorityClass(GetCurrentProcess(), class) {
            eprintln!("Priorité du processus refusée : {e}");
        }
    }
}

// ---------- Démarrage avec Windows ----------

fn quiet(program: &str, args: &[&str]) -> bool {
    Command::new(program)
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn run_key_enabled() -> bool {
    quiet("reg", &["query", RUN_KEY, "/v", NAME])
}

fn task_enabled() -> bool {
    quiet("schtasks", &["/query", "/tn", NAME])
}

pub fn autostart_enabled() -> bool {
    run_key_enabled() || task_enabled()
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// Tâche planifiée « à l'ouverture de session, avec les droits les plus élevés » :
/// seule façon de lancer un programme en administrateur au démarrage sans demande UAC.
fn create_admin_task(exe: &str) -> Result<(), String> {
    let user = format!(
        "{}\\{}",
        std::env::var("USERDOMAIN").unwrap_or_default(),
        std::env::var("USERNAME").unwrap_or_default()
    );
    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <RegistrationInfo><Description>MeikiCtrl (mode administrateur)</Description></RegistrationInfo>
  <Triggers><LogonTrigger><Enabled>true</Enabled><UserId>{user}</UserId><Delay>PT5S</Delay></LogonTrigger></Triggers>
  <Principals><Principal id="Author"><UserId>{user}</UserId><LogonType>InteractiveToken</LogonType><RunLevel>HighestAvailable</RunLevel></Principal></Principals>
  <Settings>
    <MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>
    <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>
    <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>
    <ExecutionTimeLimit>PT0S</ExecutionTimeLimit>
    <Priority>4</Priority>
  </Settings>
  <Actions Context="Author"><Exec><Command>{exe}</Command><Arguments>--minimized</Arguments></Exec></Actions>
</Task>"#,
        user = xml_escape(&user),
        exe = xml_escape(exe)
    );
    // schtasks veut un fichier XML en UTF-16.
    let path = std::env::temp_dir().join("meikictrl-task.xml");
    let mut bytes = vec![0xFF, 0xFE];
    for u in xml.encode_utf16() {
        bytes.extend_from_slice(&u.to_le_bytes());
    }
    std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
    let ok = quiet("schtasks", &["/create", "/tn", NAME, "/xml", &path.to_string_lossy(), "/f"]);
    let _ = std::fs::remove_file(&path);
    if ok {
        Ok(())
    } else {
        Err("Création de la tâche planifiée refusée (droits administrateur nécessaires).".into())
    }
}

/// Active ou coupe le lancement avec Windows. En mode admin : tâche planifiée ; sinon : clé Run.
pub fn set_autostart(on: bool, admin: bool) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe = exe.to_string_lossy().to_string();
    // On repart propre : on retire les deux méthodes (sans erreur si absentes).
    quiet("reg", &["delete", RUN_KEY, "/v", NAME, "/f"]);
    if task_enabled() {
        quiet("schtasks", &["/delete", "/tn", NAME, "/f"]);
    }
    if !on {
        return if autostart_enabled() {
            Err("Impossible de retirer la tâche de démarrage (relance MeikiCtrl en administrateur).".into())
        } else {
            Ok(())
        };
    }
    if admin {
        if !is_elevated() {
            return Err("Le démarrage en mode administrateur demande que MeikiCtrl tourne déjà en administrateur.".into());
        }
        create_admin_task(&exe)
    } else {
        let cmd = format!("\"{exe}\" --minimized");
        if quiet("reg", &["add", RUN_KEY, "/v", NAME, "/t", "REG_SZ", "/d", &cmd, "/f"]) {
            Ok(())
        } else {
            Err("Impossible de modifier le démarrage automatique.".into())
        }
    }
}

// ---------- Instance unique ----------

/// Renvoie false si MeikiCtrl tourne déjà. `wait` : attendre que l'ancienne instance se ferme
/// (cas de la relance en administrateur).
pub fn single_instance(wait: bool) -> bool {
    let tries = if wait { 40 } else { 1 };
    for _ in 0..tries {
        unsafe {
            match CreateMutexW(None, TRUE, w!("Local\\MeikiCtrl")) {
                Ok(handle) => {
                    if GetLastError() != ERROR_ALREADY_EXISTS {
                        // Le verrou n'est jamais fermé : il reste actif jusqu'à la fin du programme.
                        let _ = handle;
                        return true;
                    }
                    let _ = CloseHandle(handle);
                }
                Err(_) => return true,
            }
        }
        if wait {
            std::thread::sleep(Duration::from_millis(250));
        }
    }
    false
}
