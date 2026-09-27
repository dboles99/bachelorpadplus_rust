//! The handful of Win32 calls the product needs, behind safe functions
//! (ADR-0033).
//!
//! `bp-platform` holds the rules -- which keys a registration writes, which
//! Settings page to send the user to, what a ProgID means -- as pure
//! functions that both legs of CI run. What it cannot do is ask Windows
//! anything, because it takes no dependency and forbids `unsafe`. This crate
//! is the other half: each public function wraps one Win32 call, and nothing
//! here decides anything that could be decided in `bp-platform` instead.
//!
//! ## What is here
//!
//! | function | Win32 | used by |
//! |---|---|---|
//! | [`attach_parent_console`] | `AttachConsole(ATTACH_PARENT_PROCESS)` | `--version`/`--help` in a GUI-subsystem release |
//! | [`standard_error_is_connected`] | `GetStdHandle(STD_ERROR_HANDLE)` | the crash hook, to decide between stderr and a box |
//! | [`show_fatal_message`] | `MessageBoxW` | the crash hook when nobody can read stderr |
//! | [`notify_associations_changed`] | `SHChangeNotify(SHCNE_ASSOCCHANGED)` | after a registration changes |
//! | [`open_uri`] | `ShellExecuteW("open")` | "Open Windows Settings", `ms-settings:` only |
//! | [`user_choice_prog_id`], [`class_default_prog_id`], [`RegistryProgIds`] | `RegGetValueW` | the association report |
//! | [`registered_application`] | `RegGetValueW` | whether to offer Settings or the `.reg` file |
//! | [`windows_build`] | `RegGetValueW` | choosing the Windows 11 or Windows 10 Settings link |
//!
//! Every registry access is a **read**. The application never writes the
//! registry (ADR-0032): the installer registers, and a portable build hands
//! the user a `.reg` file. `RegGetValueW` is the only registry function this
//! crate imports.
//!
//! ## Off Windows
//!
//! The crate compiles everywhere, so the Linux leg builds the workspace
//! unchanged. There each function returns its "no" -- `false`, `None`,
//! [`OpenUriError::Unsupported`] -- and `windows-sys` is not even a
//! dependency. The pure helpers ([`to_wide`], [`from_wide`],
//! [`is_settings_uri`], [`user_choice_subkey`], [`parse_build_number`]) are
//! outside every `cfg`, so the Linux leg tests the parts that can go wrong
//! without a registry.
//!
//! ## How the `unsafe` is kept honest
//!
//! Every `unsafe` block lives in the private `sys` module, wraps exactly one
//! call (`clippy::multiple_unsafe_ops_per_block` is denied), and carries a
//! `// SAFETY:` comment (`clippy::undocumented_unsafe_blocks` is denied).
//! Every string handed to Win32 is a `WideString`, whose only constructors
//! guarantee one terminating NUL and none inside -- the precondition every
//! `PCWSTR` parameter has, established once rather than argued at each call.
//!
//! ADR-0001's warning applies: only the Windows leg of the gate runs the
//! `sys` bodies. The Linux container can `cargo check` and `clippy` them for
//! `x86_64-pc-windows-msvc`, which proves they compile against `windows-sys`
//! and nothing more.

#![deny(unsafe_op_in_unsafe_fn)]
#![deny(clippy::undocumented_unsafe_blocks)]
#![deny(clippy::multiple_unsafe_ops_per_block)]

use std::fmt;

use bp_platform::editor::ProgIdQuery;

#[cfg(windows)]
mod sys;
mod wide;

pub use wide::{from_wide, to_wide};

/// Crate identity used by workspace smoke tests and diagnostics.
pub const CRATE_NAME: &str = "bp-platform-windows";

/// The only URI scheme [`open_uri`] will hand to the shell.
pub const SETTINGS_SCHEME: &str = "ms-settings:";

