//! Reporting which application currently opens each file type.
//!
//! The half of default-editor integration that changes nothing, and therefore
//! the half a settings screen can call freely: "are we the default for `.txt`?
//! for `.md`?" is a question, and answering it needs no consent, no elevation
//! and no undo.
//!
//! Every function here takes the *already-read* state as a value -- the text
//! of a `mimeapps.list`, or a map of extension to ProgID -- rather than
//! reading it. Two reasons, and the second is the one that matters: the
//! parsing is then testable without a home directory, and the Windows shape
//! can be tested on Linux even though nothing here can read a Windows
//! registry.
//!
//! ## Windows: asked through a trait
//!
//! On Windows the current association lives in the registry
//! (`...\Explorer\FileExts\.<ext>\UserChoice`), and this crate reads no
//! registry: it has no dependency and forbids `unsafe`. So the question is
//! a trait, [`ProgIdQuery`], which `bp-platform-windows` implements over
//! `RegGetValueW` and the shell passes to [`report_from_query`]. This crate
//! keeps the logic -- ours, somebody else's, nobody's -- and the logic is
//! tested on Linux with a query made of a map.
//!
//! Without a query the answer is [`AssociationState::Unknown`] with the reason
//! attached, never [`AssociationState::Unclaimed`] -- a UI that renders "not
//! registered" when it means "could not look" invites the user to fix
//! something that is not broken.

use std::collections::{BTreeMap, BTreeSet};

use super::AppInfo;
use super::file_type::AssociationSelection;

/// The section of a `mimeapps.list` that decides the default handler.
///
/// `[Added Associations]` is deliberately not consulted: it lists applications
/// that *can* open a type, which is the question "is it in the Open With
/// list", not "is it the default". Conflating them makes a product that has
/// merely registered report itself as the default.
const DEFAULT_APPLICATIONS: &str = "Default Applications";

/// The section listing applications that can open a type in addition to the
/// ones whose `.desktop` entries claim it. Consulted only for the fallback,
/// once no `[Default Applications]` entry anywhere in the chain names an
/// installed application.
const ADDED_ASSOCIATIONS: &str = "Added Associations";

/// The section taking a type away from an application whose `.desktop` entry
/// claims it. Like [`ADDED_ASSOCIATIONS`], it shapes only the fallback: the
/// specification is explicit that an explicit default wins regardless.
const REMOVED_ASSOCIATIONS: &str = "Removed Associations";

/// The only section of a `mimeinfo.cache`, as `update-desktop-database`
/// writes it.
const MIME_CACHE: &str = "MIME Cache";

/// Who opens a file type at the moment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssociationState {
    /// This product.
    Ours,
    /// Another application. Named by whatever identifier the platform uses --
    /// a `.desktop` file name on Linux, a ProgID on Windows -- rather than a
    /// pretty name, because resolving one to the other is another lookup and
    /// an unresolvable one would have to be reported as unknown anyway.
    Other { handler: String },
    /// Nothing claims it.
    Unclaimed,
    /// The state could not be read. Distinct from [`Self::Unclaimed`], and the
    /// reason says which obstacle it hit.
    Unknown { reason: &'static str },
}

impl AssociationState {
    /// A sentence for a settings row.
    #[must_use]
    pub fn describe(&self, extension: &str) -> String {
        match self {
            Self::Ours => format!("BachelorPad+ opens .{extension} files."),
            Self::Other { handler } => format!("{handler} opens .{extension} files."),
            Self::Unclaimed => format!("Nothing is set to open .{extension} files."),
            Self::Unknown { reason } => {
                format!("Cannot tell what opens .{extension} files. {reason}")
            }
        }
    }
}

/// One row of an [`AssociationReport`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssociationEntry {
    /// The extension, without a dot.
    pub extension: &'static str,
    /// The MIME type, which is what the state was actually looked up by on
    /// Linux. Carried so a diagnostic can show why a lookup missed -- an
    /// association against `text/x-markdown` when the table says
    /// `text/markdown` is invisible otherwise.
    pub mime: &'static str,
    /// Who opens it.
    pub state: AssociationState,
}

/// What currently opens each of the selected file types.
///
/// Ordered like the selection, so two reports taken before and after a change
/// line up row by row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssociationReport {
    entries: Vec<AssociationEntry>,
}

impl AssociationReport {
    /// The rows.
    #[must_use]
    pub fn entries(&self) -> &[AssociationEntry] {
        &self.entries
    }

