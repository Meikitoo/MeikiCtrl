// Petites notifications noir et blanc en haut de l'écran, par-dessus le jeu
// (rappel de pause, bilan de session, batterie faible).
// Fenêtre Win32 dessinée en GDI : traversée par les clics, ne vole jamais le focus,
// créée seulement le temps de l'affichage (rien ne tourne le reste du temps).

use std::cell::RefCell;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::GetDpiForSystem;
use windows::Win32::UI::WindowsAndMessaging::*;

const SHOW_FOR: Duration = Duration::from_millis(6000);
const FADE: f32 = 0.18; // secondes
const MAX_ALPHA: f32 = 240.0;

static TX: OnceLock<Mutex<Sender<(String, String)>>> = OnceLock::new();

thread_local! {
    static TEXT: RefCell<(Vec<u16>, Vec<u16>, f32)> = RefCell::new((Vec::new(), Vec::new(), 1.0));
}

/// Affiche une notification (sans attendre). Ignoré si le service n'est pas lancé.
pub fn show(title: &str, body: &str) {
    if let Some(tx) = TX.get() {
        if let Ok(tx) = tx.lock() {
            let _ = tx.send((title.to_string(), body.to_string()));
        }
    }
}

pub fn spawn() {
    let (tx, rx) = channel();
    let _ = TX.set(Mutex::new(tx));
    let _ = std::thread::Builder::new().name("notifications".into()).spawn(move || run(rx));
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

fn font(height: i32, weight: i32) -> HFONT {
    let mut lf = LOGFONTW { lfHeight: -height, lfWeight: weight, ..Default::default() };
    for (i, c) in "Bahnschrift".encode_utf16().enumerate() {
        lf.lfFaceName[i] = c;
    }
    unsafe { CreateFontIndirectW(&lf) }
}

extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    unsafe {
        match msg {
            WM_MOUSEACTIVATE => LRESULT(3), // MA_NOACTIVATE
            WM_ERASEBKGND => LRESULT(1),
            WM_PAINT => {
                let mut ps = PAINTSTRUCT::default();
                let hdc = BeginPaint(hwnd, &mut ps);
                let mut rc = RECT::default();
                let _ = GetClientRect(hwnd, &mut rc);
                TEXT.with(|t| {
                    let mut guard = t.borrow_mut();
                    let (title, body, dpi) = &mut *guard;
                    let dpi = *dpi;
                    let s = |v: f32| (v * dpi).round() as i32;
                    let black = CreateSolidBrush(COLORREF(0x000B0B0B));
                    let white = CreateSolidBrush(COLORREF(0x00FFFFFF));
                    FillRect(hdc, &rc, black);
                    // Pavé blanc à gauche avec le « M » en noir (logo inversé).
                    let logo = RECT { left: 0, top: 0, right: rc.bottom, bottom: rc.bottom };
                    FillRect(hdc, &logo, white);
                    let f_logo = font(s(40.0), 800);
                    let f_title = font(s(21.0), 700);
                    let f_body = font(s(16.0), 400);

                    let old = SelectObject(hdc, HGDIOBJ(f_logo.0));
                    SetBkColor(hdc, COLORREF(0x00FFFFFF));
                    SetTextColor(hdc, COLORREF(0x000B0B0B));
                    let mut m = wide("M");
                    let mut r = logo;
                    DrawTextW(hdc, &mut m, &mut r, DT_CENTER | DT_VCENTER | DT_SINGLELINE);

                    SetBkColor(hdc, COLORREF(0x000B0B0B));
                    SetTextColor(hdc, COLORREF(0x00FFFFFF));
                    SelectObject(hdc, HGDIOBJ(f_title.0));
                    let pad = s(16.0);
                    let mut r = RECT { left: logo.right + pad, top: s(12.0), right: rc.right - pad, bottom: rc.bottom / 2 + s(4.0) };
                    DrawTextW(hdc, title, &mut r, DT_LEFT | DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS);
                    SetTextColor(hdc, COLORREF(0x00C8C8C8));
                    SelectObject(hdc, HGDIOBJ(f_body.0));
                    let mut r = RECT { left: logo.right + pad, top: rc.bottom / 2, right: rc.right - pad, bottom: rc.bottom - s(10.0) };
                    DrawTextW(hdc, body, &mut r, DT_LEFT | DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS);

                    SelectObject(hdc, old);
                    for f in [f_logo, f_title, f_body] {
                        let _ = DeleteObject(HGDIOBJ(f.0));
                    }
                    let _ = DeleteObject(HGDIOBJ(black.0));
                    let _ = DeleteObject(HGDIOBJ(white.0));
                });
                let _ = EndPaint(hwnd, &ps);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wp, lp),
        }
    }
}

fn run(rx: Receiver<(String, String)>) {
    let class = w!("MeikiCtrlToast");
    unsafe {
        let Ok(instance) = GetModuleHandleW(None) else { return };
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(wndproc),
            hInstance: instance.into(),
            lpszClassName: class,
            ..Default::default()
        };
        if RegisterClassExW(&wc) == 0 {
            eprintln!("Notifications indisponibles");
            return;
        }
        while let Ok((title, body)) = rx.recv() {
            if let Err(e) = show_one(instance.into(), class, &title, &body) {
                eprintln!("Notification : {e}");
            }
        }
    }
}

unsafe fn show_one(instance: HINSTANCE, class: PCWSTR, title: &str, body: &str) -> Result<()> {
    let dpi = GetDpiForSystem().max(96) as f32 / 96.0;
    TEXT.with(|t| *t.borrow_mut() = (wide(title), wide(body), dpi));
    let w = (440.0 * dpi) as i32;
    let h = (76.0 * dpi) as i32;
    let x = (GetSystemMetrics(SM_CXSCREEN) - w) / 2;
    let y = (36.0 * dpi) as i32;
    let hwnd = CreateWindowExW(
        WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_LAYERED | WS_EX_TRANSPARENT,
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
    let r = (14.0 * dpi) as i32;
    let rgn = CreateRoundRectRgn(0, 0, w + 1, h + 1, r, r);
    SetWindowRgn(hwnd, rgn, TRUE);
    SetLayeredWindowAttributes(hwnd, COLORREF(0), 0, LWA_ALPHA)?;
    let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
    let _ = SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);

    let start = Instant::now();
    let total = SHOW_FOR.as_secs_f32();
    loop {
        let mut msg = MSG::default();
        while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        let e = start.elapsed().as_secs_f32();
        if e >= total {
            break;
        }
        let a = (e / FADE).min(1.0).min(((total - e) / FADE).min(1.0));
        let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), (a * MAX_ALPHA) as u8, LWA_ALPHA);
        std::thread::sleep(Duration::from_millis(16));
    }
    DestroyWindow(hwnd)?;
    Ok(())
}
