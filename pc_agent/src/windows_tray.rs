//! Windows-only adapter: main-thread message pump, native menus and owned HKCU startup.
//! Unsafe calls use live handles, NUL-terminated UTF-16 and synchronous stack buffers.
use crate::{
    application::Error,
    desktop::{Desktop, DesktopCommand, DesktopStatus},
    paired_session::PairingCommand,
};
use std::{
    io,
    os::windows::process::CommandExt,
    process::Command,
    ptr,
    sync::{atomic::Ordering, mpsc},
};
use tray_icon::{
    Icon, TrayIconBuilder, TrayIconEvent,
    menu::{CheckMenuItem, Menu, MenuEvent, MenuItem},
};
use windows_sys::Win32::{System::Registry::*, UI::WindowsAndMessaging::*};

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}
fn editor_or_choose(
    launch: impl FnOnce() -> io::Result<()>,
    choose: impl FnOnce() -> io::Result<()>,
) -> io::Result<()> {
    match launch() {
        Err(error) if error.kind() == io::ErrorKind::NotFound => choose(),
        result => result,
    }
}
fn open_settings(path: &std::path::Path) -> io::Result<()> {
    editor_or_choose(
        || Command::new("notepad.exe").arg(path).spawn().map(|_| ()),
        || {
            use std::os::windows::ffi::OsStrExt;
            use windows::Win32::System::Com::{
                COINIT_APARTMENTTHREADED, CoInitializeEx, CoUninitialize,
            };
            use windows_sys::Win32::UI::Shell::{OAIF_EXEC, OPENASINFO, SHOpenWithDialog};
            // Initialize this UI thread for the shell dialog, balancing even S_FALSE.
            // A pre-existing apartment with another mode is left untouched.
            let initialized = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }.is_ok();
            let file: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
            let info = OPENASINFO {
                pcszFile: file.as_ptr(),
                pcszClass: ptr::null(),
                oaifInFlags: OAIF_EXEC,
            };
            // SAFETY: the UTF-16 path and structure remain live throughout this
            // synchronous native dialog. No default association is registered.
            let result = unsafe { SHOpenWithDialog(ptr::null_mut(), &info) };
            if initialized {
                unsafe {
                    CoUninitialize();
                }
            }
            // Cancelling editor selection is a normal presentation action.
            if result >= 0 || result == 0x800704c7u32 as i32 {
                Ok(())
            } else {
                Err(io::Error::other(format!(
                    "Editor selection failed (0x{:08X}). Open {} in your text editor.",
                    result as u32,
                    path.display()
                )))
            }
        },
    )
}
thread_local! {static REFRESH: std::cell::RefCell<Option<Box<dyn FnMut()>>> = const {std::cell::RefCell::new(None)};}
unsafe extern "system" fn refresh_timer(
    _: windows_sys::Win32::Foundation::HWND,
    _: u32,
    _: usize,
    _: u32,
) {
    // Windows dispatches TIMERPROC during nested menu/dialog loops too. Never unwind into Win32.
    let _ = std::panic::catch_unwind(|| {
        REFRESH.with(|cell| {
            if let Ok(mut callback) = cell.try_borrow_mut()
                && let Some(callback) = callback.as_mut()
            {
                callback();
            }
        })
    });
}
struct RefreshGuard;
impl Drop for RefreshGuard {
    fn drop(&mut self) {
        REFRESH.with(|cell| *cell.borrow_mut() = None);
    }
}
fn dialog(text: &str, confirm: bool) -> bool {
    // SAFETY: both buffers live through this synchronous call; no owner window required.
    unsafe {
        MessageBoxW(
            ptr::null_mut(),
            wide(text).as_ptr(),
            wide("Virtual Gamepad").as_ptr(),
            if confirm {
                MB_YESNO | MB_ICONQUESTION | MB_DEFBUTTON2
            } else {
                MB_OK | MB_ICONINFORMATION
            },
        ) == IDYES
    }
}
pub fn report_error(text: &str) {
    dialog(text, false);
}