/// Attach to the console of the process that started this one, if it has
/// one.
///
/// A release build on Windows is a GUI-subsystem binary, so launched from a
/// shell it has no console and `--version` prints into nothing. Attaching to
/// the parent's console gives `println!` somewhere to go. The shell's prompt
/// has usually been redrawn by then -- a GUI program does not make `cmd`
/// wait -- so the output appears after it; that is the known cost of this
/// approach, and the alternative (a console-subsystem binary) flashes a
/// console window on every double-click.
///
/// Returns whether a console was attached. `false` when started from
/// Explorer (no parent console), when this process already has one (a debug
/// build, which is console-subsystem), and on every other platform.
#[must_use]
pub fn attach_parent_console() -> bool {
    #[cfg(windows)]
    {
        sys::attach_parent_console()
    }
    #[cfg(not(windows))]
    {
        false
    }
}

/// Whether anything is listening on standard error.
///
/// The crash hook's question: a panic message on stderr is read by a
/// developer's terminal or a test harness's pipe, but in a release build
/// started from Explorer the handle is null and the message goes nowhere --
/// which is when [`show_fatal_message`] is the only way to tell the user.
///
/// `true` on every other platform, where stderr is always the answer and
/// there is no box to fall back to.
#[must_use]
pub fn standard_error_is_connected() -> bool {
    #[cfg(windows)]
    {
        sys::standard_error_is_connected()
    }
    #[cfg(not(windows))]
    {
        true
    }
}

/// Show a modal error box with an OK button, and wait for it.
///
/// For a crash that happens with no console to report to (see
/// [`standard_error_is_connected`]). It has no owner window on purpose: the
/// window that would own it may be the thing that just failed. A NUL inside
/// either string would end it early, so each becomes a space rather than
/// losing the rest of the message -- which, in a crash report, is usually
/// the path to the report file.
///
/// Returns whether the box was shown. `false` if Windows refused (out of
/// resources, a non-interactive session) and on every other platform.
#[must_use]
pub fn show_fatal_message(title: &str, body: &str) -> bool {
    #[cfg(windows)]
    {
        let title = wide::WideString::lossy(title);
        let body = wide::WideString::lossy(body);
        sys::message_box(&title, &body)
    }
    #[cfg(not(windows))]
    {
        let _ = (title, body);
        false
    }
}

/// Tell Explorer that file associations have changed.
///
/// Without it, icons and "Open with" lists keep what Explorer cached until
/// the next sign-in. The installer sends it itself after writing the keys;
/// the shell sends it once the user says they have run a saved `.reg` file,
/// since `regedit` does not.
///
/// Returns whether the notification was sent -- `SHChangeNotify` reports no
/// failure, so on Windows this is `true` -- and `false` on every other
/// platform.
#[must_use]
pub fn notify_associations_changed() -> bool {
    #[cfg(windows)]
    {
        sys::notify_associations_changed()
    }
    #[cfg(not(windows))]
    {
        false
    }
}

/// Why [`open_uri`] did not open anything.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenUriError {
    /// Not an `ms-settings:` URI, or one with a control character or a space
    /// in it. Refused before anything reaches the shell.
    NotASettingsUri,
    /// Not Windows.
    Unsupported,
    /// `ShellExecuteW` returned this value, which is at most 32 on failure
    /// (`SE_ERR_*`, or a Win32 error code such as `ERROR_FILE_NOT_FOUND`).
    Failed { code: isize },
}

impl fmt::Display for OpenUriError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotASettingsUri => write!(
                f,
                "only Windows Settings pages (ms-settings:) are opened from here"
            ),
            Self::Unsupported => write!(f, "Windows Settings exist only on Windows"),
            Self::Failed { code } => {
                write!(f, "Windows could not open the Settings page (code {code})")
            }
        }
    }
}

impl std::error::Error for OpenUriError {}

