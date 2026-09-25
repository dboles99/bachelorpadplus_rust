//! The Linux registration artefacts: a `.desktop` entry and a MIME package.
//!
//! Both are plain text, both are produced here as `String`s, and neither is
//! written anywhere by this module. That separation is the point -- generating
//! the file the desktop *would* read is decision-free and testable, putting it
//! where the desktop reads it is neither, and lives in [`super::install`].
//!
//! ## Escaping, which is the whole difficulty
//!
//! A `.desktop` file is a line-oriented `key=value` format with no quoting at
//! the line level, so a value containing a newline does not produce a broken
//! file -- it produces a *valid* file with an extra key in it. Every string
//! that reaches an artefact here comes from an [`AppInfo`](super::AppInfo)
//! that the shell filled in, so "the application's display name" is
//! attacker-adjacent in exactly the way a filename is, and the escaping is
//! written on that assumption rather than on the assumption that the name is
//! "BachelorPad+".
//!
//! The `Exec` key escapes **twice**, and getting that wrong is the classic
//! defect in this format: its value is first quoted by the Exec argument rules
//! (freedesktop's own reserved-character list) and the result is then a
//! desktop-entry string, where a backslash means an escape. A single Windows
//! -style path with a backslash in it therefore needs four backslashes in the
//! file to survive the round trip, and a single-escaped one launches the wrong
//! program.

use super::AppInfo;
use super::file_type::{AssociationPreset, AssociationSelection};

/// The preset a distribution package claims in its `.desktop` entry and MIME
/// package, and the one File ▸ Set as Default Editor registers for.
///
/// One constant for both, because they have to agree: when a package has
/// installed the system `.desktop` entry, Set as Default writes no per-user
/// copy (see [`super::linux_plan`]), so the system entry's `MimeType=` line is
/// the whole claim. Notepad Replacement is the smallest useful set -- plain
/// text, logs and the product's own `.bpadx` -- and the one that takes
/// nothing from anybody by surprise: a package claiming `.csv` or `.html`
/// would become the fallback handler for them on every desktop where nothing
/// else is set, which is choosing for the user by another route.
pub const PACKAGED_PRESET: AssociationPreset = AssociationPreset::NotepadReplacement;

/// The first line of every file a per-user registration writes.
///
/// How [`super::uninstall`] tells a file this product wrote from one that
/// merely has its name. A menu editor (alacarte, KDE's) makes a per-user copy
/// of a `.desktop` entry under the *same* id when the user customises it, and
/// Remove Registration deleting the user's own customisation would be taking
/// something that was never ours. Plain text rather than a key, so it changes
/// nothing a desktop reads; and free of `<`, `>`, `&` and `--`, so it is the
/// same text in a desktop-entry comment and an XML comment (a test holds it
/// to that).
pub const PER_USER_MARKER: &str =
    "Written by BachelorPad+ Set as Default Editor. Remove Registration deletes this file.";

/// One entry of the AppStream `<releases>` list, newest first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Release {
    /// The version, exactly as `Cargo.toml` spells it.
    pub version: &'static str,
    /// ISO 8601 date of the release.
    pub date: &'static str,
}

/// Every release, newest first.
///
/// A release adds a line here, and a test fails until it does: the newest
/// entry must be the workspace version, because a software centre shows the
/// newest `<release>` as "what's new" and a stale one tells users they are
/// running something they are not.
pub const RELEASES: &[Release] = &[Release {
    version: "0.1.0",
    date: "2026-09-25",
}];

/// The freedesktop menu categories, for the `.desktop` entry's `Categories=`
/// and AppStream's `<categories>` alike.
///
/// `TextEditor` alone is not enough: the menu specification makes it an
/// additional category, valid only beside a main one, and `Utility` is the
/// main category text editors use.
pub const CATEGORIES: &[&str] = &["Utility", "TextEditor"];

/// The project's home page, for AppStream.
pub const HOMEPAGE: &str = "https://github.com/dboles99/bachelorpadplus_rust";

/// A file shipped in `packaging/linux/`, generated here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackagedAsset {
    /// The file name under `packaging/linux/`.
    pub file_name: String,
    /// The whole file.
    pub contents: String,
}

/// Characters the Exec key treats as reserved, per the Desktop Entry
/// Specification.
///
/// An argument containing any of them must be quoted. Listed rather than
/// approximated with "is this alphanumeric", because the correct set is short,
/// fixed, and published, and an approximation is a quoting rule that differs
/// from the one the desktop environment parses with.
const EXEC_RESERVED: &[char] = &[
    ' ', '\t', '\n', '"', '\'', '\\', '>', '<', '~', '|', '&', ';', '$', '*', '?', '#', '(', ')',
    '`',
];

