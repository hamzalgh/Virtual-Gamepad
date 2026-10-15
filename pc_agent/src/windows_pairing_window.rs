//! Modeless pairing presentation, owned exclusively by the tray message thread.
//! No code is logged or stored outside the authoritative snapshot and native text.
use crate::desktop::DesktopCommand;
use std::{cell::Cell, io, ptr};
use tokio::{sync::mpsc, time::Instant};
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    Graphics::Gdi::*,
    System::LibraryLoader::GetModuleHandleW,
    UI::{Input::KeyboardAndMouse::SetFocus, WindowsAndMessaging::*},
};

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}
thread_local! {static WINDOW: Cell<HWND> = const { Cell::new(ptr::null_mut()) };}

/// Route Tab, Escape and Enter to native dialog navigation without another loop.
pub fn translate(message: &MSG) -> bool {
    WINDOW.with(|window| {
        let hwnd = window.get();
        // SAFETY: this thread clears WINDOW during WM_NCDESTROY, before reuse.
        !hwnd.is_null() && unsafe { IsDialogMessageW(hwnd, message) != 0 }
    })
}

unsafe extern "system" fn procedure(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    // SAFETY: Win32 supplies message-specific parameters. The Cell is boxed before
    // CreateWindowExW and remains live through DestroyWindow and WM_NCDESTROY.
    unsafe {
        match msg {
            WM_NCCREATE => {
                let create = &*(l as *const CREATESTRUCTW);
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as isize);
            }
            WM_CLOSE => {
                DestroyWindow(hwnd);
                return 0;
            }
            WM_COMMAND if w & 0xffff == IDCANCEL as usize => {
                DestroyWindow(hwnd);
                return 0;
            }
            DM_GETDEFID => return ((DC_HASDEFID as usize) << 16 | IDCANCEL as usize) as isize,
            WM_CTLCOLORSTATIC => {
                SetTextColor(w as HDC, GetSysColor(COLOR_WINDOWTEXT));
                SetBkColor(w as HDC, GetSysColor(COLOR_WINDOW));
                return GetSysColorBrush(COLOR_WINDOW) as isize;
            }
            WM_NCDESTROY => {
                let alive = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Cell<bool>;
                if !alive.is_null() {
                    (*alive).set(false);
                    SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                }
                WINDOW.with(|window| {
                    if window.get() == hwnd {
                        window.set(ptr::null_mut());
                    }
                });
                // Closing this presentation must not quit the tray message pump.
            }
            _ => {}
        }
        DefWindowProcW(hwnd, msg, w, l)
    }
}

