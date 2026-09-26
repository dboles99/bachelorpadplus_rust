//! The generated parts of the Windows installer, `packaging/windows/bachelorpad.iss`.
//!
//! ADR-0032 gives the per-user installer the job of writing the registration
//! and taking it away again. That is only safe if the installer writes
//! *exactly* [`windows::registry_values`] -- the same values the `.reg` pair
//! carries, the same values [`windows::key_objection`] has passed -- so the
//! installer's `[Registry]` section is not written by hand. It is rendered
//! here, spliced into the `.iss` between marker lines by
//! `cargo run -p bp-platform --example regenerate_windows_installer`, and a
//! test fails if the shipped block differs from this module's output by a
//! byte.
//!
//! The same applies to the uninstaller's list of the user's directories,
//! which comes from [`crate::dirs`] rather than from a second spelling of
//! `%LOCALAPPDATA%\bachelorpad` in Pascal.
//!
//! ## What uninstall removes, and how the flags say so
//!
//! Inno Setup records a removal per `[Registry]` entry, and the flag chosen
//! is the same line [`windows::unregister_script`] draws:
//!
//! * a key the product owns outright -- a `BachelorPadPlus.<ext>` ProgID, or
//!   `Software\BachelorPad+` -- gets `uninsdeletekey`, and goes whole;
//! * a value added to a shared key -- `.<ext>\OpenWithProgids`,
//!   `RegisteredApplications` -- gets `uninsdeletevalue`, so only that one
//!   value goes and every other application's value beside it stays.
//!
//! `Software\BachelorPad+` holds no value of its own, only the
//! `Capabilities` key below it, so it gets a key-only entry to carry its
//! `uninsdeletekey`; without it uninstall would leave an empty key behind.
//!
//! ## Escaping
//!
//! An Inno Setup parameter is a double-quoted string in which `{` starts a
//! constant (`{app}`) and a literal one is written `{{`, and a literal `"`
//! is written `""`. Every string is escaped on that basis, with one
//! exception the caller names: the installed executable, `{app}\bachelorpad.exe`,
//! whose `{app}` is meant as the constant. Everything else -- a description
//! that grows a brace one day, a quote in a name -- is literal text.
//!
//! The `.iss` is ASCII, so ISCC reads it the same whatever code page the
//! build machine has; the generator refuses anything else rather than rely
//! on a byte-order mark nobody remembers to keep.

use std::fmt;

use super::AppInfo;
use super::desktop::PACKAGED_PRESET;
use super::file_type::AssociationSelection;
use super::windows::{self, KeyObjection, RegistryValue, ValueName};
use crate::dirs::{self, DirKind, EnvSnapshot};
use crate::{APP_DIR, Platform};

/// The installed executable, as Inno Setup names it.
///
/// `{app}` is the directory the user chose, resolved at install time, which
/// is why the registration cannot be generated with a real path here.
pub const INSTALLED_EXECUTABLE: &str = r"{app}\bachelorpad.exe";

/// The installer script, relative to the workspace root.
pub const INSTALLER_SCRIPT: &str = "packaging/windows/bachelorpad.iss";

/// The command that rewrites every generated block, for marker lines and
/// error messages.
pub const REGENERATE: &str = "cargo run -p bp-platform --example regenerate_windows_installer";

/// What uninstall does with one `[Registry]` entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UninstallAction {
    /// `uninsdeletekey`: the whole key goes. Only for keys the product owns.
    DeleteKey,
    /// `uninsdeletevalue`: only the value goes, and the key it sits in --
    /// shared with other applications -- stays.
    DeleteValue,
}

impl UninstallAction {
    /// The Inno Setup flag.
    #[must_use]
    pub const fn flag(self) -> &'static str {
        match self {
            Self::DeleteKey => "uninsdeletekey",
            Self::DeleteValue => "uninsdeletevalue",
        }
    }
}

/// What an entry writes: a string value, or only the key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntryValue {
    /// `ValueType: none` -- the key is created, and exists to carry its
    /// uninstall flag.
    KeyOnly,
    /// `ValueType: string`, as every registration value is.
    String {
        /// Which value of the key.
        name: ValueName,
        /// Its data, unescaped.
        data: String,
    },
}

/// One line of the installer's `[Registry]` section, before rendering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryEntry {
    /// The key below `HKEY_CURRENT_USER`, without the hive.
    pub subkey: String,
    /// What is written there.
    pub value: EntryValue,
    /// What uninstall does with it.
    pub uninstall: UninstallAction,
}