/// The file name a `.desktop` entry for `app` must have.
///
/// The name is not cosmetic: `mimeapps.list` refers to an application *by this
/// file name*, so it is the application's identity on Linux and changing it
/// orphans every association the user already made.
#[must_use]
pub fn desktop_file_name(app: &AppInfo) -> String {
    format!("{}.desktop", app.app_id)
}

/// The file name of the MIME package for `app`.
#[must_use]
pub fn mime_package_file_name(app: &AppInfo) -> String {
    format!("{}.xml", app.app_id)
}

/// The file name of the AppStream metadata for `app`.
#[must_use]
pub fn metainfo_file_name(app: &AppInfo) -> String {
    format!("{}.metainfo.xml", app.app_id)
}

/// A `.desktop` entry claiming `selection`.
///
/// Deterministic: the same inputs give byte-identical output, because the
/// selection iterates in table order and the MIME list is deduplicated in a
/// stable one. An artefact that reorders itself between runs shows up as a
/// change in every backup and every configuration-management diff, and
/// eventually gets ignored.
///
/// An empty selection omits the `MimeType` key entirely rather than emitting
/// an empty one -- the product still belongs in the application menu, it just
/// claims nothing.
#[must_use]
pub fn desktop_entry(app: &AppInfo, selection: &AssociationSelection) -> String {
    let mut out = String::from("[Desktop Entry]\n");
    out.push_str("Type=Application\n");
    out.push_str("Version=1.0\n");
    push_key(&mut out, "Name", &app.display_name);
    out.push_str("GenericName=Text Editor\n");
    push_key(&mut out, "Comment", &app.description);
    // `%F` rather than `%U`: this is an editor for files on a disk, and
    // accepting a URL it cannot open would put a broken entry in "Open With"
    // for every remote location the desktop knows about.
    push_key(
        &mut out,
        "Exec",
        &format!("{} %F", quote_exec_argument(&app.executable)),
    );
    push_key(&mut out, "TryExec", &app.executable);
    push_key(&mut out, "Icon", &app.icon);
    out.push_str("Terminal=false\n");
    out.push_str("StartupNotify=true\n");
    out.push_str("Categories=");
    for category in CATEGORIES {
        out.push_str(&escape_list_item(category));
        out.push(';');
    }
    out.push('\n');
    out.push_str("Keywords=text;editor;notes;markdown;log;\n");
    let mimes = selection.mime_types();
    if !mimes.is_empty() {
        out.push_str("MimeType=");
        for mime in mimes {
            out.push_str(&escape_list_item(mime));
            out.push(';');
        }
        out.push('\n');
    }
    out
}

/// A `shared-mime-info` package defining the types nobody else defines, or
/// `None` when the selection contains none.
///
/// Only the product's own types get a glob here. Shipping one for `.json`
/// would be a disagreement with the distribution's own MIME database about
/// what `.json` is, and the distribution wins that argument on every machine
/// where it is installed second.
#[must_use]
pub fn mime_package(selection: &AssociationSelection) -> Option<String> {
    let own: Vec<_> = selection
        .file_types()
        .filter(|t| t.is_own_mime_type())
        .collect();
    if own.is_empty() {
        return None;
    }
    let mut out = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str("<mime-info xmlns=\"http://www.freedesktop.org/standards/shared-mime-info\">\n");
    // One `mime-type` element per distinct type, with every extension that
    // maps to it as a glob -- three PowerShell extensions are one type, and
    // repeating the element would define it three times.
    let mut done: Vec<&str> = Vec::new();
    for file_type in &own {
        if done.contains(&file_type.mime) {
            continue;
        }
        done.push(file_type.mime);
        out.push_str(&format!(
            "  <mime-type type=\"{}\">\n",
            escape_xml(file_type.mime)
        ));
        out.push_str(&format!(
            "    <comment>{}</comment>\n",
            escape_xml(file_type.description)
        ));
        for sibling in own.iter().filter(|t| t.mime == file_type.mime) {
            out.push_str(&format!(
                "    <glob pattern=\"{}\"/>\n",
                escape_xml(&sibling.glob())
            ));
        }
        out.push_str("  </mime-type>\n");
    }
    out.push_str("</mime-info>\n");
    Some(out)
}