struct NativeWindow {
    hwnd: HWND,
    alive: Box<Cell<bool>>,
    countdown: HWND,
    fonts: Vec<HFONT>,
    last_seconds: Option<u64>,
}
impl NativeWindow {
    fn create(code: u32) -> io::Result<Self> {
        let class = wide("VirtualGamepad.PairingWindow");
        // SAFETY: all handles, UTF-16 strings and child controls are used on the
        // creating thread. RAII releases partial construction on every failure.
        unsafe {
            let instance = GetModuleHandleW(ptr::null());
            if instance.is_null() {
                return Err(io::Error::last_os_error());
            }
            let mut registered = WNDCLASSW::default();
            if GetClassInfoW(instance, class.as_ptr(), &mut registered) == 0 {
                let definition = WNDCLASSW {
                    lpfnWndProc: Some(procedure),
                    hInstance: instance,
                    hCursor: LoadCursorW(ptr::null_mut(), IDC_ARROW),
                    hbrBackground: GetSysColorBrush(COLOR_WINDOW),
                    lpszClassName: class.as_ptr(),
                    ..Default::default()
                };
                if RegisterClassW(&definition) == 0 {
                    return Err(io::Error::last_os_error());
                }
            }
            let mut result = Self {
                hwnd: ptr::null_mut(),
                alive: Box::new(Cell::new(true)),
                countdown: ptr::null_mut(),
                fonts: Vec::new(),
                last_seconds: None,
            };
            // The existing DPI-unaware process lets Windows scale the complete
            // native window. Keep a generous fixed client area without tiny text.
            let style = WS_CAPTION | WS_SYSMENU;
            let mut rect = windows_sys::Win32::Foundation::RECT {
                left: 0,
                top: 0,
                right: 460,
                bottom: 270,
            };
            if AdjustWindowRectEx(&mut rect, style, 0, WS_EX_CONTROLPARENT) == 0 {
                return Err(io::Error::last_os_error());
            }
            result.hwnd = CreateWindowExW(
                WS_EX_CONTROLPARENT,
                class.as_ptr(),
                wide("Pair phone | Virtual Gamepad").as_ptr(),
                style,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                rect.right - rect.left,
                rect.bottom - rect.top,
                ptr::null_mut(),
                ptr::null_mut(),
                instance,
                (&*result.alive as *const Cell<bool>).cast(),
            );
            if result.hwnd.is_null() {
                return Err(io::Error::last_os_error());
            }
            let body = result.font(18, 400)?;
            let digits = result.font(56, 700)?;
            result.text("Enter this code on your phone", 20, 20, 420, 28, body)?;
            result.text(&format!("{code:06}"), 20, 55, 420, 72, digits)?;
            result.countdown = result.text("", 20, 139, 420, 28, body)?;
            result.text(
                "Closing this window cancels the unused code.",
                15,
                174,
                430,
                28,
                body,
            )?;
            let button = CreateWindowExW(
                0,
                wide("BUTTON").as_ptr(),
                wide("Close pairing window").as_ptr(),
                WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_DEFPUSHBUTTON as u32,
                120,
                218,
                220,
                34,
                result.hwnd,
                IDCANCEL as usize as HMENU,
                instance,
                ptr::null(),
            );
            if button.is_null() {
                return Err(io::Error::last_os_error());
            }
            SendMessageW(button, WM_SETFONT, body as usize, 1);
            WINDOW.with(|window| window.set(result.hwnd));
            // Populate the countdown before making the window visible in update.
            Ok(result)
        }
    }
    fn font(&mut self, height: i32, weight: i32) -> io::Result<HFONT> {
        let font = unsafe {
            CreateFontW(
                -height,
                0,
                0,
                0,
                weight,
                0,
                0,
                0,
                DEFAULT_CHARSET as u32,
                0,
                0,
                DEFAULT_QUALITY as u32,
                0,
                wide("Segoe UI").as_ptr(),
            )
        };
        if font.is_null() {
            return Err(io::Error::last_os_error());
        }
        self.fonts.push(font);
        Ok(font)
    }
    fn text(
        &self,
        value: &str,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        font: HFONT,
    ) -> io::Result<HWND> {
        unsafe {
            let child = CreateWindowExW(
                0,
                wide("STATIC").as_ptr(),
                wide(value).as_ptr(),
                WS_CHILD | WS_VISIBLE | 1, /* SS_CENTER */
                x,
                y,
                width,
                height,
                self.hwnd,
                ptr::null_mut(),
                GetModuleHandleW(ptr::null()),
                ptr::null(),
            );
            if child.is_null() {
                return Err(io::Error::last_os_error());
            }
            SendMessageW(child, WM_SETFONT, font as usize, 1);
            Ok(child)
        }
    }
    fn update(&mut self, seconds: u64) {
        if self.last_seconds == Some(seconds) {
            return;
        }
        unsafe {
            SetWindowTextW(
                self.countdown,
                wide(&format!("Expires in {seconds} seconds")).as_ptr(),
            );
            if self.last_seconds.is_none() {
                ShowWindow(self.hwnd, SW_SHOWNORMAL);
                // A hidden-console launcher can override the first ShowWindow
                // via STARTUPINFO. The explicitly requested Pair popup must show.
                if IsWindowVisible(self.hwnd) == 0 {
                    ShowWindow(self.hwnd, SW_SHOW);
                }
                SetForegroundWindow(self.hwnd);
                SetFocus(GetDlgItem(self.hwnd, IDCANCEL));
            }
        }
        self.last_seconds = Some(seconds);
    }
}
impl Drop for NativeWindow {
    fn drop(&mut self) {
        unsafe {
            if !self.hwnd.is_null() && self.alive.get() {
                DestroyWindow(self.hwnd);
            }
            // Fonts must outlive all controls that select them.
            for font in &self.fonts {
                DeleteObject(*font);
            }
        }
    }
}

