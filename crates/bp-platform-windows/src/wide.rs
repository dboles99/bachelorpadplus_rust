//! UTF-16 for Win32, with the NUL rule enforced by a type.
//!
//! Every `PCWSTR` parameter Win32 takes has the same precondition: a pointer
//! to UTF-16 code units ending in exactly one NUL, readable for the duration
//! of the call. Established once here, in safe code, the `SAFETY:` comments
//! in `sys` can cite it instead of re-arguing it at each call.

// `WideString` is only constructed by the Windows arms of the public
// functions; on other targets the tests are its only users.
#![cfg_attr(not(windows), allow(dead_code))]

/// `text` as UTF-16 code units followed by one NUL, or `None` if `text`
/// already contains a NUL.
///
/// Refused rather than truncated, because Win32 would silently stop at the
/// first NUL: a registry path `Software\x<NUL>\y` is read as `Software\x`, a
/// different key from the one asked for.
#[must_use]
pub fn to_wide(text: &str) -> Option<Vec<u16>> {
    if text.contains('\0') {
        return None;
    }
    Some(text.encode_utf16().chain(std::iter::once(0)).collect())
}

/// The string in `units` up to the first NUL (or the end), or `None` if
/// those units are not valid UTF-16.
///
/// A registry value's buffer may hold its terminator, an extra one, or --
/// for a value written carelessly by somebody else -- none; stopping at the
/// first NUL reads all three the same way.
#[must_use]
pub fn from_wide(units: &[u16]) -> Option<String> {
    let end = units
        .iter()
        .position(|&unit| unit == 0)
        .unwrap_or(units.len());
    String::from_utf16(units.get(..end)?).ok()
}

/// UTF-16 that is NUL-terminated and has no NUL inside.
///
/// The invariant is the whole point, and both constructors keep it: `new`
/// refuses a string with a NUL in it, `lossy` replaces each with a space.
/// Nothing can reach the buffer to break it afterwards.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WideString(Vec<u16>);

impl WideString {
    /// `text`, or `None` if it holds a NUL.
    pub(crate) fn new(text: &str) -> Option<Self> {
        to_wide(text).map(Self)
    }

    /// `text` with every NUL replaced by a space, for text that must be shown
    /// whole -- a crash message -- rather than refused.
    pub(crate) fn lossy(text: &str) -> Self {
        Self(to_wide(&text.replace('\0', " ")).unwrap_or_else(|| vec![0]))
    }

    /// The pointer Win32 wants. Valid, and NUL-terminated, for as long as
    /// `self` is borrowed.
    pub(crate) fn as_ptr(&self) -> *const u16 {
        self.0.as_ptr()
    }

    /// The code units, terminator included. For tests of the invariant.
    #[cfg(test)]
    pub(crate) fn units(&self) -> &[u16] {
        &self.0
    }
}
