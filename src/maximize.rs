#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::{HWND, LPARAM};
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowTextLengthW, GetWindowTextW, SetForegroundWindow, ShowWindow, SW_MAXIMIZE,
};

/*********************
 * Windows Functions *
 ********************/

#[cfg(target_os = "windows")]
/// Maximize the window with the title "Status Board"
// TODO: [WINDOWS] Check if multiple windows; do nothing if so
pub fn maximize_window(target: &str) {
    unsafe extern "system" fn lpenumfunc(hwnd: HWND, lparam: LPARAM) -> i32 {
        let target = &*(lparam as *const String);
        let length = GetWindowTextLengthW(hwnd);
        let mut buffer = vec![0u16; length as usize + 1];
        GetWindowTextW(hwnd, buffer.as_mut_ptr(), length + 1);
        let title = String::from_utf16_lossy(&buffer);
        if title.contains(target) {
            SetForegroundWindow(hwnd);
            ShowWindow(hwnd, SW_MAXIMIZE);
        }
        1
    }

    let target_string = target.to_string();
    let lparam = &target_string as *const String as LPARAM;
    unsafe {
        EnumWindows(Some(lpenumfunc), lparam);
    }
}

/***********************
 * Non-Windows Systems *
 **********************/

#[cfg(not(target_os = "windows"))]
/// Placeholder function for non-Windows operating systems
/// - Prints a message to the console
pub fn maximize_window(target: &str) {
    spit(target);
    spit("Maximize window not supported on this platform.");
}