/// Why a generated block could not be produced or spliced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GeneratorError {
    /// A value's key is one [`windows::key_objection`] refuses. The generator
    /// re-checks rather than trusting its input, for the reason
    /// [`windows::objections`] gives.
    Objection {
        /// The refused key.
        key: String,
        /// Which promise it would break.
        why: KeyObjection,
    },
    /// The rendered text is not ASCII; see the module docs.
    NotAscii(String),
    /// `bp-platform` could not resolve a Windows directory from the
    /// placeholder environment. Would mean [`crate::dirs`] changed shape.
    UnresolvedDirectory(DirKind),
    /// A marker line is missing from the script.
    MissingMarker(String),
    /// A marker line appears more than once, so which block is meant is
    /// ambiguous.
    DuplicateMarker(String),
    /// The end marker comes before the begin marker.
    MarkersOutOfOrder(String),
}

impl fmt::Display for GeneratorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Objection { key, why } => write!(f, "{key}: {}", why.describe()),
            Self::NotAscii(text) => write!(
                f,
                "the installer script is kept to ASCII, and this is not: {text:?}"
            ),
            Self::UnresolvedDirectory(kind) => write!(
                f,
                "bp-platform did not resolve the Windows {} directory",
                kind.token()
            ),
            Self::MissingMarker(marker) => write!(f, "marker line not found: {marker}"),
            Self::DuplicateMarker(marker) => {
                write!(f, "marker line appears more than once: {marker}")
            }
            Self::MarkersOutOfOrder(name) => {
                write!(
                    f,
                    "the end marker of block {name} comes before its begin marker"
                )
            }
        }
    }
}

impl std::error::Error for GeneratorError {}

/// The installer's `[Registry]` entries for `values`.
///
/// Every key is passed through [`windows::key_objection`] first; one refusal
/// fails the whole section, because an installer that writes most of a
/// registration is worse than one that does not build.
pub fn registry_entries(values: &[RegistryValue]) -> Result<Vec<RegistryEntry>, GeneratorError> {
    let mut entries = Vec::new();
    let mut owned_roots: Vec<String> = Vec::new();
    for value in values {
        if let Some(why) = windows::key_objection(&value.key) {
            return Err(GeneratorError::Objection {
                key: value.key.clone(),
                why,
            });
        }
        let owned = windows::owned_root(&value.key);
        if let Some(root) = &owned
            && !owned_roots.iter().any(|r| r.eq_ignore_ascii_case(root))
        {
            owned_roots.push(root.clone());
        }
        entries.push(RegistryEntry {
            subkey: without_hive(&value.key),
            value: EntryValue::String {
                name: value.name.clone(),
                data: value.data.clone(),
            },
            uninstall: if owned.is_some() {
                UninstallAction::DeleteKey
            } else {
                UninstallAction::DeleteValue
            },
        });
    }

    // A root that holds no value of its own would otherwise survive
    // uninstall as an empty key. Its entry goes first, so it reads as the
    // parent of what follows.
    let mut roots_without_values: Vec<RegistryEntry> = owned_roots
        .iter()
        .filter(|root| !values.iter().any(|v| v.key.eq_ignore_ascii_case(root)))
        .map(|root| RegistryEntry {
            subkey: without_hive(root),
            value: EntryValue::KeyOnly,
            uninstall: UninstallAction::DeleteKey,
        })
        .collect();
    roots_without_values.extend(entries);
    Ok(roots_without_values)
}

/// `entries` as the lines of a `[Registry]` section.
///
/// `executable` is the one piece of text passed through unescaped wherever it
/// appears in a value's data, so that `{app}` stays a constant; see the
/// module docs. Each line ends with `\n`.
pub fn render_registry_entries(
    entries: &[RegistryEntry],
    executable: &str,
) -> Result<String, GeneratorError> {
    let mut out = String::new();
    for entry in entries {
        out.push_str("Root: HKCU; Subkey: ");
        out.push_str(&inno_string(&entry.subkey));
        match &entry.value {
            EntryValue::KeyOnly => out.push_str("; ValueType: none"),
            EntryValue::String { name, data } => {
                out.push_str("; ValueType: string; ValueName: ");
                out.push_str(&inno_string(match name {
                    ValueName::Default => "",
                    ValueName::Named(name) => name,
                }));
                out.push_str("; ValueData: ");
                out.push_str(&inno_data(data, executable));
            }
        }
        out.push_str("; Flags: ");
        out.push_str(entry.uninstall.flag());
        out.push('\n');
    }
    ascii(out)
}

