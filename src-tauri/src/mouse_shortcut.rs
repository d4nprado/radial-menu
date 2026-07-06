#[cfg(target_os = "windows")]
mod platform {
    use crate::commands;
    use std::{
        sync::{
            atomic::{AtomicBool, AtomicU32, AtomicU8, Ordering},
            mpsc::{self, RecvTimeoutError, SyncSender},
            Arc, Mutex, OnceLock,
        },
        thread::{self, JoinHandle},
        time::Duration,
    };
    use tauri::{Emitter, Manager};
    use windows::Win32::{
        Foundation::{HINSTANCE, LPARAM, LRESULT, WPARAM},
        System::{LibraryLoader::GetModuleHandleW, Threading::GetCurrentThreadId},
        UI::WindowsAndMessaging::{
            CallNextHookEx, GetMessageW, PostThreadMessageW, SetWindowsHookExW,
            UnhookWindowsHookEx, MSG, MSLLHOOKSTRUCT, WH_MOUSE_LL, WM_LBUTTONDOWN, WM_LBUTTONUP,
            WM_MBUTTONDOWN, WM_MBUTTONUP, WM_QUIT, WM_RBUTTONDOWN, WM_RBUTTONUP, WM_XBUTTONDOWN,
            WM_XBUTTONUP, XBUTTON1, XBUTTON2,
        },
    };

    enum HookEvent {
        Triggered,
        Captured(u8),
        InvalidCapture,
        Stop,
    }

    struct HookState {
        sender: SyncSender<HookEvent>,
        target: AtomicU8,
        capturing: AtomicBool,
        pressed: AtomicU32,
        shutdown: AtomicBool,
    }

    static HOOK_STATE: OnceLock<Arc<HookState>> = OnceLock::new();

    pub struct MouseShortcutManager {
        state: Arc<HookState>,
        hook_thread_id: AtomicU32,
        hook_thread: Mutex<Option<JoinHandle<()>>>,
        dispatch_thread: Mutex<Option<JoinHandle<()>>>,
        stopped: AtomicBool,
        startup_error: Option<String>,
    }