/// [`desktop_entry`], marked as written by a per-user registration.
///
/// The marker is a comment, which the Desktop Entry Specification allows
/// before the first group, so the file a desktop reads is exactly
/// [`desktop_entry`]'s. See [`PER_USER_MARKER`] for why it is there.
#[must_use]
pub fn per_user_desktop_entry(app: &AppInfo, selection: &AssociationSelection) -> String {
    format!("# {PER_USER_MARKER}\n{}", desktop_entry(app, selection))
}

/// [`mime_package`], marked as written by a per-user registration.
///
/// The comment goes after the XML declaration, which must be the first thing
/// in the file.
#[must_use]
pub fn per_user_mime_package(selection: &AssociationSelection) -> Option<String> {
    let package = mime_package(selection)?;
    let (declaration, rest) = package.split_once('\n')?;
    Some(format!("{declaration}\n<!-- {PER_USER_MARKER} -->\n{rest}"))
}

/// Whether `contents` is a file a per-user registration wrote.
///
/// Only the first two lines are looked at -- that is where both writers put
/// the marker -- so a file that merely *mentions* the sentence somewhere,
/// say in a `Comment`, does not qualify.
#[must_use]
pub fn is_per_user_registration(contents: &str) -> bool {
    contents
        .lines()
        .take(2)
        .any(|line| line.contains(PER_USER_MARKER))
}

/// AppStream metadata for `app`, providing `selection`'s MIME types.
///
/// Generated rather than hand-written for the same reason as the other two
/// files: `<provides><mediatype>` has to list exactly what the `.desktop`
/// entry's `MimeType=` claims, and two hand-kept lists of MIME types drift the
/// first time somebody adds a file type. The licences are the metadata's own
/// (CC0, as AppStream asks, so catalogues may copy it) and the project's.
#[must_use]
pub fn metainfo(app: &AppInfo, selection: &AssociationSelection) -> String {
    let mut out = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str("<component type=\"desktop-application\">\n");
    push_element(&mut out, "id", &app.app_id);
    push_element(&mut out, "metadata_license", "CC0-1.0");
    push_element(&mut out, "project_license", "MIT OR Apache-2.0");
    push_element(&mut out, "name", &app.display_name);
    push_element(&mut out, "summary", &app.description);
    out.push_str("  <description>\n");
    for paragraph in [
        "BachelorPad+ is a plain-text editor that starts as fast as Notepad and stays out \
         of the way: open a file, change it, save it.",
        "It reads plain text, logs, Markdown, JSON, YAML, TOML, CSV and XML, keeps a \
         recovery journal so unsaved work survives a crash, and can save a document \
         encrypted with a passphrase in its own .bpadx format.",
    ] {
        out.push_str(&format!("    <p>{}</p>\n", escape_xml(paragraph)));
    }
    out.push_str("  </description>\n");
    out.push_str("  <developer id=\"io.github.dboles99\">\n");
    out.push_str("    <name>BachelorPad+ contributors</name>\n");
    out.push_str("  </developer>\n");
    out.push_str(&format!(
        "  <url type=\"homepage\">{}</url>\n",
        escape_xml(HOMEPAGE)
    ));
    out.push_str(&format!(
        "  <launchable type=\"desktop-id\">{}</launchable>\n",
        escape_xml(&desktop_file_name(app))
    ));
    out.push_str(&format!(
        "  <icon type=\"stock\">{}</icon>\n",
        escape_xml(&app.icon)
    ));
    // Repeated from the `.desktop` entry because a catalogue validating the
    // metainfo on its own -- `appstreamcli validate`, Flathub -- does not
    // merge the two, and reports an application with no category.
    out.push_str("  <categories>\n");
    for category in CATEGORIES {
        push_indented_element(&mut out, 4, "category", category);
    }
    out.push_str("  </categories>\n");
    // No attributes under the rating: the product has no violence, no
    // in-app purchases, no social features and no advertising, and an
    // `oars-1.1` rating with nothing in it says exactly that.
    out.push_str("  <content_rating type=\"oars-1.1\"/>\n");
    let mimes = selection.mime_types();
    if !mimes.is_empty() {
        out.push_str("  <provides>\n");
        for mime in mimes {
            push_indented_element(&mut out, 4, "mediatype", mime);
        }
        out.push_str("  </provides>\n");
    }
    out.push_str("  <releases>\n");
    for release in RELEASES {
        out.push_str(&format!(
            "    <release version=\"{}\" date=\"{}\"/>\n",
            escape_xml(release.version),
            escape_xml(release.date)
        ));
    }
    out.push_str("  </releases>\n");
    out.push_str("</component>\n");
    out
}