/// The `[Registry]` block the shipped installer carries.
///
/// The registration File ▸ Set as Default Editor would produce, for the
/// same preset ([`PACKAGED_PRESET`]), with the executable at
/// [`INSTALLED_EXECUTABLE`]. The same preset matters: the app's own Remove
/// Registration and the association report then describe exactly what the
/// installer wrote.
pub fn packaged_registry_block() -> Result<String, GeneratorError> {
    let app = AppInfo::bachelorpad(INSTALLED_EXECUTABLE);
    let selection = AssociationSelection::preset(PACKAGED_PRESET);
    let entries = registry_entries(&windows::registry_values(&app, &selection))?;
    render_registry_entries(&entries, INSTALLED_EXECUTABLE)
}

/// One of the user's directories, as the installer's `[Code]` names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserDirectory {
    /// Which directory.
    pub kind: DirKind,
    /// The path with an Inno Setup constant for its root --
    /// `{localappdata}\bachelorpad\data`, `{userappdata}\bachelorpad`.
    pub path: String,
}

/// Where Inno Setup's `{userappdata}` and `{localappdata}` stand in for
/// `%APPDATA%` and `%LOCALAPPDATA%`. Absolute and distinct, so
/// [`crate::dirs`] resolves through them and the root can be recognised
/// again afterwards.
const ROAMING_PLACEHOLDER: &str = r"C:\ROAMING";
const LOCAL_PLACEHOLDER: &str = r"C:\LOCAL";

/// Every directory the product keeps files in on Windows, rooted at the
/// Inno Setup constant for its profile folder.
///
/// Resolved by [`dirs::directory`] itself, with the two profile roots
/// replaced by placeholders and the placeholders by constants, so the
/// uninstaller asks about the directories the product really writes and not
/// about a copy of them that could drift.
pub fn user_directories() -> Result<Vec<UserDirectory>, GeneratorError> {
    let env = EnvSnapshot {
        appdata: Some(ROAMING_PLACEHOLDER.to_owned()),
        local_appdata: Some(LOCAL_PLACEHOLDER.to_owned()),
        ..EnvSnapshot::default()
    };
    DirKind::ALL
        .iter()
        .map(|&kind| {
            let unresolved = GeneratorError::UnresolvedDirectory(kind);
            let root = dirs::root(Platform::Windows, kind, &env).ok_or(unresolved.clone())?;
            let path = dirs::directory(Platform::Windows, kind, &env).ok_or(unresolved.clone())?;
            let (root, path) = (root.to_string_lossy(), path.to_string_lossy());
            let constant = match root.as_ref() {
                ROAMING_PLACEHOLDER => "{userappdata}",
                LOCAL_PLACEHOLDER => "{localappdata}",
                _ => return Err(unresolved),
            };
            let rest = path.strip_prefix(root.as_ref()).ok_or(unresolved)?;
            Ok(UserDirectory {
                kind,
                path: format!("{constant}{}", rest.replace('{', "{{")),
            })
        })
        .collect()
}

/// The `[Code]` constants naming the user's directories.
///
/// One Pascal constant per [`DirKind`] (`DataDirectory`, ...), plus
/// `ProductLocalDirectory`, the `bachelorpad` folder in `%LOCALAPPDATA%`
/// that holds the three local ones, so the uninstaller can remove it once
/// they are gone and it is empty. Each is a string for `ExpandConstant`.
pub fn user_directories_block() -> Result<String, GeneratorError> {
    let directories = user_directories()?;
    let product_local = format!(r"{{localappdata}}\{}", APP_DIR.replace('{', "{{"));
    // The uninstaller removes this folder once it is empty. That is only
    // right while every local directory is strictly inside it; if `dirs`
    // ever put one straight into `%LOCALAPPDATA%`, the folder would be
    // somebody else's too, so refuse to generate rather than ask about it.
    let below = format!(r"{product_local}\");
    if let Some(stray) = directories
        .iter()
        .find(|d| d.path.starts_with("{localappdata}") && !d.path.starts_with(&below))
    {
        return Err(GeneratorError::UnresolvedDirectory(stray.kind));
    }
    let mut out = String::from("const\n");
    for directory in &directories {
        out.push_str(&format!(
            "  {}Directory = {};\n",
            pascal_name(directory.kind),
            pascal_string(&directory.path)
        ));
    }
    out.push_str(&format!(
        "  ProductLocalDirectory = {};\n",
        pascal_string(&product_local)
    ));
    ascii(out)
}

/// One generated block of the installer script.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedBlock {
    /// A short name, which is also in the markers.
    pub name: &'static str,
    /// The line that opens the block, without its line ending.
    pub begin: String,
    /// The line that closes it.
    pub end: String,
    /// Everything between the two lines, each line ending with `\n`.
    pub contents: String,
}

