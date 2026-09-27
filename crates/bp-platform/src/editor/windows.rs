//! The Windows registration shape, as values, plus the guard ADR-0012 needs.
//!
//! ADR-0012 says: register as a supported handler and guide the user to the
//! operating system's own default-app selection; do not replace, patch, shim
//! or redirect `notepad.exe`. Written down that is a paragraph. Here it is a
//! function -- [`key_objection`] -- that every generated key is passed through
//! and that a test asserts nothing gets past.
//!
//! ## What is written, and what is deliberately not
//!
//! Three places, all of them under `HKEY_CURRENT_USER`:
//!
//! * `HKCU\Software\Classes\BachelorPadPlus.<ext>` -- one ProgID per file
//!   type, holding the friendly name, the icon and the open command. These are
//!   ours; nothing else has a claim on a key with our name in it.
//! * `HKCU\Software\Classes\.<ext>\OpenWithProgids` -- one *value* per
//!   extension, naming the ProgID. This is the additive registration: it puts
//!   the product in the "Open with" list without taking the extension from
//!   whoever has it.
//! * `HKCU\Software\BachelorPad+\Capabilities` and one value under
//!   `HKCU\Software\RegisteredApplications` -- what makes the product appear
//!   in Settings ▸ Default apps at all.
//!
//! And what is **not** written, each for a reason:
//!
//! * the *default* value of `HKCU\Software\Classes\.<ext>`. Setting it is the
//!   old way to seize an extension, it works, and it is precisely the
//!   "bypass" ADR-0012 refuses. The `OpenWithProgids` value beside it offers;
//!   the default value takes.
//! * `...\Explorer\FileExts\.<ext>\UserChoice`. This is where Windows records
//!   what the *user* picked. It is protected by a hash over the extension, the
//!   SID and a timestamp; writing it means forging that hash, Microsoft
//!   documents it as unsupported, and it is the single clearest example of
//!   what ADR-0012 exists to forbid. It is *read* -- by
//!   `bp-platform-windows`, for the association report -- and never written.
//! * anything under `HKEY_LOCAL_MACHINE`, and anything belonging to another
//!   application -- `txtfile`, Notepad's own ProgID, most of all.
//!
//! ## Who writes these values
//!
//! Not the application, ever (ADR-0032). The per-user installer writes
//! exactly [`registry_values`] and removes exactly the same set on uninstall;
//! a portable build offers the same values as a pair of `.reg` files --
//! [`registry_file`] to add them and [`unregister_file`] to take them away
//! again -- which the user reads and runs themselves. Given ADR-0012's
//! insistence that the user keeps control, a file the user reads and
//! double-clicks is not obviously the worse answer.
//!
//! Both files are UTF-16LE with a byte-order mark, which is the encoding
//! `regedit` itself exports. The `Windows Registry Editor Version 5.00`
//! header promises Unicode; a file carrying it in UTF-8 or the ANSI code page
//! is read through the code page, and an executable path with a character
//! outside it -- `C:\Users\Zoë\...` on a machine set to another code page --
//! is registered as a path that does not exist.
//!
//! ## Where the user finishes the job
//!
//! [`default_apps_uri`] chooses the Settings address by build number: the
//! product's own page on Windows 11, the generic Default apps page on
//! Windows 10, which has no per-application page to go to. Choosing is pure;
//! opening it is `bp-platform-windows`'s job.

use super::AppInfo;
use super::file_type::AssociationSelection;

/// The prefix of every ProgID this product owns.
///
/// A prefix rather than one flat ProgID because Windows shows the friendly
/// name *per ProgID*: one shared ProgID would label every file type "Text
/// document", including the notebooks and the encrypted documents.
pub const PROG_ID_PREFIX: &str = "BachelorPadPlus";

/// The key holding the application's declared capabilities.
pub const CAPABILITIES_KEY: &str = r"HKCU\Software\BachelorPad+\Capabilities";

/// [`CAPABILITIES_KEY`] without its hive: the data of the product's value
/// under [`REGISTERED_APPLICATIONS_KEY`], which Windows resolves against the
/// same hive the value was found in.
pub const CAPABILITIES_PATH: &str = r"Software\BachelorPad+\Capabilities";