    impl MouseShortcutManager {
        pub fn start(app: tauri::AppHandle) -> Result<Self, String> {
            let (event_tx, event_rx) = mpsc::sync_channel(8);
            let state = Arc::new(HookState {
                sender: event_tx,
                target: AtomicU8::new(0),
                capturing: AtomicBool::new(false),
                pressed: AtomicU32::new(0),
                shutdown: AtomicBool::new(false),
            });
            HOOK_STATE
                .set(state.clone())
                .map_err(|_| "O listener global do mouse já foi iniciado.".to_string())?;

            let state_for_dispatch = state.clone();
            let dispatch_thread = thread::Builder::new()
                .name("orbit-mouse-dispatch".into())
                .spawn(move || {
                    while !state_for_dispatch.shutdown.load(Ordering::SeqCst) {
                        match event_rx.recv_timeout(Duration::from_millis(100)) {
                            Ok(HookEvent::Triggered) => {
                                let app_for_task = app.clone();
                                let _ = app.run_on_main_thread(move || {
                                    if let Some(window) = app_for_task.get_webview_window("main") {
                                        if let Err(error) = commands::show_window_at_cursor(&window)
                                        {
                                            eprintln!("Falha ao abrir o menu pelo mouse: {error}");
                                        }
                                    }
                                });
                            }
                            Ok(HookEvent::Captured(button)) => {
                                let _ =
                                    app.emit("mouse-shortcut-captured", format!("Mouse{button}"));
                            }
                            Ok(HookEvent::InvalidCapture) => {
                                let _ = app.emit(
                                    "mouse-shortcut-capture-error",
                                    "Botão esquerdo/direito não pode ser usado como atalho.",
                                );
                            }
                            Ok(HookEvent::Stop) => break,
                            Err(RecvTimeoutError::Timeout) => {}
                            Err(RecvTimeoutError::Disconnected) => break,
                        }
                    }
                })
                .map_err(|error| {
                    format!("Não foi possível iniciar o dispatcher do mouse: {error}")
                })?;

            let (ready_tx, ready_rx) = mpsc::sync_channel(1);
            let thread_id = Arc::new(AtomicU32::new(0));
            let hook_thread_id = thread_id.clone();
            let hook_thread = match thread::Builder::new()
                .name("orbit-mouse-hook".into())
                .spawn(move || unsafe {
                    hook_thread_id.store(GetCurrentThreadId(), Ordering::SeqCst);
                    let module = match GetModuleHandleW(None) {
                        Ok(module) => HINSTANCE(module.0),
                        Err(error) => {
                            let _ = ready_tx.send(Err(error.to_string()));
                            return;
                        }
                    };
                    let hook = match SetWindowsHookExW(
                        WH_MOUSE_LL,
                        Some(mouse_hook_proc),
                        Some(module),
                        0,
                    ) {
                        Ok(hook) => hook,
                        Err(error) => {
                            let _ = ready_tx.send(Err(error.to_string()));
                            return;
                        }
                    };

                    let _ = ready_tx.send(Ok(()));
                    let mut message = MSG::default();
                    while GetMessageW(&mut message, None, 0, 0).0 > 0 {}
                    let _ = UnhookWindowsHookEx(hook);
                }) {
                Ok(thread) => thread,
                Err(error) => {
                    let _ = state.sender.try_send(HookEvent::Stop);
                    let _ = dispatch_thread.join();
                    return Err(format!("Não foi possível iniciar o hook do mouse: {error}"));
                }
            };

            match ready_rx.recv() {
                Ok(Ok(())) => {}
                Ok(Err(error)) => {
                    let _ = state.sender.try_send(HookEvent::Stop);
                    let _ = hook_thread.join();
                    let _ = dispatch_thread.join();
                    return Err(format!(
                        "Não foi possível registrar o atalho global do mouse: {error}"
                    ));
                }
                Err(_) => {
                    let _ = state.sender.try_send(HookEvent::Stop);
                    let _ = hook_thread.join();
                    let _ = dispatch_thread.join();
                    return Err("O hook do mouse encerrou durante a inicialização.".into());
                }
            }

            Ok(Self {
                state,
                hook_thread_id: AtomicU32::new(thread_id.load(Ordering::SeqCst)),
                hook_thread: Mutex::new(Some(hook_thread)),
                dispatch_thread: Mutex::new(Some(dispatch_thread)),
                stopped: AtomicBool::new(false),
                startup_error: None,
            })
        }

        pub fn unavailable(error: String) -> Self {
            let (sender, receiver) = mpsc::sync_channel(1);
            drop(receiver);
            Self {
                state: Arc::new(HookState {
                    sender,
                    target: AtomicU8::new(0),
                    capturing: AtomicBool::new(false),
                    pressed: AtomicU32::new(0),
                    shutdown: AtomicBool::new(true),
                }),
                hook_thread_id: AtomicU32::new(0),
                hook_thread: Mutex::new(None),
                dispatch_thread: Mutex::new(None),
                stopped: AtomicBool::new(false),
                startup_error: Some(error),
            }
        }

        pub fn set_button(&self, button: u8) -> Result<(), String> {
            if let Some(error) = &self.startup_error {
                return Err(format!(
                    "O listener global do mouse não está disponível: {error}"
                ));
            }
            if !(3..=5).contains(&button) {
                return Err("Escolha Mouse3, Mouse4 ou Mouse5.".into());
            }
            self.state.target.store(button, Ordering::SeqCst);
            Ok(())
        }

        pub fn disable(&self) {
            self.state.target.store(0, Ordering::SeqCst);
        }

        pub fn start_capture(&self) -> Result<(), String> {
            if let Some(error) = &self.startup_error {
                return Err(format!(
                    "A captura global do mouse não está disponível: {error}"
                ));
            }
            self.state.capturing.store(true, Ordering::SeqCst);
            Ok(())
        }

        pub fn cancel_capture(&self) {
            self.state.capturing.store(false, Ordering::SeqCst);
        }