    /// The state of one extension, if it was in the selection.
    #[must_use]
    pub fn state_of(&self, extension: &str) -> Option<&AssociationState> {
        let wanted = extension
            .trim()
            .trim_start_matches('.')
            .to_ascii_lowercase();
        self.entries
            .iter()
            .find(|entry| entry.extension == wanted)
            .map(|entry| &entry.state)
    }

    /// Whether this product opens `extension`.
    ///
    /// False for both "somebody else does" and "cannot tell", which is the
    /// right default for anything that acts on the answer: a caller offering
    /// to register should offer when the answer is unknown.
    #[must_use]
    pub fn is_ours(&self, extension: &str) -> bool {
        matches!(self.state_of(extension), Some(AssociationState::Ours))
    }

    /// How many of the selected types this product opens.
    #[must_use]
    pub fn ours(&self) -> usize {
        self.entries
            .iter()
            .filter(|entry| entry.state == AssociationState::Ours)
            .count()
    }

    /// A one-line summary for a settings screen.
    ///
    /// Says "of the types you chose", not "of all types", because a user who
    /// picked Notepad Replacement and sees "6 of 34" will reasonably think
    /// something failed.
    #[must_use]
    pub fn summary(&self) -> String {
        let total = self.entries.len();
        if total == 0 {
            return "No file types are selected.".to_owned();
        }
        let unknown = self
            .entries
            .iter()
            .filter(|entry| matches!(entry.state, AssociationState::Unknown { .. }))
            .count();
        if unknown == total {
            return format!("Cannot tell which application opens the {total} selected file types.");
        }
        format!(
            "BachelorPad+ opens {} of the {total} selected file types.",
            self.ours()
        )
    }
}

/// The report a Linux `mimeapps.list` implies.
///
/// `mimeapps` is the file's contents. An absent file is an empty string and
/// yields [`AssociationState::Unclaimed`] throughout, which is correct: a user
/// who has never chosen a handler has none recorded.
#[must_use]
pub fn report_from_mimeapps(
    mimeapps: &str,
    app: &AppInfo,
    selection: &AssociationSelection,
) -> AssociationReport {
    let defaults = default_applications(mimeapps);
    let ours = super::desktop::desktop_file_name(app);
    AssociationReport {
        entries: selection
            .file_types()
            .map(|file_type| AssociationEntry {
                extension: file_type.extension,
                mime: file_type.mime,
                state: match defaults.get(file_type.mime).and_then(|v| v.first()) {
                    None => AssociationState::Unclaimed,
                    Some(handler) if *handler == ours => AssociationState::Ours,
                    Some(handler) => AssociationState::Other {
                        handler: handler.clone(),
                    },
                },
            })
            .collect(),
    }
}

/// The report a map of extension to ProgID implies.
///
/// The Windows shape, taking the values a caller read rather than reading
/// them, so the logic is exercised by the Linux leg of CI too. Keys may be
/// given with or without a leading dot and in any case.
///
/// Takes no [`AppInfo`], unlike its Linux counterpart, and the asymmetry is
/// the platforms' rather than an oversight: on Linux the product is identified
/// by the name of a file it installs, which only the caller knows, while on
/// Windows it is identified by a ProgID this crate owns outright.
#[must_use]
pub fn report_from_prog_ids(
    prog_ids: &BTreeMap<String, String>,
    selection: &AssociationSelection,
) -> AssociationReport {
    let normalised: BTreeMap<String, &str> = prog_ids
        .iter()
        .map(|(key, value)| {
            (
                key.trim().trim_start_matches('.').to_ascii_lowercase(),
                value.as_str(),
            )
        })
        .collect();
    AssociationReport {
        entries: selection
            .file_types()
            .map(|file_type| AssociationEntry {
                extension: file_type.extension,
                mime: file_type.mime,
                state: match normalised.get(file_type.extension) {
                    None => AssociationState::Unclaimed,
                    Some(id)
                        if super::windows::prog_id(file_type.extension)
                            .eq_ignore_ascii_case(id) =>
                    {
                        AssociationState::Ours
                    }
                    Some(id) => AssociationState::Other {
                        handler: (*id).to_owned(),
                    },
                },
            })
            .collect(),
    }
}