fn remaining(deadline: Instant, now: Instant) -> Option<u64> {
    let duration = deadline.checked_duration_since(now)?;
    if duration.is_zero() {
        return None;
    }
    Some(duration.as_secs() + u64::from(duration.subsec_nanos() != 0))
}

#[derive(Default)]
pub struct PairingWindow {
    current: Option<(u32, Instant)>,
    window: Option<NativeWindow>,
    dismissed: bool,
}
impl PairingWindow {
    pub fn refresh(
        &mut self,
        live: Option<(u32, Instant)>,
        commands: &mpsc::Sender<DesktopCommand>,
    ) -> io::Result<()> {
        let now = Instant::now();
        let live = live.filter(|(_, deadline)| remaining(*deadline, now).is_some());
        if live != self.current {
            self.window = None;
            self.current = live;
            self.dismissed = false;
        }
        let Some((code, deadline)) = live else {
            return Ok(());
        };
        if self
            .window
            .as_ref()
            .is_some_and(|window| !window.alive.get())
        {
            self.window = None;
            self.dismissed = true;
        }
        if self.dismissed {
            // Retry bounded queues until this particular window disappears. The
            // receiver also checks the deadline, so an old close cannot cancel a new Pair.
            let _ = commands.try_send(DesktopCommand::ClosePairingWindow(deadline));
            return Ok(());
        }
        if self.window.is_none() {
            match NativeWindow::create(code) {
                Ok(window) => self.window = Some(window),
                Err(error) => {
                    self.dismissed = true;
                    return Err(error);
                }
            }
        }
        if let Some(window) = &mut self.window {
            window.update(remaining(deadline, now).unwrap());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dismissed_popup_retries_full_queue_without_reopening_and_expiry_clears_it() {
        let deadline = Instant::now() + std::time::Duration::from_secs(120);
        let live = Some((123456, deadline));
        let mut popup = PairingWindow {
            current: live,
            dismissed: true,
            ..Default::default()
        };
        let (commands, mut receiver) = mpsc::channel(1);
        commands.try_send(DesktopCommand::Stop).unwrap();
        popup.refresh(live, &commands).unwrap();
        assert!(popup.window.is_none());
        assert!(matches!(receiver.try_recv(), Ok(DesktopCommand::Stop)));
        popup.refresh(live, &commands).unwrap();
        assert!(
            matches!(receiver.try_recv(), Ok(DesktopCommand::ClosePairingWindow(value)) if value == deadline)
        );
        popup
            .refresh(Some((123456, Instant::now())), &commands)
            .unwrap();
        assert!(popup.current.is_none());
        assert!(popup.window.is_none());
        assert!(receiver.try_recv().is_err());
    }
    #[test]
    fn countdown_rounds_up_and_clears_at_deadline() {
        let now = Instant::now();
        assert_eq!(
            remaining(now + std::time::Duration::from_millis(119999), now),
            Some(120)
        );
        assert_eq!(
            remaining(now + std::time::Duration::from_millis(1), now),
            Some(1)
        );
        assert_eq!(remaining(now, now), None);
        assert_eq!(
            remaining(now, now + std::time::Duration::from_millis(1)),
            None
        );
    }
}