/// Open a Windows Settings page.
///
/// **Only `ms-settings:` URIs**, and that is a safety property rather than a
/// limitation. `ShellExecuteW("open", x)` runs whatever `x` names -- a path
/// to an executable, a `file:` URL, any registered protocol -- so a
/// function that opened any string would be a function that could be talked
/// into launching a program. The product needs exactly one kind of address
/// (the Default apps page, chosen by
/// `bp_platform::editor::windows::default_apps_uri`), so exactly that kind is
/// accepted; see [`is_settings_uri`].
///
/// Call it from the UI thread. `ShellExecuteW` may hand the URI to a shell
/// extension over COM, and the windowing thread already has COM initialised
/// (winit does it for drag and drop); this crate does not initialise COM on
/// a thread it does not own.
///
/// # Errors
///
/// [`OpenUriError::NotASettingsUri`] before anything is called,
/// [`OpenUriError::Unsupported`] off Windows, [`OpenUriError::Failed`] with
/// the shell's code if Windows could not open it.
pub fn open_uri(uri: &str) -> Result<(), OpenUriError> {
    if !is_settings_uri(uri) {
        return Err(OpenUriError::NotASettingsUri);
    }
    #[cfg(windows)]
    {
        let wide = wide::WideString::new(uri).ok_or(OpenUriError::NotASettingsUri)?;
        sys::shell_open(&wide).map_err(|code| OpenUriError::Failed { code })
    }
    #[cfg(not(windows))]
    {
        Err(OpenUriError::Unsupported)
    }
}

/// Whether [`open_uri`] would pass `uri` to the shell.
///
/// It must start with [`SETTINGS_SCHEME`] (in any case, as URI schemes are
/// case-insensitive) and contain no control character and no whitespace --
/// a well-formed URI escapes both, and a raw one is either a mistake or an
/// attempt to smuggle a second argument past the shell's parser.
#[must_use]
pub fn is_settings_uri(uri: &str) -> bool {
    let has_scheme = uri
        .get(..SETTINGS_SCHEME.len())
        .is_some_and(|scheme| scheme.eq_ignore_ascii_case(SETTINGS_SCHEME));
    has_scheme && !uri.chars().any(|c| c.is_control() || c.is_whitespace())
}

/// The ProgID the user chose for `extension`, from
/// `HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\.<ext>\UserChoice`.
///
/// This is where Windows records a choice made in "Open with" or in
/// Settings, and it wins over everything else. It is hash-protected against
/// writing, and this crate only ever reads it (ADR-0012, ADR-0032).
///
/// `extension` may be given with or without its dot. `None` when there is no
/// recorded choice, when `extension` is not one (see [`user_choice_subkey`]),
/// and on every other platform.
#[must_use]
pub fn user_choice_prog_id(extension: &str) -> Option<String> {
    let subkey = user_choice_subkey(extension)?;
    read_string(Hive::CurrentUser, &subkey, Some("ProgId"))
}

/// The ProgID `HKEY_CLASSES_ROOT\.<ext>` names as the type's default.
///
/// What opens a file when the user has made no choice: the merged view of
/// the per-user and machine-wide class registrations. On a fresh Windows,
/// `.txt` has no `UserChoice` and this says `txtfile`. `None` when nothing
/// is named (an empty default counts as nothing) and on other platforms.
#[must_use]
pub fn class_default_prog_id(extension: &str) -> Option<String> {
    let extension = normalised_extension(extension)?;
    read_string(Hive::ClassesRoot, &format!(".{extension}"), None).filter(|id| !id.is_empty())
}

/// The data of `name` under `HKCU\Software\RegisteredApplications`.
///
/// What `bp_platform::editor::windows::registration_route` needs to choose
/// between "Open Windows Settings" (registered) and "Save registration
/// file…" (not). `name` is
/// `bp_platform::editor::windows::registered_application_name`. `None` when
/// the value is absent, when `name` is empty or holds a control character,
/// and on other platforms.
#[must_use]
pub fn registered_application(name: &str) -> Option<String> {
    if name.is_empty() || name.chars().any(char::is_control) {
        return None;
    }
    read_string(
        Hive::CurrentUser,
        r"Software\RegisteredApplications",
        Some(name),
    )
}

