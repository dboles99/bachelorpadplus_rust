//! The Win32 calls themselves. The only module in the workspace with
//! `unsafe` in it (ADR-0033).
//!
//! One call per `unsafe` block, one `SAFETY:` comment per block, and every
//! string argument a [`WideString`], whose constructors guarantee the one
//! precondition every `PCWSTR` has: NUL-terminated, no NUL inside, alive for
//! the call. Everything that can be decided without Windows was decided in
//! safe code before these functions are reached.

use std::ptr;

use windows_sys::Win32::Foundation::{ERROR_MORE_DATA, ERROR_SUCCESS, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::Console::{
    ATTACH_PARENT_PROCESS, AttachConsole, GetStdHandle, STD_ERROR_HANDLE,
};
use windows_sys::Win32::System::Registry::{
    HKEY, HKEY_CLASSES_ROOT, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ, RegGetValueW,
};
use windows_sys::Win32::UI::Shell::{
    SHCNE_ASSOCCHANGED, SHCNF_IDLIST, SHChangeNotify, ShellExecuteW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    MB_ICONERROR, MB_OK, MB_SETFOREGROUND, MessageBoxW, SW_SHOWNORMAL,
};
use windows_sys::core::w;

use crate::Hive;
use crate::wide::{WideString, from_wide};

/// `AttachConsole(ATTACH_PARENT_PROCESS)`.
pub(crate) fn attach_parent_console() -> bool {
    // SAFETY: AttachConsole takes one integer by value and no pointers;
    // ATTACH_PARENT_PROCESS is its documented sentinel. It has no
    // precondition, and every failure (no parent console, one already
    // attached) is reported through the BOOL it returns.
    let attached = unsafe { AttachConsole(ATTACH_PARENT_PROCESS) };
    attached != 0
}

/// Whether `GetStdHandle(STD_ERROR_HANDLE)` is a handle at all.
pub(crate) fn standard_error_is_connected() -> bool {
    // SAFETY: GetStdHandle takes one integer by value and returns a handle
    // this code only compares and never uses or closes; it has no
    // precondition and reports failure as INVALID_HANDLE_VALUE, and "no
    // handle" as null.
    let handle = unsafe { GetStdHandle(STD_ERROR_HANDLE) };
    !handle.is_null() && handle != INVALID_HANDLE_VALUE
}

/// `MessageBoxW` with an error icon and an OK button, no owner.
pub(crate) fn message_box(title: &WideString, body: &WideString) -> bool {
    // MB_SETFOREGROUND because a crash box with no owner window otherwise
    // opens behind whatever the user was looking at, and a crash nobody
    // notices is the failure this box exists to prevent.
    let style = MB_OK | MB_ICONERROR | MB_SETFOREGROUND;
    // SAFETY: `body` and `title` are WideStrings -- NUL-terminated, no
    // interior NUL -- borrowed for the whole call, and MessageBoxW only reads
    // them. A null owner HWND is documented as "no owner window".
    let pressed = unsafe { MessageBoxW(ptr::null_mut(), body.as_ptr(), title.as_ptr(), style) };
    // Zero is MessageBoxW's only failure value; any button is success.
    pressed != 0
}

/// `SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, null, null)`.
pub(crate) fn notify_associations_changed() -> bool {
    // The binding takes the event id as i32 while the constant is a u32;
    // SHCNE_ASSOCCHANGED is 0x0800_0000 and fits, so this never fails.
    let Ok(event) = i32::try_from(SHCNE_ASSOCCHANGED) else {
        return false;
    };
    // SAFETY: SHCNE_ASSOCCHANGED with SHCNF_IDLIST is documented to take null
    // for both item pointers, so no memory is passed at all; SHChangeNotify
    // returns nothing and has no other precondition.
    unsafe { SHChangeNotify(event, SHCNF_IDLIST, ptr::null(), ptr::null()) };
    true
}

/// `ShellExecuteW(null, "open", uri, null, null, SW_SHOWNORMAL)`.
///
/// The caller has already checked `uri` is an `ms-settings:` URI; this
/// function trusts that and checks nothing, which is why it is not public.
pub(crate) fn shell_open(uri: &WideString) -> Result<(), isize> {
    // SAFETY: the verb is a `w!` literal (static, NUL-terminated) and `uri` a
    // WideString borrowed for the whole call, and ShellExecuteW only reads
    // both. Null parameters and directory are documented as "none" and "the
    // current directory"; a null HWND as "no owner window".
    let instance = unsafe {
        ShellExecuteW(
            ptr::null_mut(),
            w!("open"),
            uri.as_ptr(),
            ptr::null(),
            ptr::null(),
            SW_SHOWNORMAL,
        )
    };
    // Not a real HINSTANCE: a value above 32 means success, anything else is
    // an error code, which is how ShellExecuteW has reported since Win16.
    let code = instance as isize;
    if code > 32 { Ok(()) } else { Err(code) }
}

/// One `REG_SZ` value via `RegGetValueW`, growing the buffer if it is short.
pub(crate) fn read_string(
    hive: Hive,
    subkey: &WideString,
    value: Option<&WideString>,
) -> Option<String> {
    let root: HKEY = match hive {
        Hive::CurrentUser => HKEY_CURRENT_USER,
        Hive::LocalMachine => HKEY_LOCAL_MACHINE,
        Hive::ClassesRoot => HKEY_CLASSES_ROOT,
    };
    // Null asks for the key's default value, which RegGetValueW documents.
    let value_ptr = value.map_or(ptr::null(), WideString::as_ptr);
    let mut buffer: Vec<u16> = vec![0; 128];
    // A value can grow between the call that reports its size and the call
    // that reads it; a few attempts cover that without looping forever.
    for _ in 0..4 {
        let mut size = u32::try_from(buffer.len().checked_mul(2)?).ok()?;
        // SAFETY: `subkey` and (when present) the value name are WideStrings
        // borrowed for the call, and a null value name is documented as the
        // default value. `buffer` is writable for exactly `size` bytes, set
        // on the line above from its length, and RegGetValueW writes no more
        // than `size` bytes into it -- reporting ERROR_MORE_DATA and the
        // needed size instead when it would. `size` is a live local; the type
        // out-pointer may be null. RRF_RT_REG_SZ refuses any other type, so
        // what is written is UTF-16 text and RegGetValueW NUL-terminates it.
        let status = unsafe {
            RegGetValueW(
                root,
                subkey.as_ptr(),
                value_ptr,
                RRF_RT_REG_SZ,
                ptr::null_mut(),
                buffer.as_mut_ptr().cast(),
                &mut size,
            )
        };
        let bytes = usize::try_from(size).ok()?;
        match status {
            ERROR_SUCCESS => return from_wide(buffer.get(..bytes / 2)?),
            ERROR_MORE_DATA => buffer.resize(bytes.div_ceil(2) + 1, 0),
            _ => return None,
        }
    }
    None
}