/// Which ProgID opens a file extension on Windows, asked of whatever can read
/// the registry.
///
/// A trait here and an implementation in `bp-platform-windows`, because this
/// crate stays dependency-free and `forbid(unsafe_code)` (ADR-0033) while the
/// answer needs a Win32 call. The direction matters: the Windows crate
/// depends on this one, never the reverse.
pub trait ProgIdQuery {
    /// The ProgID that opens `extension` (no leading dot, lowercase) for the
    /// current user, or `None` if nothing is recorded for it.
    ///
    /// `None` means *nothing is set*, and is reported as
    /// [`AssociationState::Unclaimed`]. An implementation that cannot look
    /// at all is not a `ProgIdQuery`: it should not be passed, and the
    /// caller reports "cannot tell" instead.
    fn current_prog_id(&self, extension: &str) -> Option<String>;
}

/// A map of extension to ProgID answers the question by lookup, so a test --
/// or a caller holding values it read some other way -- needs no type of its
/// own. Keys are matched as [`report_from_prog_ids`] matches them.
impl ProgIdQuery for BTreeMap<String, String> {
    fn current_prog_id(&self, extension: &str) -> Option<String> {
        self.iter()
            .find(|(key, _)| {
                key.trim()
                    .trim_start_matches('.')
                    .eq_ignore_ascii_case(extension)
            })
            .map(|(_, id)| id.clone())
    }
}

/// The report a [`ProgIdQuery`] implies, asking it once per selected type.
///
/// How Windows answers "what opens `.txt` now": the shell hands in
/// `bp-platform-windows`' registry reader, and this function decides what
/// each answer means -- the same way [`report_from_prog_ids`] does, which is
/// what a test on Linux checks.
#[must_use]
pub fn report_from_query(
    query: &dyn ProgIdQuery,
    selection: &AssociationSelection,
) -> AssociationReport {
    let prog_ids: BTreeMap<String, String> = selection
        .file_types()
        .filter_map(|file_type| {
            query
                .current_prog_id(file_type.extension)
                .map(|id| (file_type.extension.to_owned(), id))
        })
        .collect();
    report_from_prog_ids(&prog_ids, selection)
}

/// A report that says, for every selected type, that the state could not be
/// read.
///
/// The honest answer on Windows when no [`ProgIdQuery`] was supplied. A function rather than
/// an ad-hoc construction at the call site so that the shape of "we do not
/// know" is identical everywhere and a UI can special-case it once.
#[must_use]
pub fn unknown_report(reason: &'static str, selection: &AssociationSelection) -> AssociationReport {
    AssociationReport {
        entries: selection
            .file_types()
            .map(|file_type| AssociationEntry {
                extension: file_type.extension,
                mime: file_type.mime,
                state: AssociationState::Unknown { reason },
            })
            .collect(),
    }
}

/// The `[Default Applications]` section of a `mimeapps.list`, as a map from
/// MIME type to the desktop files listed for it, in order.
///
/// Public because it is the useful half on its own -- a diagnostic wants to
/// print what the file says, not just what it implies about us.
///
/// The format is the desktop-entry one, and the tolerances here match what
/// real files contain: comment lines beginning `#`, blank lines, whitespace
/// around `=`, a trailing `;` on the list, and a first-wins rule for a key
/// that appears twice, which is what the desktop-entry specification says
/// about duplicate keys.
#[must_use]
pub fn default_applications(mimeapps: &str) -> BTreeMap<String, Vec<String>> {
    list_section(mimeapps, DEFAULT_APPLICATIONS)
}

/// One section of a `mimeapps.list` or `mimeinfo.cache`, as a map from MIME
/// type to the desktop file ids listed for it, in order.
///
/// Both files are the same format -- desktop-entry groups whose values are
/// `;`-separated lists of ids -- so one parser reads every section of either;
/// see [`default_applications`] for the tolerances.
#[must_use]
pub fn list_section(text: &str, section: &str) -> BTreeMap<String, Vec<String>> {
    let mut out: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut in_section = false;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            in_section = name.trim() == section;
            continue;
        }
        if !in_section {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let handlers: Vec<String> = value
            .split(';')
            .map(str::trim)
            .filter(|h| !h.is_empty())
            .map(str::to_owned)
            .collect();
        if handlers.is_empty() {
            continue;
        }
        out.entry(key.trim().to_owned()).or_insert(handlers);
    }
    out
}

/// One `mimeapps.list`, parsed into the three sections the lookup uses.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct MimeappsList {
    defaults: BTreeMap<String, Vec<String>>,
    added: BTreeMap<String, Vec<String>>,
    removed: BTreeMap<String, Vec<String>>,
}