#[derive(Default)]
struct FaultNotice {
    sent: bool,
}
impl FaultNotice {
    fn next(&mut self, snapshot: &DesktopStatus, quitting: bool) -> Option<String> {
        // A new run clears service status before publishing Running.
        if snapshot.service.as_ref().is_none_or(|s| !s.backend.fault) {
            self.sent = false;
        }
        if quitting || snapshot.phase != "Running" || self.sent {
            return None;
        }
        let backend = &snapshot.service.as_ref()?.backend;
        if !backend.fault {
            return None;
        }
        self.sent = true; // also suppress repeated requests if Windows refuses it
        Some(backend.diagnostic.map(|d| format!("Controller {} failed{} (attempt {}). Stop, then Start and pair with a fresh code. See Status and setup details.", d.operation, d.native_code.map(|n| format!(", Windows error {n}")).unwrap_or_default(), d.attempt))
            .unwrap_or_else(|| "Controller failed. Stop, then Start and pair with a fresh code. See Status and setup details.".into()))
    }
}
fn notification(tray: &tray_icon::TrayIcon, guid: u128, text: &str) -> bool {
    use windows_sys::Win32::UI::Shell::*;
    let mut data = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: tray.window_handle(),
        uFlags: NIF_GUID | NIF_INFO,
        guidItem: windows_sys::core::GUID::from_u128(guid),
        dwInfoFlags: NIIF_ERROR | NIIF_RESPECT_QUIET_TIME,
        ..Default::default()
    };
    for (slot, value) in data.szInfo.iter_mut().take(255).zip(text.encode_utf16()) {
        *slot = value;
    }
    for (slot, value) in data
        .szInfoTitle
        .iter_mut()
        .take(63)
        .zip("Virtual controller failure".encode_utf16())
    {
        *slot = value;
    }
    // SAFETY: live icon HWND and the exact GUID used at creation; fixed, zero
    // terminated buffers live through this call. The shell owns presentation.
    unsafe { Shell_NotifyIconW(NIM_MODIFY, &data) != 0 }
}

