//! Standalone-only touch-ups to the app window on Windows: a fixed size (no resize border, no
//! maximize button) and the Simple Osc icon in the title bar and taskbar.
//!
//! nih-plug's standalone wrapper creates the window internally and doesn't expose it, so this
//! waits for the window to appear and adjusts it. Plugin hosts already can't resize the plugin;
//! nih-plug tells them so.

#[cfg(windows)]
pub fn fix_up_when_open(title: &'static str) {
    std::thread::spawn(move || {
        // The window appears within a moment of startup; give up quietly after a few seconds
        for _ in 0..200 {
            if let Some(hwnd) = win::find_window(title) {
                win::fix_up(hwnd);
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
    });
}

#[cfg(not(windows))]
pub fn fix_up_when_open(_title: &'static str) {}

#[cfg(windows)]
mod win {
    use std::ptr::null_mut;
    use windows_sys::Win32::Foundation::{BOOL, HWND, LPARAM, RECT};
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::System::Threading::GetCurrentProcessId;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        AdjustWindowRectEx, EnumWindows, GetClientRect, GetSystemMetrics, GetWindowLongPtrW,
        GetWindowTextW, GetWindowThreadProcessId, LoadImageW, SendMessageW, SetWindowLongPtrW,
        SetWindowPos, GWL_EXSTYLE, GWL_STYLE, ICON_BIG, ICON_SMALL, IMAGE_ICON, LR_DEFAULTCOLOR,
        SM_CXICON, SM_CXSMICON, SWP_FRAMECHANGED, SWP_NOMOVE, SWP_NOZORDER, WM_SETICON,
        WS_MAXIMIZEBOX, WS_SIZEBOX,
    };

    /// Resource id of the icon `build.rs` embeds.
    const ICON_ID: u16 = 1;

    struct Search {
        title: Vec<u16>,
        pid: u32,
        found: HWND,
    }

    unsafe extern "system" fn visit(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let search = &mut *(lparam as *mut Search);
        let mut pid = 0;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if pid == search.pid {
            let mut buf = [0u16; 64];
            let len = GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32);
            if len > 0 && buf[..len as usize] == search.title[..] {
                search.found = hwnd;
                return 0; // stop
            }
        }
        1 // keep going
    }

    /// This process's top-level window with the given title.
    pub fn find_window(title: &str) -> Option<HWND> {
        let mut search = Search {
            title: title.encode_utf16().collect(),
            pid: unsafe { GetCurrentProcessId() },
            found: null_mut(),
        };
        unsafe { EnumWindows(Some(visit), &mut search as *mut Search as LPARAM) };
        (!search.found.is_null()).then_some(search.found)
    }

    pub fn fix_up(hwnd: HWND) {
        unsafe {
            // Drop the resize border and maximize button, then resize the frame so the content
            // area stays exactly the same size
            let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32 & !(WS_SIZEBOX | WS_MAXIMIZEBOX);
            let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
            let mut rect: RECT = std::mem::zeroed();
            GetClientRect(hwnd, &mut rect);
            SetWindowLongPtrW(hwnd, GWL_STYLE, style as isize);
            AdjustWindowRectEx(&mut rect, style, 0, ex_style);
            SetWindowPos(
                hwnd,
                null_mut(),
                0,
                0,
                rect.right - rect.left,
                rect.bottom - rect.top,
                SWP_NOMOVE | SWP_NOZORDER | SWP_FRAMECHANGED,
            );

            // Title bar and taskbar icon
            let module = GetModuleHandleW(std::ptr::null());
            for (which, metric) in [(ICON_SMALL, SM_CXSMICON), (ICON_BIG, SM_CXICON)] {
                let size = GetSystemMetrics(metric);
                let icon = LoadImageW(
                    module,
                    ICON_ID as usize as *const u16,
                    IMAGE_ICON,
                    size,
                    size,
                    LR_DEFAULTCOLOR,
                );
                if !icon.is_null() {
                    SendMessageW(hwnd, WM_SETICON, which as usize, icon as isize);
                }
            }
        }
    }
}