/// Every file that decides which application opens a type, read and parsed,
/// highest precedence first.
///
/// The freedesktop "Association between MIME types and applications"
/// specification spreads the answer over up to a dozen files -- a
/// desktop-specific `mimeapps.list` beats the generic one, the user's beats
/// the administrator's (`/etc/xdg`), which beats the distribution's
/// (`/usr/share/applications`) -- and falls back to the `mimeinfo.cache` that
/// `update-desktop-database` builds from every installed `.desktop` entry.
/// Reading only `~/.config/mimeapps.list` answers "what did the user pick"
/// and reports "nothing" on a fresh install where the distribution's default
/// is what actually opens the file. This value is the whole chain, built from
/// text so the precedence rules are testable without a filesystem;
/// [`super::linux::association_report_in`] reads it from the real one.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AssociationChain {
    lists: Vec<MimeappsList>,
    caches: Vec<BTreeMap<String, Vec<String>>>,
}

impl AssociationChain {
    /// The chain made of `lists` (the text of each `mimeapps.list`, highest
    /// precedence first) and `caches` (each `mimeinfo.cache`, in data
    /// directory order). An absent file is simply not in either.
    #[must_use]
    pub fn from_texts<'a>(
        lists: impl IntoIterator<Item = &'a str>,
        caches: impl IntoIterator<Item = &'a str>,
    ) -> Self {
        Self {
            lists: lists
                .into_iter()
                .map(|text| MimeappsList {
                    defaults: list_section(text, DEFAULT_APPLICATIONS),
                    added: list_section(text, ADDED_ASSOCIATIONS),
                    removed: list_section(text, REMOVED_ASSOCIATIONS),
                })
                .collect(),
            caches: caches
                .into_iter()
                .map(|text| list_section(text, MIME_CACHE))
                .collect(),
        }
    }

    /// The desktop file id that opens `mime`, per the specification's
    /// "Default Application" algorithm, or `None` when nothing does.
    ///
    /// `is_installed` answers whether a desktop file id names an installed
    /// application. The specification skips a default that is not -- a
    /// `mimeapps.list` naming an application the user has since removed
    /// must fall through to the next one rather than report a handler that
    /// cannot launch -- and whether it is installed is a filesystem question
    /// this value does not ask itself.
    ///
    /// 1. The first installed id under `[Default Applications]`, in the
    ///    first file that has one. Removed associations do not apply here.
    /// 2. Failing that, the most preferred application that can open the
    ///    type: `[Added Associations]` in precedence order, then each
    ///    `mimeinfo.cache`, skipping any id that a file of *higher*
    ///    precedence lists under `[Removed Associations]`.
    #[must_use]
    pub fn default_for(&self, mime: &str, is_installed: impl Fn(&str) -> bool) -> Option<String> {
        for list in &self.lists {
            if let Some(found) = list
                .defaults
                .get(mime)
                .and_then(|ids| ids.iter().find(|id| is_installed(id)))
            {
                return Some(found.clone());
            }
        }
        // A removal hides an association from the files *below* it, so the
        // set grows as the walk descends, and is checked before each file's
        // own additions are considered.
        let mut removed: BTreeSet<&str> = BTreeSet::new();
        for list in &self.lists {
            if let Some(found) = list.added.get(mime).and_then(|ids| {
                ids.iter()
                    .find(|id| !removed.contains(id.as_str()) && is_installed(id))
            }) {
                return Some(found.clone());
            }
            if let Some(ids) = list.removed.get(mime) {
                removed.extend(ids.iter().map(String::as_str));
            }
        }
        self.caches.iter().find_map(|cache| {
            cache.get(mime).and_then(|ids| {
                ids.iter()
                    .find(|id| !removed.contains(id.as_str()) && is_installed(id))
                    .cloned()
            })
        })
    }
}

/// The report the whole association chain implies.
///
/// What a double-click on each selected type opens, which is the question a
/// user means by "is BachelorPad+ my default". `is_installed` is passed
/// through to [`AssociationChain::default_for`].
#[must_use]
pub fn report_from_chain(
    chain: &AssociationChain,
    is_installed: impl Fn(&str) -> bool,
    app: &AppInfo,
    selection: &AssociationSelection,
) -> AssociationReport {
    let ours = super::desktop::desktop_file_name(app);
    AssociationReport {
        entries: selection
            .file_types()
            .map(|file_type| AssociationEntry {
                extension: file_type.extension,
                mime: file_type.mime,
                state: match chain.default_for(file_type.mime, &is_installed) {
                    None => AssociationState::Unclaimed,
                    Some(handler) if handler == ours => AssociationState::Ours,
                    Some(handler) => AssociationState::Other { handler },
                },
            })
            .collect(),
    }
}