/// The key everything the product owns outside `Software\Classes` lives under.
///
/// The unregister script deletes this whole key rather than only
/// `Capabilities`, because the register script created it and nothing else
/// writes there: the application never writes the registry (ADR-0032).
pub const OWN_SOFTWARE_KEY: &str = r"HKCU\Software\BachelorPad+";

/// The first Windows 11 build.
///
/// Windows 10 and 11 report the same major and minor version (10.0), so the
/// build number is the only thing that tells them apart. 22000 is Windows 11
/// 21H2; every Windows 10 release is below it.
pub const WINDOWS_11_FIRST_BUILD: u32 = 22000;

/// The generic Default apps page, which both Windows 10 and 11 have.
pub const DEFAULT_APPS_URI: &str = "ms-settings:defaultapps";

/// The one shared key this registration touches, and it adds a single value to
/// it under the product's own name.
///
/// Windows discovers registered applications by enumerating the values here;
/// there is no per-application alternative. Adding one named value is the
/// documented mechanism, and it neither reads nor disturbs anybody else's.
pub const REGISTERED_APPLICATIONS_KEY: &str = r"HKCU\Software\RegisteredApplications";

/// Which value of a key is meant.
///
/// A registry key's unnamed "default" value and its named values are different
/// things with different meanings, and a `String` that is empty for one of them
/// is a bug waiting for the day somebody writes a value genuinely named `""`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueName {
    /// The key's unnamed value, written `@=` in a `.reg` file.
    Default,
    /// A named value.
    Named(String),
}

/// One registry value a registration would set.
///
/// Data as a value, rather than a call that sets it, because the whole
/// question "what would this do to my machine" then has an answer a user can
/// read before anything happens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryValue {
    /// The full key path, starting `HKCU\`.
    pub key: String,
    /// Which value of it.
    pub name: ValueName,
    /// The string data. Every value this registration writes is a `REG_SZ`;
    /// nothing here needs a binary or numeric type, and keeping it to one type
    /// keeps the `.reg` rendering to one case.
    pub data: String,
}

/// Why a registry key must not be written.
///
/// Separate variants rather than a bool because they are separate promises,
/// and a refusal that cannot say which promise it is keeping is one nobody
/// can review.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyObjection {
    /// Not under `HKEY_CURRENT_USER`. A per-user registration has no business
    /// anywhere else, and a machine-wide one would need elevation the product
    /// never asks for.
    OutsideTheUserHive,
    /// Inside another application's territory: a ProgID that is not ours, or a
    /// key under `Software` belonging to somebody else.
    AnotherApplication,
    /// The extension's default value, or its `UserChoice`. Both are ways of
    /// *taking* an association rather than offering one; ADR-0012 forbids
    /// both, and `UserChoice` additionally requires forging a hash.
    SeizesTheAssociation,
    /// Names Notepad. ADR-0012's sentence, made mechanical.
    TouchesNotepad,
}

impl KeyObjection {
    /// A sentence naming the promise, for a refusal a reviewer can check.
    #[must_use]
    pub const fn describe(self) -> &'static str {
        match self {
            Self::OutsideTheUserHive => {
                "Only per-user registration is supported; this key is outside HKEY_CURRENT_USER."
            }
            Self::AnotherApplication => {
                "This key belongs to another application. BachelorPad+ registers only its own \
                 ProgIDs and adds itself to the Open With list."
            }
            Self::SeizesTheAssociation => {
                "This would take the file association rather than offer it. BachelorPad+ registers \
                 as a handler and leaves the choice to Windows (ADR-0012)."
            }
            Self::TouchesNotepad => {
                "BachelorPad+ never replaces, patches or redirects Notepad (ADR-0012)."
            }
        }
    }
}