/// Every generated block the shipped installer script carries.
pub fn packaged_blocks() -> Result<Vec<GeneratedBlock>, GeneratorError> {
    Ok(vec![
        // `[Registry]` is Inno Setup's own syntax, where `;` starts a comment.
        block("registry", ";", packaged_registry_block()?),
        // `[Code]` is Pascal, where it does not.
        block("user-directories", "//", user_directories_block()?),
    ])
}

fn block(name: &'static str, comment: &str, contents: String) -> GeneratedBlock {
    GeneratedBlock {
        name,
        begin: format!("{comment} BEGIN GENERATED {name}: {REGENERATE}"),
        end: format!("{comment} END GENERATED {name}"),
        contents,
    }
}

/// What `script` currently holds between `block`'s markers.
pub fn generated_text<'a>(
    script: &'a str,
    block: &GeneratedBlock,
) -> Result<&'a str, GeneratorError> {
    let (start, end) = block_span(script, block)?;
    Ok(script.get(start..end).unwrap_or_default())
}

/// `script` with the text between `block`'s markers replaced by its
/// contents. Everything outside the markers is left byte for byte.
pub fn splice(script: &str, block: &GeneratedBlock) -> Result<String, GeneratorError> {
    let (start, end) = block_span(script, block)?;
    let mut out = String::with_capacity(script.len() + block.contents.len());
    out.push_str(script.get(..start).unwrap_or_default());
    out.push_str(&block.contents);
    out.push_str(script.get(end..).unwrap_or_default());
    Ok(out)
}

/// The byte range between the end of the begin line and the start of the
/// end line.
///
/// Markers are matched as whole lines, so a marker quoted inside a comment
/// elsewhere in the script cannot be mistaken for the real one.
fn block_span(script: &str, block: &GeneratedBlock) -> Result<(usize, usize), GeneratorError> {
    let begin = unique_line(script, &block.begin)?;
    let end = unique_line(script, &block.end)?;
    let start = begin.1;
    if end.0 < start {
        return Err(GeneratorError::MarkersOutOfOrder(block.name.to_owned()));
    }
    Ok((start, end.0))
}

/// The byte offsets of the line equal to `marker`: where it starts, and
/// where the next line starts.
fn unique_line(script: &str, marker: &str) -> Result<(usize, usize), GeneratorError> {
    let mut found = None;
    let mut offset = 0;
    for line in script.split_inclusive('\n') {
        let next = offset + line.len();
        if line.trim_end_matches(['\n', '\r']) == marker {
            if found.is_some() {
                return Err(GeneratorError::DuplicateMarker(marker.to_owned()));
            }
            found = Some((offset, next));
        }
        offset = next;
    }
    found.ok_or_else(|| GeneratorError::MissingMarker(marker.to_owned()))
}

/// `key` without its `HKCU\` or `HKEY_CURRENT_USER\` prefix; the entry names
/// the hive in `Root:`.
fn without_hive(key: &str) -> String {
    key.strip_prefix(r"HKCU\")
        .or_else(|| key.strip_prefix(r"HKEY_CURRENT_USER\"))
        .unwrap_or(key)
        .to_owned()
}

/// `text` as a quoted Inno Setup parameter, every character literal.
fn inno_string(text: &str) -> String {
    format!("\"{}\"", escape_inno(text))
}

/// `data` as a quoted Inno Setup parameter, with every occurrence of
/// `executable` left as written so its constant is expanded.
fn inno_data(data: &str, executable: &str) -> String {
    if executable.is_empty() {
        return inno_string(data);
    }
    let pieces: Vec<String> = data.split(executable).map(escape_inno).collect();
    format!("\"{}\"", pieces.join(executable))
}

/// The inside of a quoted Inno Setup parameter.
///
/// A control character has no escape there, and a line break would end the
/// entry and let the rest be read as another one -- the same injection
/// `escape_reg` in [`windows`] guards against, and answered the same way.
fn escape_inno(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect::<String>()
        .replace('{', "{{")
        .replace('"', "\"\"")
}

/// `text` as a Pascal string literal: quotes doubled, controls made spaces.
fn pascal_string(text: &str) -> String {
    let cleaned: String = text
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    format!("'{}'", cleaned.replace('\'', "''"))
}

/// The Pascal constant-name stem for `kind`.
fn pascal_name(kind: DirKind) -> &'static str {
    match kind {
        DirKind::Config => "Config",
        DirKind::Data => "Data",
        DirKind::Cache => "Cache",
        DirKind::State => "State",
    }
}

fn ascii(text: String) -> Result<String, GeneratorError> {
    if text.is_ascii() {
        Ok(text)
    } else {
        Err(GeneratorError::NotAscii(text))
    }
}

#[cfg(test)]
mod tests;