/// The Windows build number, such as `19045` or `26100`.
///
/// Read from `HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion`
/// (`CurrentBuildNumber`) rather than `GetVersionEx`, which reports whatever
/// the application manifest claims compatibility with rather than the truth.
/// The only use is choosing between the Windows 11 and Windows 10 Settings
/// links, where an unreadable build safely means "the generic page".
#[must_use]
pub fn windows_build() -> Option<u32> {
    read_string(
        Hive::LocalMachine,
        r"SOFTWARE\Microsoft\Windows NT\CurrentVersion",
        Some("CurrentBuildNumber"),
    )
    .as_deref()
    .and_then(parse_build_number)
}

/// The registry, as a [`ProgIdQuery`] `bp-platform` can ask what opens each
/// file type.
///
/// `UserChoice` first, because it wins; then the class default, because it is
/// what applies when the user has chosen nothing. Neither: nothing is set,
/// which `bp_platform::editor::state::report_from_query` reports as
/// unclaimed.
///
/// Obtainable only on Windows ([`Self::new`] is `None` elsewhere), so a Linux
/// build cannot accidentally report every type as unclaimed from a query
/// that could not look.
#[derive(Debug, Clone, Copy)]
pub struct RegistryProgIds {
    _windows_only: (),
}

impl RegistryProgIds {
    /// The registry reader, on Windows; `None` anywhere else.
    #[must_use]
    pub fn new() -> Option<Self> {
        cfg!(windows).then_some(Self { _windows_only: () })
    }
}

impl ProgIdQuery for RegistryProgIds {
    fn current_prog_id(&self, extension: &str) -> Option<String> {
        user_choice_prog_id(extension).or_else(|| class_default_prog_id(extension))
    }
}

/// The `UserChoice` key for `extension`, relative to `HKEY_CURRENT_USER`.
///
/// `None` for anything that is not plainly a file extension -- empty, or
/// holding a separator, whitespace or a control character -- because the
/// extension is spliced into a registry path, and `txt\..\..` is not an
/// extension but it is a different key.
#[must_use]
pub fn user_choice_subkey(extension: &str) -> Option<String> {
    let extension = normalised_extension(extension)?;
    Some(format!(
        r"Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\.{extension}\UserChoice"
    ))
}

/// A build number as the registry stores it: decimal digits in a string.
#[must_use]
pub fn parse_build_number(text: &str) -> Option<u32> {
    let text = text.trim();
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

/// `extension` without its dot, lowercased, if it is safe to put in a path.
fn normalised_extension(extension: &str) -> Option<String> {
    let bare = extension
        .trim()
        .strip_prefix('.')
        .unwrap_or(extension.trim());
    let acceptable = !bare.is_empty()
        && !bare
            .chars()
            .any(|c| c == '\\' || c == '/' || c.is_control() || c.is_whitespace());
    acceptable.then(|| bare.to_lowercase())
}

/// Which registry root a read starts from.
///
/// Named here rather than passed as an `HKEY`, so the choice is made in safe
/// code and `sys` maps it to the predefined handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Hive {
    CurrentUser,
    LocalMachine,
    ClassesRoot,
}

/// One `REG_SZ` value, or `None` if it is absent, of another type, not valid
/// UTF-16, or unreadable -- and always `None` off Windows. `value` `None`
/// means the key's default value.
fn read_string(hive: Hive, subkey: &str, value: Option<&str>) -> Option<String> {
    #[cfg(windows)]
    {
        let subkey = wide::WideString::new(subkey)?;
        let value = match value {
            Some(name) => Some(wide::WideString::new(name)?),
            None => None,
        };
        sys::read_string(hive, &subkey, value.as_ref())
    }
    #[cfg(not(windows))]
    {
        let _ = (hive, subkey, value);
        None
    }
}

#[cfg(test)]
mod tests;