        pub fn stop(&self) {
            if self.stopped.swap(true, Ordering::SeqCst) {
                return;
            }
            self.disable();
            self.state.shutdown.store(true, Ordering::SeqCst);
            let thread_id = self.hook_thread_id.load(Ordering::SeqCst);
            if thread_id != 0 {
                unsafe {
                    let _ = PostThreadMessageW(thread_id, WM_QUIT, WPARAM(0), LPARAM(0));
                }
            }
            let _ = self.state.sender.try_send(HookEvent::Stop);

            if let Ok(mut thread) = self.hook_thread.lock() {
                if let Some(thread) = thread.take() {
                    let _ = thread.join();
                }
            }
            if let Ok(mut thread) = self.dispatch_thread.lock() {
                if let Some(thread) = thread.take() {
                    let _ = thread.join();
                }
            }
        }
    }

    unsafe extern "system" fn mouse_hook_proc(
        code: i32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        if code >= 0 {
            if let Some(state) = HOOK_STATE.get() {
                if let Some((button, pressed)) = button_event(wparam.0 as u32, lparam) {
                    let mask = 1u32 << button;
                    if pressed {
                        let repeated = state.pressed.fetch_or(mask, Ordering::SeqCst) & mask != 0;
                        if !repeated {
                            if state.capturing.load(Ordering::SeqCst) {
                                if button <= 2 {
                                    let _ = state.sender.try_send(HookEvent::InvalidCapture);
                                } else {
                                    state.capturing.store(false, Ordering::SeqCst);
                                    let _ = state.sender.try_send(HookEvent::Captured(button));
                                }
                            } else if state.target.load(Ordering::SeqCst) == button {
                                let _ = state.sender.try_send(HookEvent::Triggered);
                            }
                        }
                    } else {
                        state.pressed.fetch_and(!mask, Ordering::SeqCst);
                    }
                }
            }
        }

        // Do not consume the event: Mouse3/4/5 keep their normal behavior.
        CallNextHookEx(None, code, wparam, lparam)
    }

    unsafe fn button_event(message: u32, lparam: LPARAM) -> Option<(u8, bool)> {
        match message {
            WM_LBUTTONDOWN => Some((1, true)),
            WM_LBUTTONUP => Some((1, false)),
            WM_RBUTTONDOWN => Some((2, true)),
            WM_RBUTTONUP => Some((2, false)),
            WM_MBUTTONDOWN => Some((3, true)),
            WM_MBUTTONUP => Some((3, false)),
            WM_XBUTTONDOWN | WM_XBUTTONUP => {
                let data = &*(lparam.0 as *const MSLLHOOKSTRUCT);
                let xbutton = ((data.mouseData >> 16) & 0xffff) as u16;
                let button = match xbutton {
                    XBUTTON1 => 4,
                    XBUTTON2 => 5,
                    _ => return None,
                };
                Some((button, message == WM_XBUTTONDOWN))
            }
            _ => None,
        }
    }
}

#[cfg(not(target_os = "windows"))]
mod platform {
    pub struct MouseShortcutManager;

    impl MouseShortcutManager {
        pub fn start(_app: tauri::AppHandle) -> Result<Self, String> {
            Ok(Self)
        }

        pub fn set_button(&self, _button: u8) -> Result<(), String> {
            Err("Atalhos globais de mouse estão disponíveis apenas no Windows.".into())
        }

        pub fn unavailable(_error: String) -> Self {
            Self
        }

        pub fn disable(&self) {}
        pub fn start_capture(&self) -> Result<(), String> {
            Err("A captura global do mouse está disponível apenas no Windows.".into())
        }
        pub fn cancel_capture(&self) {}
        pub fn stop(&self) {}
    }
}

pub use platform::MouseShortcutManager;

#[tauri::command]
pub fn start_mouse_shortcut_capture(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager;
    app.state::<MouseShortcutManager>().start_capture()
}

#[tauri::command]
pub fn cancel_mouse_shortcut_capture(app: tauri::AppHandle) {
    use tauri::Manager;
    app.state::<MouseShortcutManager>().cancel_capture();
}