fn backend_details(snapshot: &DesktopStatus) -> String {
    snapshot
        .service
        .as_ref()
        .and_then(|s| s.backend.diagnostic)
        .map(|d| d.message())
        .unwrap_or_default()
}
pub fn detach_console() {
    // SAFETY: presentation selection occurs before any terminal worker starts.
    unsafe {
        windows_sys::Win32::System::Console::FreeConsole();
    }
}
/// Presence guard lasts through all service and log cleanup. Setup never closes us.
pub struct RuntimeGuard(windows_sys::Win32::Foundation::HANDLE);
impl RuntimeGuard {
    pub fn acquire() -> io::Result<Self> {
        use windows_sys::Win32::{
            Foundation::{CloseHandle, GetLastError},
            System::Threading::*,
        };
        fn setup_absent() -> io::Result<()> {
            let handle = unsafe {
                OpenMutexW(
                    SYNCHRONIZATION_SYNCHRONIZE,
                    0,
                    wide("Global\\VirtualGamepad.Desktop.Setup").as_ptr(),
                )
            };
            if !handle.is_null() {
                unsafe {
                    CloseHandle(handle);
                }
                return Err(io::Error::new(
                    io::ErrorKind::WouldBlock,
                    "Setup is running. Wait for it to finish before starting the agent.",
                ));
            }
            let error = unsafe { GetLastError() };
            if error != 2 {
                return Err(io::Error::from_raw_os_error(error as i32));
            }
            Ok(())
        }
        setup_absent()?;
        let handle = unsafe {
            CreateMutexW(
                ptr::null(),
                0,
                wide("Global\\VirtualGamepad.Desktop.Runtime").as_ptr(),
            )
        };
        if handle.is_null() {
            return Err(io::Error::last_os_error());
        }
        let guard = Self(handle);
        setup_absent()?;
        Ok(guard)
    }
}
impl Drop for RuntimeGuard {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.0);
        }
    }
}
pub fn label(s: &DesktopStatus) -> String {
    if s.phase != "Running" {
        return s.phase.into();
    }
    if !s.error.is_empty() || s.pairing.storage_failed {
        return "Action required — see details".into();
    }
    if let Some(service) = &s.service {
        if service.backend.fault {
            return "Controller failed; see details, then Stop/Start".into();
        }
        if service.backend.timed_out {
            return "Timed out — controller neutral".into();
        }
        if service.backend.ready && service.backend.attached {
            if service.counters.estimated_loss > 0 {
                return "Connected / packet loss recorded — see details".into();
            }
            return "Connected / healthy".into();
        }
    }
    if s.pairing
        .window
        .is_some_and(|(_, expiry)| expiry > tokio::time::Instant::now())
    {
        return "Pairing window open".into();
    }
    "Ready / awaiting fresh pairing".into()
}
fn endpoint_label(snapshot: &DesktopStatus) -> String {
    if matches!(snapshot.phase, "Running" | "Stopping")
        && let Some(service) = &snapshot.service
    {
        let addresses = service
            .addresses
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ");
        return format!(
            "IPv4: {} | UDP {}",
            if addresses.is_empty() {
                "unavailable"
            } else {
                &addresses
            },
            service.bound.port()
        );
    }
    format!(
        "Configured UDP {} | {}",
        snapshot.settings.port, snapshot.phase
    )
}
pub fn run() -> Result<(), Error> {
    let mut desktop = Desktop::spawn()?;
    let menu = Menu::new();
    let state = MenuItem::with_id("state", "Starting", false, None);
    let endpoint = MenuItem::with_id("endpoint", "IPv4: starting", false, None);
    let code = MenuItem::with_id("code", "Pairing window closed", false, None);
    let phone = MenuItem::with_id("phone", "Last phone: none", false, None);
    let start = MenuItem::with_id("start", "Start listening", true, None);
    let stop = MenuItem::with_id("stop", "Stop listening", true, None);
    let pair = MenuItem::with_id("pair", "Pair — end current session / new code", true, None);
    let close = MenuItem::with_id("close", "Close pairing window", true, None);
    let forget = MenuItem::with_id("forget", "Forget last phone…", true, None);
    let replace = MenuItem::with_id("replace", "Replace phone / new code…", true, None);
    let details = MenuItem::with_id("details", "Status and setup details", true, None);
    let settings = MenuItem::with_id("settings", "Edit settings…", true, None);
    let reload = MenuItem::with_id("reload", "Reload settings (while stopped)", false, None);
    let logs = MenuItem::with_id("logs", "Open log folder", true, None);
    let help = MenuItem::with_id("help", "Troubleshooting", true, None);
    let preflight = MenuItem::with_id(
        "preflight",
        "Check driver / firewall (read only)",
        true,
        None,
    );
    let startup = CheckMenuItem::with_id("startup", "Start with Windows", true, false, None);
    let exit = MenuItem::with_id("exit", "Exit", true, None);
    menu.append_items(&[
        &state, &endpoint, &code, &phone, &start, &stop, &pair, &close, &forget, &replace,
        &details, &settings, &reload, &logs, &help, &preflight, &startup, &exit,
    ])?;
    let (events, rx) = mpsc::sync_channel(8);
    let exiting = desktop.exit.clone();
    MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
        if event.id.0 == "exit" {
            exiting.store(true, Ordering::Release);
        } else {
            let _ = events.try_send(event.id.0);
        }
    }));
    TrayIconEvent::set_event_handler(Some(|_: TrayIconEvent| {}));
    let mut pixels = vec![0u8; 32 * 32 * 4];
    for y in 6..26 {
        for x in 3..29 {
            let i = (y * 32 + x) * 4;
            pixels[i..i + 4].copy_from_slice(
                if (x < 14 && (y == 15 || y == 16))
                    || ((x == 8 || x == 9) && y > 10 && y < 22)
                    || (x > 20 && (y == 12 || y == 20))
                {
                    &[255, 255, 255, 255]
                } else {
                    &[35, 96, 180, 255]
                },
            );
        }
    }
    // Per-run identity avoids binding portable preview notifications to an old path.
    let mut guid_bytes = [0u8; 16];
    getrandom::fill(&mut guid_bytes).map_err(|e| io::Error::other(e.to_string()))?;
    let guid = u128::from_le_bytes(guid_bytes);
    let tray = TrayIconBuilder::new()
        .with_guid(guid)
        .with_menu(Box::new(menu))
        .with_icon(Icon::from_rgba(pixels, 32, 32)?)
        .with_tooltip("Virtual Gamepad")
        .build()?;
    // The native timer also runs inside nested Windows menu/dialog loops.
    let view = desktop.status.clone();
    let quitting = desktop.exit.clone();
    let live_code = code.clone();
    let live_endpoint = endpoint.clone();
    let popup_commands = desktop.commands.clone();
    let mut popup = crate::windows_pairing_window::PairingWindow::default();
    let live_tray = tray.clone();
    let live_state = state.clone();
    let mut notices = FaultNotice::default();
    REFRESH.with(|cell| {
        *cell.borrow_mut() = Some(Box::new(move || {
            let s = view.borrow().clone();
            live_endpoint.set_text(endpoint_label(&s));
            let exiting = quitting.load(Ordering::Acquire);
            live_state.set_text(if exiting { "Shutting down".into() } else { label(&s) });
            if let Some(text) = notices.next(&s, exiting)
                && !notification(&live_tray, guid, &text) {
                tracing::warn!("Windows could not display controller failure notification; details remain in tray status");
            }
            let live = if s.phase == "Running"
                && !exiting
                && !s.pairing.pending
                && !s.pairing.storage_failed
                && !s.service.as_ref().is_some_and(|service| service.backend.fault)
            {
                s.pairing.window
            } else {
                None
            };
            live_code.set_text(
                if live.is_some_and(|(_, deadline)| deadline > tokio::time::Instant::now()) {
                    "Pairing code is shown in its window"
                } else {
                    "Pairing window closed"
                },
            );
            if let Err(error) = popup.refresh(live, &popup_commands) {
                dialog(
                    &format!("Could not open pairing window: {error}. Choose Pair to retry."),
                    false,
                );
            }
        }))
    });
    let _refresh = RefreshGuard;
    // SAFETY: thread timer is created and destroyed on the tray owner thread.
    let timer = unsafe { SetTimer(ptr::null_mut(), 0, 200, Some(refresh_timer)) };
    if timer == 0 {
        return Err(io::Error::last_os_error().into());
    }
    let result = (|| -> Result<(), Error> {
        loop {
            let mut message = MSG::default();
            // SAFETY: initialized writable MSG; this thread owns the native queue.
            let got = unsafe { GetMessageW(&mut message, ptr::null_mut(), 0, 0) };
            if got < 0 {
                return Err(io::Error::last_os_error().into());
            }
            if got == 0 {
                break;
            }
            if !crate::windows_pairing_window::translate(&message) {
                unsafe {
                    TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
            }
            let snapshot = desktop.status.borrow().clone();
            let quitting = desktop.exit.load(Ordering::Acquire);
            let running = snapshot.phase == "Running" && !quitting;
            let title = if quitting {
                "Shutting down".into()
            } else {
                label(&snapshot)
            };
            state.set_text(&title);
            let _ = tray.set_tooltip(Some(format!("Virtual Gamepad: {title}")));
            endpoint.set_text(endpoint_label(&snapshot));
            phone.set_text(
                snapshot
                    .pairing
                    .last_phone
                    .map(|id| format!("Last phone: …{id:06X} (fresh code required)"))
                    .unwrap_or_else(|| "Last phone: none / unavailable while stopped".into()),
            );
            start.set_enabled(snapshot.phase == "Stopped" && !quitting);
            stop.set_enabled(running);
            for item in [&pair, &close, &forget, &replace] {
                item.set_enabled(running && !snapshot.pairing.pending);
            }
            reload.set_enabled(snapshot.phase == "Stopped" && !quitting);
            startup.set_checked(snapshot.startup);
            if desktop.worker.as_ref().is_some_and(|w| w.is_finished()) {
                break;
            }
            if quitting {
                continue;
            }
            for id in rx.try_iter().take(8) {
                let command = match id.as_str() {
                    "start" => Some(DesktopCommand::Start),
                    "stop" => Some(DesktopCommand::Stop),
                    "pair" => Some(DesktopCommand::Pair(PairingCommand::Open)),
                    "close" => Some(DesktopCommand::Pair(PairingCommand::Close)),
                    "forget"
                        if dialog("End the current session and forget the last phone?", true) =>
                    {
                        Some(DesktopCommand::Pair(PairingCommand::Forget))
                    }
                    "replace"
                        if dialog(
                            "End the current session, forget the last phone, and open a new pairing window?",
                            true,
                        ) =>
                    {
                        Some(DesktopCommand::Pair(PairingCommand::Replace))
                    }
                    "reload" => Some(DesktopCommand::Reload),
                    "startup" => Some(DesktopCommand::Startup(!snapshot.startup)),
                    "preflight" => Some(DesktopCommand::Preflight),
                    "details" => {
                        dialog(
                            &format!(
                                "{}\nVersion {} | Windows {}\n{}\nCurrent service (or last stopped run): {:?}\n{}\n{}\n{}\nController slot: assigned by Windows (not exposed by backend).\nLog drops: {}",
                                label(&snapshot),
                                env!("CARGO_PKG_VERSION"),
                                std::env::consts::ARCH,
                                endpoint_label(&snapshot),
                                snapshot.service,
                                snapshot.setup,
                                snapshot.error,
                                backend_details(&snapshot),
                                crate::telemetry::log_drops()
                            ),
                            false,
                        );
                        None
                    }
                    "settings" => {
                        if let Err(error) = open_settings(&desktop.root.join("settings.ini")) {
                            dialog(&format!("Could not open settings: {error}"), false);
                        }
                        None
                    }
                    "logs" => {
                        if let Err(error) = Command::new("explorer.exe").arg(&desktop.root).spawn()
                        {
                            dialog(&format!("Could not open log folder: {error}"), false);
                        }
                        None
                    }
                    "help" => {
                        dialog(include_str!("../packaging/troubleshooting.txt"), false);
                        None
                    }
                    _ => None,
                };
                if let Some(command) = command
                    && desktop.commands.try_send(command).is_err()
                {
                    dialog("Desktop command queue is busy. Retry shortly.", false);
                }
            }
        }
        Ok(())
    })();
    // Clear secret-bearing presentation before any potentially slow service join,
    // including when the message pump itself failed.
    drop(_refresh);
    unsafe {
        KillTimer(ptr::null_mut(), timer);
    }
    let cleanup = desktop.shutdown();
    drop(tray);
    // muda's handler is a OnceCell. It retains only the bounded, now closed sender
    // until process exit; do not pretend a second set_event_handler replaces it.
    result.and(cleanup)
}

