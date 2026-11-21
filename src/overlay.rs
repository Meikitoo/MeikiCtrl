// La fenêtre overlay : transparente, toujours au premier plan, traversée par les clics,
// jamais active, absente de la barre des tâches. Son contenu est posé par DirectComposition.

use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::GetDpiForSystem;
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::state::OverlayCfg;

// ---- Réglages ----
const BASE_W: i32 = 460; // taille à 100 % d'échelle Windows et 100 % de taille d'overlay
const BASE_H: i32 = 320;
const MARGIN: i32 = 24; // distance au coin de l'écran
/// Mets `false` si la manette reste invisible (test de secours : l'overlay
/// ne laissera alors plus passer les clics).
const CLICK_THROUGH: bool = true;

pub struct Overlay {
    pub hwnd: HWND,
    pub width: u32,
    pub height: u32,
}

extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    unsafe {
        match msg {
            WM_DESTROY => {
                PostQuitMessage(0);
                LRESULT(0)
            }
            // MA_NOACTIVATE : ne jamais voler le focus au jeu.
            WM_MOUSEACTIVATE => LRESULT(3),
            _ => DefWindowProcW(hwnd, msg, wp, lp),
        }
    }
}

/// Position et taille de l'overlay selon le coin choisi et la taille.
fn rect(cfg: &OverlayCfg) -> (i32, i32, i32, i32) {
    unsafe {
        let dpi = GetDpiForSystem().max(96) as f32 / 96.0;
        let w = (BASE_W as f32 * dpi * cfg.scale).round() as i32;
        let h = (BASE_H as f32 * dpi * cfg.scale).round() as i32;
        let mut work = RECT::default();
        let _ = SystemParametersInfoW(
            SPI_GETWORKAREA,
            0,
            Some(&mut work as *mut RECT as *mut _),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        );
        let right = cfg.corner == 0 || cfg.corner == 2;
        let bottom = cfg.corner <= 1;
        let x = if right { work.right - w - MARGIN } else { work.left + MARGIN };
        let y = if bottom { work.bottom - h - MARGIN } else { work.top + MARGIN };
        (x, y, w, h)
    }
}

impl Overlay {
    /// Déplace / redimensionne. Renvoie true si la taille a changé (il faut recréer le rendu).
    pub fn place(&mut self, cfg: &OverlayCfg) -> bool {
        let (x, y, w, h) = rect(cfg);
        unsafe {
            let _ = SetWindowPos(self.hwnd, HWND_TOPMOST, x, y, w, h, SWP_NOACTIVATE | SWP_NOOWNERZORDER);
        }
        let resized = self.width != w as u32 || self.height != h as u32;
        self.width = w as u32;
        self.height = h as u32;
        resized
    }

    pub fn create(cfg: &OverlayCfg) -> Result<Self> {
        unsafe {
            let instance = GetModuleHandleW(None)?;
            let class = w!("MeikiCtrlOverlay");
            let wc = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                lpfnWndProc: Some(wndproc),
                hInstance: instance.into(),
                lpszClassName: class,
                ..Default::default()
            };
            if RegisterClassExW(&wc) == 0 {
                return Err(Error::from_win32());
            }

            let (x, y, w, h) = rect(cfg);

            let mut ex = WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_NOREDIRECTIONBITMAP;
            if CLICK_THROUGH {
                ex |= WS_EX_LAYERED | WS_EX_TRANSPARENT;
            }

            let hwnd = CreateWindowExW(
                ex,
                class,
                w!("MeikiCtrl"),
                WS_POPUP,
                x,
                y,
                w,
                h,
                None,
                None,
                instance,
                None,
            )?;

            if CLICK_THROUGH {
                SetLayeredWindowAttributes(hwnd, COLORREF(0), 255, LWA_ALPHA)?;
            }
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);

            Ok(Self { hwnd, width: w as u32, height: h as u32 })
        }
    }

    pub fn set_visible(&self, v: bool) {
        unsafe {
            let _ = ShowWindow(self.hwnd, if v { SW_SHOWNOACTIVATE } else { SW_HIDE });
        }
    }

    pub fn reassert_topmost(&self) {
        unsafe {
            let _ = SetWindowPos(
                self.hwnd,
                HWND_TOPMOST,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOOWNERZORDER,
            );
        }
    }
}
