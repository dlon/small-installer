use core::{ptr, mem};
use core::ptr::NonNull;
use std::ffi::c_void;
use std::io;
use windows_sys::Win32::Foundation::{HWND, LPARAM};
use windows_sys::Win32::Graphics::Gdi::{UpdateWindow, COLOR_WINDOW, HBRUSH};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Controls::{InitCommonControls, InitCommonControlsEx, ICC_PROGRESS_CLASS, INITCOMMONCONTROLSEX, PBM_SETMARQUEE, PBM_SETPOS, PBM_SETRANGE};
use windows_sys::Win32::UI::WindowsAndMessaging::{CreateDialogParamW, DefWindowProcW, DispatchMessageW, GetMessageW, GetSystemMetrics, GetWindowLongPtrW, MoveWindow, PostQuitMessage, RegisterClassW, SendMessageW, SetForegroundWindow, ShowWindow, TranslateMessage, CS_HREDRAW, CS_VREDRAW, GWLP_HINSTANCE, SM_CXSCREEN, SM_CYSCREEN, SW_SHOW, WM_CREATE, WM_DESTROY, WM_SIZE, WNDCLASSW, WS_CHILD, WS_MAXIMIZEBOX, WS_MINIMIZEBOX, WS_OVERLAPPEDWINDOW, WS_THICKFRAME, WS_VISIBLE};
use windows_sys::Win32::UI::{Controls::PROGRESS_CLASS, WindowsAndMessaging::{MSG, CreateWindowExW}};
use windows_sys::w;

static mut HWND_PROGRESS: NonNull<c_void> = NonNull::dangling();

const RANGE_MIN: isize = 0;
const RANGE_MAX: isize = 1000;

unsafe extern "system" fn wndproc(window: HWND, msg: u32, wparam: usize, lparam: isize) -> isize {
    match msg {
        WM_CREATE => {
            let Some(hwnd_progress) = NonNull::new(unsafe {
                CreateWindowExW(
                    0,
                    PROGRESS_CLASS,
                    w!("Loader"),
                    WS_CHILD | WS_VISIBLE,
                    -1,
                    -1,
                    1,
                    1,
                    window,
                    ptr::null_mut(),
                    GetWindowLongPtrW(window, GWLP_HINSTANCE) as _,
                    ptr::null_mut(),
                )
            }) else {
                panic!("FIXME");
            };
        
            // TODO: error handling
        
            // Set range and initial position
            unsafe {
                SendMessageW(hwnd_progress.as_ptr(), PBM_SETRANGE, 0, makeLparam(RANGE_MIN, RANGE_MAX) as LPARAM);
                SendMessageW(hwnd_progress.as_ptr(), PBM_SETPOS, 100, 0);
                //SendMessageW(hwnd_progress.as_ptr(), PBM_SETMARQUEE, 1, 0);
            }

            HWND_PROGRESS = hwnd_progress;
        }

        WM_SIZE => {
            let new_width = lparam & 0xffff;
            let new_height = (lparam & 0xffff0000) >> 16;

            let padding = 10;
            let bar_height = 20;
            let mut bar_width = new_width - 2 * padding;
            let mut bar_y = (new_height - bar_height) / 2;

            // Ensure bar_width doesn't go negative if window is too small
            if bar_width < 20 {
                bar_width = 20;
            }
            if bar_y < 0 {
                bar_y = 0;
            }

            MoveWindow(HWND_PROGRESS.as_ptr(), padding as i32, bar_y as i32, bar_width as i32, bar_height as i32, 1);
        }

        WM_DESTROY => {
            PostQuitMessage(0);
        }

        _ => {
	        return DefWindowProcW(window, msg, wparam, lparam);
        }
    }

    0
}

const WNDCLASS: *const u16 = w!("MVDLOADERPROGRESS");

pub struct ProgressHandle {
    progress_window: HWND,
}

impl ProgressHandle {
    pub fn set_progress(&self, complete_frac: f32) {
        let clamped_percent = complete_frac.max(0f32).min(1f32);
        let progress = (clamped_percent * (RANGE_MAX - RANGE_MIN) as f32) as isize + RANGE_MIN;

        //println!("{progress}");

        //unsafe { SendMessageW(self.progress_window, PBM_SETPOS, progress as usize, 0) };
        unsafe { SendMessageW(HWND_PROGRESS.as_ptr(), PBM_SETPOS, progress as usize, 0) };
    }
}

// SAFETY: HWND is thread safe
unsafe impl Send for ProgressHandle {}

pub fn open() -> anyhow::Result<ProgressHandle> {
    let (result_tx, result_rx) = std::sync::mpsc::sync_channel(0);

    std::thread::spawn(move || {
        //unsafe { InitCommonControls(); }
        let mut controls_ex: INITCOMMONCONTROLSEX = unsafe { mem::zeroed() };
        controls_ex.dwSize = u32::try_from(mem::size_of_val(&controls_ex)).unwrap();
        controls_ex.dwICC = ICC_PROGRESS_CLASS;
        unsafe { InitCommonControlsEx(&controls_ex); }

        let hinstance = unsafe { GetModuleHandleW(ptr::null_mut()) };

        let mut wnd_class: WNDCLASSW = unsafe {
            mem::zeroed()
        };
        wnd_class.style = CS_HREDRAW | CS_VREDRAW;
        wnd_class.lpszClassName = WNDCLASS;
        wnd_class.lpfnWndProc = Some(wndproc);
        wnd_class.hInstance = hinstance;
        wnd_class.hbrBackground = (COLOR_WINDOW+1) as HBRUSH;

        // TODO: hinstance? background? cursor?
        if unsafe { RegisterClassW(&wnd_class) } == 0 {
            panic!("{:?}", std::io::Error::last_os_error());
        }

        let win_width = 350;
        let win_height = 100;

        let screen_width = unsafe { GetSystemMetrics(SM_CXSCREEN) };
        let screen_height = unsafe { GetSystemMetrics(SM_CYSCREEN) };
        let x = (screen_width - win_width) / 2;
        let y = (screen_height - win_height) / 2;

        let Some(main_window) = NonNull::new(unsafe {
            CreateWindowExW(
                0,
                WNDCLASS,
                w!("Loader"),
                //WS_CHILD | WS_VISIBLE,
                //WS_OVERLAPPEDWINDOW & !WS_THICKFRAME,
                WS_OVERLAPPEDWINDOW & !WS_MAXIMIZEBOX & !WS_MINIMIZEBOX,
                x,
                y,
                win_width,
                win_height,
                ptr::null_mut(),
                ptr::null_mut(),
                hinstance,
                ptr::null_mut(),
            )
        }) else {
            let err = std::io::Error::last_os_error();
            panic!("FIXME 0: {:?}", err.raw_os_error());
        };

        unsafe {
            ShowWindow(main_window.as_ptr(), SW_SHOW);
            UpdateWindow(main_window.as_ptr());
        };

        result_tx.send(Ok(ProgressHandle {
            progress_window: unsafe { HWND_PROGRESS }.as_ptr(),
        }))
        .unwrap();

        loop {
            let mut msg: MSG = unsafe { mem::zeroed() };
            //let ret = GetMessageW(&mut msg, hwnd_progress, 0, 0)
            let result = unsafe { GetMessageW(&mut msg, ptr::null_mut(), 0, 0) };
            if result == 0 {
                // TODO: handle error
                break;
            }
            unsafe {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    });

    result_rx.recv().unwrap()
}

/// This is identical to the macro function MAKELPARAM
fn makeLparam(a: LPARAM, b: LPARAM) -> LPARAM {
    b << 16 | a
}