/// Every file `packaging/linux/` ships, generated.
///
/// The single source for the package's static assets: a test asserts the
/// shipped files equal this byte for byte, and
/// `cargo run -p bp-platform --example regenerate_linux_assets` rewrites them
/// from it. Built from [`AppInfo::packaged`](super::AppInfo::packaged) and
/// [`PACKAGED_PRESET`], and without the per-user marker -- a package owns
/// its files outright and removes them itself.
#[must_use]
pub fn packaged_assets() -> Vec<PackagedAsset> {
    let app = AppInfo::packaged();
    let selection = AssociationSelection::preset(PACKAGED_PRESET);
    let mut assets = vec![PackagedAsset {
        file_name: desktop_file_name(&app),
        contents: desktop_entry(&app, &selection),
    }];
    if let Some(package) = mime_package(&selection) {
        assets.push(PackagedAsset {
            file_name: mime_package_file_name(&app),
            contents: package,
        });
    }
    assets.push(PackagedAsset {
        file_name: metainfo_file_name(&app),
        contents: metainfo(&app, &selection),
    });
    assets
}

/// Append `  <name>text</name>` with the text escaped.
fn push_element(out: &mut String, name: &str, text: &str) {
    push_indented_element(out, 2, name, text);
}

/// Append `<name>text</name>`, indented by `indent` spaces, with the text
/// escaped.
fn push_indented_element(out: &mut String, indent: usize, name: &str, text: &str) {
    out.push_str(&format!(
        "{:indent$}<{name}>{}</{name}>\n",
        "",
        escape_xml(text)
    ));
}

/// Append `key=value` with the value escaped as a desktop-entry string.
fn push_key(out: &mut String, key: &str, value: &str) {
    out.push_str(key);
    out.push('=');
    out.push_str(&escape_value(value));
    out.push('\n');
}

/// A string escaped for a desktop-entry value.
///
/// The escape that matters is the newline. A `.desktop` file has no line
/// continuation and no quoting, so an un-escaped newline in a `Name` does not
/// corrupt the file -- it silently adds whatever follows as another key, which
/// is how a display name becomes an `Exec` line. The backslash escape has to
/// come first, or escaping a newline would itself be re-escaped.
#[must_use]
pub fn escape_value(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '\\' => out.push_str(r"\\"),
            '\n' => out.push_str(r"\n"),
            '\t' => out.push_str(r"\t"),
            '\r' => out.push_str(r"\r"),
            // Any other control character has no escape in this format and no
            // legitimate use in a name; dropping it is the only option that
            // cannot produce a file the parser reads differently.
            c if (c as u32) < 0x20 => {}
            c => out.push(c),
        }
    }
    out
}

/// One item of a semicolon-separated desktop-entry list.
///
/// The separator is escapable and must be escaped, or a MIME type containing a
/// semicolon -- which nothing sane has, and an [`AppInfo`] field certainly
/// could -- would split into two claims.
#[must_use]
pub fn escape_list_item(item: &str) -> String {
    escape_value(item).replace(';', r"\;")
}

/// One argument of an `Exec` value, quoted if the specification requires it.
///
/// Quoting only when required, because `Exec=/usr/bin/bachelorpad %F` is what
/// every other entry on the system looks like and an always-quoted value, while
/// legal, makes a hand-inspected file look wrong. The escaping *inside* the
/// quotes is the specification's own short list: `"`, backtick, `$` and `\`.
///
/// Note this returns the Exec-level form. It is escaped a second time by
/// [`escape_value`] when it becomes a line, which is correct and is the step
/// most implementations miss.
#[must_use]
pub fn quote_exec_argument(argument: &str) -> String {
    if !argument.is_empty() && !argument.contains(EXEC_RESERVED) {
        return argument.to_owned();
    }
    let mut out = String::with_capacity(argument.len() + 2);
    out.push('"');
    for c in argument.chars() {
        if matches!(c, '"' | '`' | '$' | '\\') {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('"');
    out
}

/// Text escaped for an XML attribute or element body.
///
/// Attributes here are always double-quoted, so the apostrophe needs no
/// entity; the other four do. `&` first, or the ampersands introduced by the
/// later replacements would be escaped again.
#[must_use]
pub fn escape_xml(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
