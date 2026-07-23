use std::{
    ffi::CString,
    os::raw::{c_int, c_uchar, c_ulong},
    ptr,
};

use x11_dl::xlib;

use super::LinuxWindowProvider;
use crate::recorder::types::{
    ProtectableWindow, ProtectableWindowList, ProtectableWindowProviderStatus,
    WindowProtectionStatus,
};

pub(super) struct X11WindowProvider {
    display: *mut xlib::Display,
    root: xlib::Window,
    xlib: xlib::Xlib,
}

impl X11WindowProvider {
    pub(super) fn open() -> Option<Self> {
        let xlib = xlib::Xlib::open().ok()?;
        // SAFETY: XOpenDisplay reads process environment and returns either a valid
        // display handle or null. The handle is closed in Drop.
        let display = unsafe { (xlib.XOpenDisplay)(ptr::null()) };
        if display.is_null() {
            return None;
        }
        // SAFETY: display is a live Xlib display handle until Drop.
        let root = unsafe { (xlib.XDefaultRootWindow)(display) };
        Some(Self {
            display,
            root,
            xlib,
        })
    }

    fn protectable_window(&self, window: xlib::Window) -> Option<ProtectableWindow> {
        if window == 0 {
            return None;
        }
        let process_id = self
            .window_property_u32(window, "_NET_WM_PID")
            .and_then(|pid| u32::try_from(pid).ok());
        let wm_class = self.window_text_property(window, "WM_CLASS");
        let class_parts = wm_class
            .as_deref()
            .map(|value| {
                value
                    .split('\0')
                    .filter(|part| !part.trim().is_empty())
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let class_name = class_parts.last().cloned();
        let process_name = process_id
            .and_then(process_name_for_pid)
            .or_else(|| class_name.clone());
        let title = self
            .window_text_property(window, "_NET_WM_NAME")
            .or_else(|| self.window_text_property(window, "WM_NAME"));

        Some(ProtectableWindow {
            app_identity: class_name.or_else(|| process_name.clone()),
            id: format!("x11:{window}"),
            platform: "linux".to_string(),
            process_id,
            process_name,
            title,
        })
    }

    fn window_property_windows(&self, window: xlib::Window, property: &str) -> Vec<xlib::Window> {
        let Some(bytes) = self.window_property_bytes(window, property, xlib::AnyPropertyType as _)
        else {
            return Vec::new();
        };
        bytes
            .chunks_exact(std::mem::size_of::<c_ulong>())
            .map(|chunk| {
                let mut raw = [0_u8; std::mem::size_of::<c_ulong>()];
                raw.copy_from_slice(chunk);
                c_ulong::from_ne_bytes(raw)
            })
            .collect()
    }

    fn window_property_u32(&self, window: xlib::Window, property: &str) -> Option<u64> {
        self.window_property_bytes(window, property, xlib::XA_CARDINAL)
            .and_then(|bytes| {
                bytes
                    .chunks_exact(std::mem::size_of::<c_ulong>())
                    .next()
                    .map(|chunk| {
                        let mut raw = [0_u8; std::mem::size_of::<c_ulong>()];
                        raw.copy_from_slice(chunk);
                        c_ulong::from_ne_bytes(raw) as u64
                    })
            })
    }

    fn window_text_property(&self, window: xlib::Window, property: &str) -> Option<String> {
        self.window_property_bytes(window, property, xlib::AnyPropertyType as _)
            .and_then(|bytes| {
                if property == "WM_CLASS" {
                    Some(
                        bytes
                            .split(|byte| *byte == 0)
                            .map(|part| String::from_utf8_lossy(part).trim().to_string())
                            .filter(|part| !part.is_empty())
                            .collect::<Vec<_>>()
                            .join("\0"),
                    )
                } else {
                    String::from_utf8(bytes)
                        .ok()
                        .map(|value| value.trim_matches('\0').trim().to_string())
                }
            })
            .filter(|value| !value.is_empty())
    }

    fn window_property_bytes(
        &self,
        window: xlib::Window,
        property: &str,
        required_type: xlib::Atom,
    ) -> Option<Vec<u8>> {
        let property = CString::new(property).ok()?;
        // SAFETY: display is live; property is a valid C string for this call.
        let atom = unsafe { (self.xlib.XInternAtom)(self.display, property.as_ptr(), xlib::False) };
        if atom == 0 {
            return None;
        }

        let mut actual_type: xlib::Atom = 0;
        let mut actual_format: c_int = 0;
        let mut item_count: c_ulong = 0;
        let mut bytes_after: c_ulong = 0;
        let mut property_data: *mut c_uchar = ptr::null_mut();
        // SAFETY: all out pointers are valid for the duration of the call. Xlib
        // owns property_data on success and it is released with XFree below.
        let status = unsafe {
            (self.xlib.XGetWindowProperty)(
                self.display,
                window,
                atom,
                0,
                1024 * 1024,
                xlib::False,
                required_type,
                &mut actual_type,
                &mut actual_format,
                &mut item_count,
                &mut bytes_after,
                &mut property_data,
            )
        };
        if status != xlib::Success as c_int || property_data.is_null() || actual_format <= 0 {
            if !property_data.is_null() {
                // SAFETY: property_data was allocated by XGetWindowProperty.
                unsafe {
                    (self.xlib.XFree)(property_data.cast());
                }
            }
            return None;
        }

        let bytes_per_item = match actual_format {
            8 => std::mem::size_of::<c_uchar>(),
            16 => 2,
            32 => std::mem::size_of::<c_ulong>(),
            _ => return None,
        };
        let byte_len = usize::try_from(item_count)
            .ok()?
            .saturating_mul(bytes_per_item);
        // SAFETY: XGetWindowProperty returned property_data with item_count items
        // of actual_format bits. We copy it before freeing.
        let bytes =
            unsafe { std::slice::from_raw_parts(property_data.cast::<u8>(), byte_len) }.to_vec();
        // SAFETY: property_data was allocated by XGetWindowProperty.
        unsafe {
            (self.xlib.XFree)(property_data.cast());
        }

        Some(bytes)
    }
}

impl LinuxWindowProvider for X11WindowProvider {
    fn focused_window(&self) -> Option<ProtectableWindow> {
        self.window_property_windows(self.root, "_NET_ACTIVE_WINDOW")
            .into_iter()
            .next()
            .and_then(|window| self.protectable_window(window))
    }

    fn list_windows(&self) -> ProtectableWindowList {
        let windows = self
            .window_property_windows(self.root, "_NET_CLIENT_LIST")
            .into_iter()
            .filter_map(|window| self.protectable_window(window))
            .collect();

        ProtectableWindowList {
            provider: Some("linux-x11".to_string()),
            status: ProtectableWindowProviderStatus::Available,
            windows,
        }
    }

    fn status(&self) -> WindowProtectionStatus {
        WindowProtectionStatus {
            message: "Focused-window detection is available through X11.".to_string(),
            platform: "linux".to_string(),
            provider: Some("linux-x11".to_string()),
            status: ProtectableWindowProviderStatus::Available,
        }
    }
}

impl Drop for X11WindowProvider {
    fn drop(&mut self) {
        if !self.display.is_null() {
            // SAFETY: display is owned by this provider and closed once here.
            unsafe {
                (self.xlib.XCloseDisplay)(self.display);
            }
        }
    }
}

fn process_name_for_pid(pid: u32) -> Option<String> {
    let comm_path = format!("/proc/{pid}/comm");
    std::fs::read_to_string(comm_path)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}
