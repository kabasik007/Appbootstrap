//! Windows multimedia hardware keys (VK_MEDIA_*). RegisterHotKey is called
//! only on a dedicated message-loop thread, never on Slint/audio threads.
//! Registration is best-effort: if another program already owns a key, we
//! leave it alone and keep UI controls functional.
use std::{
    ptr,
    sync::mpsc::{self, Receiver},
    thread::{self, JoinHandle},
};
use windows_sys::Win32::{
    System::Threading::GetCurrentThreadId,
    UI::{
        Input::KeyboardAndMouse::{RegisterHotKey, UnregisterHotKey, MOD_NOREPEAT},
        WindowsAndMessaging::{
            GetMessageW, PeekMessageW, PostThreadMessageW,
            MSG, PM_NOREMOVE, WM_HOTKEY, WM_QUIT,
        },
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaAction { PlayPause, Next, Previous, Stop }

const KEYS: [(i32, u32, MediaAction); 4] = [
    (1, 0xB3, MediaAction::PlayPause), // VK_MEDIA_PLAY_PAUSE
    (2, 0xB0, MediaAction::Next),      // VK_MEDIA_NEXT_TRACK
    (3, 0xB1, MediaAction::Previous),  // VK_MEDIA_PREV_TRACK
    (4, 0xB2, MediaAction::Stop),      // VK_MEDIA_STOP
];

pub struct MediaKeys {
    pub events: Receiver<MediaAction>,
    thread_id: u32,
    worker: Option<JoinHandle<()>>,
}
impl MediaKeys {
    pub fn start() -> Self {
        let (actions, events) = mpsc::channel();
        let (ready, receiver) = mpsc::sync_channel(1);
        let worker = thread::Builder::new().name("zilla-windows-media-keys".into())
            .spawn(move || {
                // Force a message queue before publishing thread id. Without
                // this, PostThreadMessage(WM_QUIT) could race thread startup.
                let mut message: MSG = unsafe { std::mem::zeroed() };
                unsafe { PeekMessageW(&mut message, ptr::null_mut(), 0, 0, PM_NOREMOVE); }
                let mut registered = Vec::new();
                for (id, vk, _) in KEYS {
                    let ok = unsafe { RegisterHotKey(ptr::null_mut(), id, MOD_NOREPEAT, vk) } != 0;
                    if ok { registered.push(id); }
                }
                let _ = ready.send(unsafe { GetCurrentThreadId() });
                loop {
                    let code = unsafe { GetMessageW(&mut message, ptr::null_mut(), 0, 0) };
                    if code <= 0 { break; }
                    if message.message != WM_HOTKEY { continue; }
                    if let Some((_, _, action)) =
                        KEYS.iter().find(|(id, _, _)| *id as usize == message.wParam)
                    {
                        if actions.send(*action).is_err() { break; }
                    }
                }
                for id in registered {
                    unsafe { UnregisterHotKey(ptr::null_mut(), id); }
                }
            }).ok();
        let thread_id = if worker.is_some() {
            receiver.recv().unwrap_or(0)
        } else { 0 };
        Self { events, thread_id, worker }
    }
    pub fn shutdown(mut self) {
        if self.thread_id != 0 {
            let posted = unsafe { PostThreadMessageW(self.thread_id, WM_QUIT, 0, 0) } != 0;
            if posted {
                if let Some(thread) = self.worker.take() { let _ = thread.join(); }
            }
        }
    }
}