const RUN_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
const RUN_NAME: &str = "VirtualGamepad.Desktop";
struct Key(HKEY);
impl Drop for Key {
    fn drop(&mut self) {
        unsafe {
            RegCloseKey(self.0);
        }
    }
}
fn key(write: bool) -> io::Result<Option<Key>> {
    let mut handle = ptr::null_mut();
    let result = unsafe {
        RegOpenKeyExW(
            HKEY_CURRENT_USER,
            wide(RUN_KEY).as_ptr(),
            0,
            if write {
                KEY_QUERY_VALUE | KEY_SET_VALUE
            } else {
                KEY_QUERY_VALUE
            },
            &mut handle,
        )
    };
    if result == 2 {
        return Ok(None);
    }
    if result != 0 {
        return Err(io::Error::from_raw_os_error(result as i32));
    }
    Ok(Some(Key(handle)))
}
fn startup_value(key: &Key) -> io::Result<Option<String>> {
    let mut bytes = vec![0u16; 32768];
    let mut len = (bytes.len() * 2) as u32;
    let mut kind = 0;
    let result = unsafe {
        RegQueryValueExW(
            key.0,
            wide(RUN_NAME).as_ptr(),
            ptr::null(),
            &mut kind,
            bytes.as_mut_ptr().cast(),
            &mut len,
        )
    };
    if result == 2 {
        return Ok(None);
    }
    if result != 0 {
        return Err(io::Error::from_raw_os_error(result as i32));
    }
    if kind != REG_SZ || !len.is_multiple_of(2) {
        return Err(io::Error::other(
            "Startup entry has an unexpected format; preserved.",
        ));
    }
    bytes.truncate(len as usize / 2);
    if bytes.last() == Some(&0) {
        bytes.pop();
    }
    String::from_utf16(&bytes)
        .map(Some)
        .map_err(io::Error::other)
}
fn expected_startup() -> io::Result<String> {
    Ok(format!("\"{}\" --tray", std::env::current_exe()?.display()))
}
pub fn startup_enabled() -> io::Result<bool> {
    match key(false)? {
        Some(key) => Ok(startup_value(&key)?.as_deref() == Some(expected_startup()?.as_str())),
        None => Ok(false),
    }
}
pub fn set_startup(enabled: bool) -> io::Result<()> {
    let expected = expected_startup()?;
    let key = match key(true)? {
        Some(key) => key,
        None if !enabled => return Ok(()),
        None => {
            let mut handle = ptr::null_mut();
            let result = unsafe {
                RegCreateKeyExW(
                    HKEY_CURRENT_USER,
                    wide(RUN_KEY).as_ptr(),
                    0,
                    ptr::null(),
                    REG_OPTION_NON_VOLATILE,
                    KEY_QUERY_VALUE | KEY_SET_VALUE,
                    ptr::null(),
                    &mut handle,
                    ptr::null_mut(),
                )
            };
            if result != 0 {
                return Err(io::Error::from_raw_os_error(result as i32));
            }
            Key(handle)
        }
    };
    let current = startup_value(&key)?;
    if current.as_ref().is_some_and(|value| value != &expected) {
        return Err(io::Error::other(
            "Startup entry belongs to another path. It was preserved.",
        ));
    }
    let result = if enabled {
        let value = wide(&expected);
        unsafe {
            RegSetValueExW(
                key.0,
                wide(RUN_NAME).as_ptr(),
                0,
                REG_SZ,
                value.as_ptr().cast(),
                (value.len() * 2) as u32,
            )
        }
    } else if current.is_some() {
        unsafe { RegDeleteValueW(key.0, wide(RUN_NAME).as_ptr()) }
    } else {
        0
    };
    if result != 0 {
        return Err(io::Error::from_raw_os_error(result as i32));
    }
    Ok(())
}
pub fn preflight() -> String {
    // Read-only inventory, no virtual target creation. Bound child lifetime; kill only this child.
    let script = r#"$ErrorActionPreference='Stop'; $os=Get-CimInstance Win32_OperatingSystem; 'OS: '+$os.Caption+' '+$os.Version; $driver=Get-CimInstance Win32_SystemDriver -Filter "Name='ViGEmBus'"; if($driver){'ViGEmBus service: '+$driver.State; 'Driver file: '+$driver.PathName; $driverFile=Join-Path $env:SystemRoot 'System32\drivers\ViGEmBus.sys'; if(Test-Path -LiteralPath $driverFile){'Driver version: '+(Get-Item -LiteralPath $driverFile).VersionInfo.FileVersion}else{'Driver version unavailable'}}else{'ViGEmBus absent'}; $r=Get-NetFirewallRule -Name 'VirtualGamepad.Desktop.UDP' -ErrorAction SilentlyContinue; if($r){$r | Select-Object Name,Enabled,Direction,Action,Profile | Format-List | Out-String; $r | Get-NetFirewallApplicationFilter | Select-Object Program | Format-List | Out-String; $r | Get-NetFirewallPortFilter | Select-Object Protocol,LocalPort | Format-List | Out-String; $r | Get-NetFirewallAddressFilter | Select-Object RemoteAddress | Format-List | Out-String}else{'Owned firewall rule absent; manual setup may exist'}"#;
    let child = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .creation_flags(0x08000000)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn();
    let Ok(mut child) = child else {
        return "Setup inventory could not start; no system changes made.".into();
    };
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => {
                return match child.wait_with_output() {
                    Ok(output) => format!(
                        "{}\n{}\nPresence is informational; backend usability requires pairing. Supported installer 1.22.0 / driver 1.21.442.0. See Troubleshooting.",
                        String::from_utf8_lossy(&output.stdout),
                        String::from_utf8_lossy(&output.stderr)
                    ),
                    Err(e) => e.to_string(),
                };
            }
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                return e.to_string();
            }
            _ => {}
        }
        if std::time::Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return "Setup inventory timed out; no configuration changed.".into();
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stopped_port_is_configured_and_running_port_is_actual_binding() {
        let mut snapshot = DesktopStatus {
            phase: "Stopped",
            service: Some(crate::application::ServiceStatus {
                lifecycle: crate::application::Lifecycle::Stopped,
                bound: "0.0.0.0:8765".parse().unwrap(),
                addresses: vec![],
                development: false,
                backend: Default::default(),
                counters: Default::default(),
            }),
            ..Default::default()
        };
        snapshot.settings.port = 9876;
        assert_eq!(endpoint_label(&snapshot), "Configured UDP 9876 | Stopped");
        snapshot.phase = "Starting";
        assert_eq!(endpoint_label(&snapshot), "Configured UDP 9876 | Starting");
        snapshot.phase = "Running";
        assert_eq!(endpoint_label(&snapshot), "IPv4: unavailable | UDP 8765");
    }
    #[test]
    fn missing_editor_opens_chooser_but_other_failures_remain_errors() {
        let chosen = std::cell::Cell::new(false);
        editor_or_choose(
            || Err(io::Error::from(io::ErrorKind::NotFound)),
            || {
                chosen.set(true);
                Ok(())
            },
        )
        .unwrap();
        assert!(chosen.get());
        editor_or_choose(
            || Ok(()),
            || panic!("An available editor must not open the chooser"),
        )
        .unwrap();
        let error = editor_or_choose(
            || Err(io::Error::from(io::ErrorKind::PermissionDenied)),
            || panic!("Do not mask permission failures"),
        )
        .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
        assert!(
            editor_or_choose(
                || Err(io::Error::from(io::ErrorKind::NotFound)),
                || Err(io::Error::other("chooser unavailable"))
            )
            .is_err()
        );
    }
    #[test]
    fn status_distinguishes_stopped_and_actionable_failure() {
        let mut s = DesktopStatus {
            phase: "Stopped",
            ..Default::default()
        };
        assert_eq!(label(&s), "Stopped");
        s.phase = "Running";
        assert_eq!(label(&s), "Ready / awaiting fresh pairing");
        s.error = "disk failure".into();
        assert!(label(&s).contains("Action required"));
    }

    #[test]
    fn backend_failure_details_and_notification_are_actionable_and_once_per_run() {
        let diagnostic = crate::virtual_gamepad::BackendDiagnostic {
            operation: "wait ready",
            native_code: Some(483),
            attempt: 2,
            cleanup_failed: true,
        };
        let mut snapshot = DesktopStatus {
            phase: "Running",
            service: Some(crate::application::ServiceStatus {
                lifecycle: crate::application::Lifecycle::Running,
                bound: "127.0.0.1:8765".parse().unwrap(),
                addresses: vec![],
                development: false,
                backend: crate::application::BackendStatus::from(crate::input::OutputHealth {
                    fault: true,
                    diagnostic: Some(diagnostic),
                    ..Default::default()
                }),
                counters: Default::default(),
            }),
            ..Default::default()
        };
        let text = backend_details(&snapshot);
        for expected in [
            "wait ready",
            "483",
            "attempt 2",
            "timeout is possible",
            "cause is not confirmed",
            "Cleanup also failed",
            "fresh code",
        ] {
            assert!(text.contains(expected), "{expected}: {text}");
        }
        assert!(label(&snapshot).contains("Controller failed"));
        let mut notice = FaultNotice::default();
        let first = notice.next(&snapshot, false).unwrap();
        assert!(first.contains("483"));
        assert!(first.encode_utf16().count() < 256);
        for _ in 0..100 {
            assert!(notice.next(&snapshot, false).is_none());
        }
        snapshot.phase = "Stopped";
        assert!(notice.next(&snapshot, false).is_none());
        assert_eq!(backend_details(&snapshot), text);
        snapshot.phase = "Starting";
        let previous = snapshot.service.take();
        assert!(notice.next(&snapshot, false).is_none());
        snapshot.phase = "Running";
        snapshot.service = previous;
        assert!(notice.next(&snapshot, true).is_none());
        assert!(notice.next(&snapshot, false).is_some());
    }
}