/// Why `key` must not be written, if it must not.
///
/// The guard rather than the generator is the security boundary, because the
/// generator will grow: a preset, a file type, a new capability key. Passing
/// every value through one predicate means the next person to add a key gets a
/// failing test rather than a code review.
///
/// Comparison is case-insensitive throughout. Registry paths are, and a check
/// that is not is a check spelled around by changing one letter's case.
#[must_use]
pub fn key_objection(key: &str) -> Option<KeyObjection> {
    let lower = key.to_ascii_lowercase();

    if lower.contains("notepad") {
        return Some(KeyObjection::TouchesNotepad);
    }
    let Some(rest) = lower
        .strip_prefix(r"hkcu\")
        .or_else(|| lower.strip_prefix(r"hkey_current_user\"))
    else {
        return Some(KeyObjection::OutsideTheUserHive);
    };

    // Wherever it is found -- the real one is under `Explorer\FileExts`, and
    // a `UserChoice` under `Software\Classes` would be read by nothing but is
    // no less an attempt -- it is the user's own record of their choice.
    if rest.contains(r"\userchoice") {
        return Some(KeyObjection::SeizesTheAssociation);
    }

    if let Some(under_classes) = rest.strip_prefix(r"software\classes\") {
        let mut parts = under_classes.split('\\');
        let first = parts.next().unwrap_or_default();
        if let Some(extension) = first.strip_prefix('.') {
            if extension.is_empty() {
                return Some(KeyObjection::AnotherApplication);
            }
            // Under an extension key, exactly one sub-key is ours to write.
            // The extension key *itself* is not: its default value is the
            // association, and `UserChoice` is the user's own record of it.
            return match parts.next() {
                Some("openwithprogids") => None,
                _ => Some(KeyObjection::SeizesTheAssociation),
            };
        }
        // Not an extension, so it is a ProgID. Only ours.
        let prefix = format!("{}.", PROG_ID_PREFIX.to_ascii_lowercase());
        return if first.starts_with(&prefix) {
            None
        } else {
            Some(KeyObjection::AnotherApplication)
        };
    }

    if rest == "software\\registeredapplications" {
        return None;
    }
    if rest.starts_with("software\\bachelorpad+") {
        return None;
    }
    Some(KeyObjection::AnotherApplication)
}

/// The ProgID for one extension.
#[must_use]
pub fn prog_id(extension: &str) -> String {
    format!("{PROG_ID_PREFIX}.{}", extension.trim_start_matches('.'))
}

/// Every registry value a registration for `selection` would set.
///
/// Every key in the result satisfies [`key_objection`] -- a test asserts it
/// over every preset -- so a caller that applies these without re-checking is
/// still safe, and a caller that re-checks (as an installer should) finds
/// nothing.
#[must_use]
pub fn registry_values(app: &AppInfo, selection: &AssociationSelection) -> Vec<RegistryValue> {
    let mut values = Vec::new();
    let command = format!("\"{}\" \"%1\"", app.executable);

    for file_type in selection.file_types() {
        let id = prog_id(file_type.extension);
        let key = format!(r"HKCU\Software\Classes\{id}");
        values.push(RegistryValue {
            key: key.clone(),
            name: ValueName::Default,
            data: format!("{} ({})", file_type.description, app.display_name),
        });
        values.push(RegistryValue {
            key: key.clone(),
            name: ValueName::Named("FriendlyTypeName".to_owned()),
            data: format!("{} ({})", file_type.description, app.display_name),
        });
        values.push(RegistryValue {
            key: format!(r"{key}\DefaultIcon"),
            name: ValueName::Default,
            // Icon index 0: the application's own first icon resource. A
            // per-type icon would need an icon per type, and a missing one
            // renders as a blank page in Explorer.
            data: format!("\"{}\",0", app.executable),
        });
        values.push(RegistryValue {
            key: format!(r"{key}\shell\open\command"),
            name: ValueName::Default,
            data: command.clone(),
        });
        // The additive half: offer, never take. There is deliberately no
        // value written for the extension key's own default.
        values.push(RegistryValue {
            key: format!(
                r"HKCU\Software\Classes\{}\OpenWithProgids",
                file_type.dotted()
            ),
            name: ValueName::Named(id),
            // Windows wants this value present and empty; the value's *name*
            // is the ProgID and its data is ignored.
            data: String::new(),
        });
    }

    values.push(RegistryValue {
        key: CAPABILITIES_KEY.to_owned(),
        name: ValueName::Named("ApplicationName".to_owned()),
        data: app.display_name.clone(),
    });
    values.push(RegistryValue {
        key: CAPABILITIES_KEY.to_owned(),
        name: ValueName::Named("ApplicationDescription".to_owned()),
        data: app.description.clone(),
    });
    values.push(RegistryValue {
        key: CAPABILITIES_KEY.to_owned(),
        name: ValueName::Named("ApplicationIcon".to_owned()),
        // What Settings ▸ Default apps draws beside the name. Without it
        // Windows falls back to the icon of the first registered ProgID,
        // which is the same picture today and a different one the day a
        // file type gets its own. Spelt like `DefaultIcon` above so the two
        // cannot disagree about how the path is quoted.
        data: format!("\"{}\",0", app.executable),
    });
    for file_type in selection.file_types() {
        values.push(RegistryValue {
            key: format!(r"{CAPABILITIES_KEY}\FileAssociations"),
            name: ValueName::Named(file_type.dotted()),
            data: prog_id(file_type.extension),
        });
    }
    values.push(RegistryValue {
        key: REGISTERED_APPLICATIONS_KEY.to_owned(),
        name: ValueName::Named(registered_application_name(app).to_owned()),
        data: CAPABILITIES_PATH.to_owned(),
    });

    values
}

/// Any of `values` whose key this crate refuses to write.
///
/// Exists so the mutating step can re-check rather than trust the plan it was
/// handed. A plan is a value and values travel: through a config file, across
/// a version, into a test fixture. The check is cheap and the promise it keeps
/// is ADR-0012's.
#[must_use]
pub fn objections(values: &[RegistryValue]) -> Vec<(String, KeyObjection)> {
    values
        .iter()
        .filter_map(|value| key_objection(&value.key).map(|why| (value.key.clone(), why)))
        .collect()
}

/// The name the product is registered under in
/// [`REGISTERED_APPLICATIONS_KEY`].
///
/// The display name, because Microsoft's Default Programs documentation says
/// `Capabilities\ApplicationName` "must always match the name that is
/// registered under RegisteredApplications", and because it is the name the
/// Windows 11 deep link ([`default_apps_uri`]) looks the application up by.
/// One function so the value, the link and a check of the value cannot drift
/// apart.
#[must_use]
pub fn registered_application_name(app: &AppInfo) -> &str {
    &app.display_name
}

/// The first line of every `.reg` file this crate writes.
///
/// Version 5.00 is the Unicode format; `REGEDIT4` is the ANSI one, and would
/// mangle exactly the paths the UTF-16 encoding is there to protect.
pub const REG_HEADER: &str = "Windows Registry Editor Version 5.00";

/// `values` rendered as the text of a `.reg` file.
///
/// The reviewable form: what a dialog shows and what a test asserts on. What
/// goes to disk is [`registry_file`], the same text in the encoding `regedit`
/// reads without guessing.
///
/// CRLF line endings and the [`REG_HEADER`] are both required -- `regedit`
/// refuses a file without the header.
#[must_use]
pub fn registry_script(values: &[RegistryValue]) -> String {
    let mut out = format!("{REG_HEADER}\r\n");
    let mut current: Option<&str> = None;
    for value in values {
        if current != Some(value.key.as_str()) {
            out.push_str("\r\n[");
            out.push_str(&full_hive(&value.key));
            out.push_str("]\r\n");
            current = Some(&value.key);
        }
        match &value.name {
            ValueName::Default => out.push('@'),
            ValueName::Named(name) => {
                out.push('"');
                out.push_str(&escape_reg(name));
                out.push('"');
            }
        }
        out.push_str("=\"");
        out.push_str(&escape_reg(&value.data));
        out.push_str("\"\r\n");
    }
    out
}

/// [`registry_script`] as the bytes of a `.reg` file: UTF-16LE with a BOM.
///
/// Bytes rather than a `String` on purpose. A `String` written with
/// `fs::write` is UTF-8, and a UTF-8 file under the Version 5.00 header is
/// read through the ANSI code page -- see the module docs for what that does
/// to a path with a non-ASCII character in it.
#[must_use]
pub fn registry_file(values: &[RegistryValue]) -> Vec<u8> {
    utf16le_with_bom(&registry_script(values))
}

/// The `.reg` text that takes away exactly what [`registry_script`] adds.
///
/// Two kinds of line, and which one a value gets is the whole safety
/// argument:
///
/// * **Keys this product owns outright** -- a `BachelorPadPlus.<ext>` ProgID,
///   and [`OWN_SOFTWARE_KEY`] -- are deleted whole, `[-HKEY_CURRENT_USER\...]`.
///   Nothing but the register script created them, so nothing else is lost.
/// * **Shared keys** -- `.<ext>\OpenWithProgids`, `RegisteredApplications` --
///   are never deleted. Only the one value the register script added to each
///   is removed, `"name"=-`, and every other application's value beside it is
///   left exactly as it was.
///
/// A value whose key is neither (which [`registry_values`] never produces) is
/// treated as shared, so the worst a surprising input can do is remove one
/// value it named rather than a key it did not.
///
/// The caller should pass `values` through [`objections`] first, as for the
/// register script: the guard that keeps the register script off Notepad's
/// keys keeps this one off them too.
#[must_use]
pub fn unregister_script(values: &[RegistryValue]) -> String {
    let mut out = format!("{REG_HEADER}\r\n");

    // Shared keys first, value by value, in the order the register script
    // wrote them.
    let mut current: Option<&str> = None;
    for value in values.iter().filter(|v| owned_root(&v.key).is_none()) {
        if current != Some(value.key.as_str()) {
            out.push_str("\r\n[");
            out.push_str(&full_hive(&value.key));
            out.push_str("]\r\n");
            current = Some(&value.key);
        }
        match &value.name {
            ValueName::Default => out.push('@'),
            ValueName::Named(name) => {
                out.push('"');
                out.push_str(&escape_reg(name));
                out.push('"');
            }
        }
        out.push_str("=-\r\n");
    }

    // Then the keys that are ours, each once, root first so a reader sees
    // one line per ProgID rather than one per sub-key.
    let mut roots: Vec<String> = Vec::new();
    for value in values {
        if let Some(root) = owned_root(&value.key)
            && !roots.iter().any(|r| r.eq_ignore_ascii_case(&root))
        {
            roots.push(root);
        }
    }
    for root in roots {
        out.push_str("\r\n[-");
        out.push_str(&full_hive(&root));
        out.push_str("]\r\n");
    }
    out
}

/// [`unregister_script`] as the bytes of a `.reg` file: UTF-16LE with a BOM,
/// for the same reason as [`registry_file`].
#[must_use]
pub fn unregister_file(values: &[RegistryValue]) -> Vec<u8> {
    utf16le_with_bom(&unregister_script(values))
}

/// `text` as UTF-16LE preceded by the byte-order mark `FF FE`.
///
/// What `regedit` writes when it exports, and so the one encoding it is
/// certain to read back as it was meant.
#[must_use]
pub fn utf16le_with_bom(text: &str) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(2 + text.len() * 2);
    bytes.extend_from_slice(&[0xFF, 0xFE]);
    for unit in text.encode_utf16() {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    bytes
}

/// The key to delete whole if `key` lies inside one this product owns.
///
/// Only two shapes qualify, and both are ones [`key_objection`] already
/// accepts as ours: `HKCU\Software\Classes\BachelorPadPlus.<ext>` (and
/// anything below it), and [`OWN_SOFTWARE_KEY`] (and anything below it).
/// Everything else -- `OpenWithProgids` above all -- is shared, and its key
/// is never deleted. The installer generator ([`super::inno`]) draws the same
/// line for its uninstall flags, so the `.reg` pair and the installer cannot
/// disagree about what is ours.
pub(crate) fn owned_root(key: &str) -> Option<String> {
    let parts: Vec<&str> = key.split('\\').collect();
    let [hive, software, third, ..] = parts.as_slice() else {
        return None;
    };
    let in_user_hive =
        hive.eq_ignore_ascii_case("HKCU") || hive.eq_ignore_ascii_case("HKEY_CURRENT_USER");
    if !in_user_hive || !software.eq_ignore_ascii_case("Software") {
        return None;
    }
    if third.eq_ignore_ascii_case("BachelorPad+") {
        return Some(format!(r"{hive}\{software}\{third}"));
    }
    if third.eq_ignore_ascii_case("Classes") {
        let prog_id = parts.get(3)?;
        let prefix = format!("{PROG_ID_PREFIX}.");
        let ours = prog_id.len() > prefix.len()
            && prog_id
                .get(..prefix.len())
                .is_some_and(|head| head.eq_ignore_ascii_case(&prefix));
        return ours.then(|| format!(r"{hive}\{software}\{third}\{prog_id}"));
    }
    None
}

/// Where Windows Settings should open for the user to choose the product.
///
/// Pure over the build number, which the shell reads (`bp-platform-windows`'
/// `windows_build`) and passes in:
///
/// * Windows 11 (build [`WINDOWS_11_FIRST_BUILD`] or later):
///   `ms-settings:defaultapps?registeredAppUser=<name>`, the product's own
///   page in Default apps, where one click sets every type it registered.
///   `registeredAppUser` is the parameter Microsoft documents for an
///   application registered under `HKEY_CURRENT_USER\Software\
///   RegisteredApplications` ("Launch the Default Apps settings page",
///   Microsoft Learn), and the value is the registered name, URI-escaped --
///   so `BachelorPad+` travels as `BachelorPad%2B`, since a bare `+` in a
///   query is read as a space.
/// * Windows 10, or a build that could not be read: the generic
///   [`DEFAULT_APPS_URI`], which exists on both.
///
/// Microsoft documents the parameter from Windows 11 21H2 and 22H2 *with the
/// 2023-04 cumulative update*; an older Windows 11 that ignores it lands on
/// the generic page, which is the Windows 10 answer and still correct.
#[must_use]
pub fn default_apps_uri(build: Option<u32>, app: &AppInfo) -> String {
    match build {
        Some(build) if build >= WINDOWS_11_FIRST_BUILD => format!(
            "{DEFAULT_APPS_URI}?registeredAppUser={}",
            escape_uri_component(registered_application_name(app))
        ),
        _ => DEFAULT_APPS_URI.to_owned(),
    }
}

/// `text` percent-encoded as a URI component.
///
/// The same rule as .NET's `Uri.EscapeDataString`, which Microsoft's own
/// example of this deep link uses: the RFC 3986 unreserved characters pass,
/// every other byte of the UTF-8 encoding becomes `%XX`.
fn escape_uri_component(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            out.push(char::from(byte));
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

/// Which action the Set as Default dialog should lead with on Windows.
///
/// Decided by whether the product *is registered*, not by whether this is a
/// portable build. The two usually coincide -- the installer registers, the
/// portable zip does not -- but the state is what matters: a portable user
/// who has run the `.reg` is registered and needs Settings, and an installed
/// copy whose registration was removed needs the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistrationRoute {
    /// Registered: "Open Windows Settings", at [`default_apps_uri`]. The
    /// product already appears in Default apps; choosing it is the user's
    /// step.
    OpenSettings,
    /// Not registered: "Save registration file…", the [`registry_file`] and
    /// [`unregister_file`] pair. Opening Settings first would show a list
    /// the product is not in.
    SaveRegistrationFile,
}

/// The route for a machine whose `HKCU\Software\RegisteredApplications`
/// holds `registered` under [`registered_application_name`].
///
/// `registered` is the value's data as read (`bp-platform-windows`'
/// `registered_application`), or `None` if there is no such value. Anything
/// but [`CAPABILITIES_PATH`] counts as not registered, because a value
/// pointing somewhere else is a registration Default apps cannot follow to
/// this product's capabilities.
#[must_use]
pub fn registration_route(registered: Option<&str>) -> RegistrationRoute {
    match registered {
        Some(path)
            if path
                .trim()
                .trim_matches('\\')
                .eq_ignore_ascii_case(CAPABILITIES_PATH) =>
        {
            RegistrationRoute::OpenSettings
        }
        _ => RegistrationRoute::SaveRegistrationFile,
    }
}

/// `HKCU\` expanded, because `.reg` files require the long hive name.
fn full_hive(key: &str) -> String {
    match key.strip_prefix(r"HKCU\") {
        Some(rest) => format!(r"HKEY_CURRENT_USER\{rest}"),
        None => key.to_owned(),
    }
}

/// A string escaped for a `.reg` file's quoted value syntax.
///
/// Two characters are escaped, and both matter a great deal: an unescaped
/// backslash in a path turns `C:\temp` into `C:<tab>emp` in some readers and
/// an unescaped quote ends the value early, which is how the rest of a
/// program's path becomes a new key.
///
/// Control characters -- a line break above all -- have no escape in a
/// quoted `.reg` string, and one would end the line and let the rest of the
/// value be read as a new `[key]` or a `[-key]` deletion. None can occur in
/// what is written here (a Windows path cannot hold one, and the names are
/// the product's own), so each becomes a space rather than a refusal: the
/// guard is against injection, not against a value that was never possible.
fn escape_reg(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect::<String>()
        .replace('\\', r"\\")
        .replace('"', "\\\"")
}
