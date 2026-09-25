//! Set as Default on Linux, resolved against the machine it runs on.
//!
//! [`super::plan`] answers "what would a registration consist of" from the
//! application and the file types alone. That is not enough to act on, for
//! four reasons that each come from a real way of installing this product:
//!
//! * **A distribution package** has already put
//!   `/usr/share/applications/<id>.desktop` and a MIME package in place. A
//!   per-user copy under `~/.local/share` would shadow it -- same id, higher
//!   precedence -- and keep launching whatever path it named after the package
//!   is upgraded or removed. So [`linux_plan`] writes no per-user copy of a
//!   file the system already provides, and the plan shrinks to `xdg-mime
//!   default`.
//! * **An AppImage** runs from a temporary mount that is gone when it exits.
//!   [`AppInfo::launched_from`] names the `.AppImage` file instead.
//! * **A Flatpak** is sandboxed: its `.desktop` entry is exported by the
//!   Flatpak runtime, the user's `~/.local/share` is not the one the host
//!   desktop reads, and the default is chosen in the host's own settings.
//!   Registration is unavailable there, with [`FLATPAK_REASON`], and the shell
//!   greys the row.
//! * **What opens a file today** is decided by up to a dozen files, not by
//!   `~/.config/mimeapps.list` alone. [`association_report_in`] follows the
//!   freedesktop chain in full.
//!
//! Everything the environment contributes arrives as a [`DesktopEnv`] value,
//! including whether `/.flatpak-info` exists, so every rule here is tested
//! against temporary XDG roots and none of them against the machine running
//! the tests.
//!
//! Nothing here runs a program. [`LinuxPlan::commands`] and
//! [`Uninstalled::follow_up`] are the command lines, as data, for the shell to
//! run once the user has agreed to exactly those lines.

use std::fs;
use std::path::{Path, PathBuf};

use super::desktop::{self, is_per_user_registration};
use super::state::{self, AssociationChain};
use super::{
    AppInfo, ArtefactKind, AssociationReport, AssociationSelection, Consent, FollowUp,
    InstallRefusal, RegistrationPlan, UPDATE_DESKTOP_DATABASE, UPDATE_MIME_DATABASE,
    WHY_DESKTOP_DATABASE, WHY_MIME_DATABASE, check_relative_path, refuse_unsupported, under_root,
};
use crate::capabilities::{Availability, Capability};
use crate::paths::{is_absolute, join};
use crate::{DirKind, EnvSnapshot, Platform};

/// Why Set as Default is unavailable inside a Flatpak sandbox.
///
/// One sentence for the greyed row, the refusal and the dialog, so the three
/// cannot drift apart.
pub const FLATPAK_REASON: &str = "BachelorPad+ is running as a Flatpak, and the sandbox \
                                  manages file associations: its entry is already in Open \
                                  With, and the default is chosen in the desktop's own \
                                  settings.";

/// `$XDG_DATA_DIRS` when it is unset or blank, per the Base Directory
/// Specification.
const DEFAULT_DATA_DIRS: &[&str] = &["/usr/local/share", "/usr/share"];

/// `$XDG_CONFIG_DIRS` when it is unset or blank. Where an administrator's
/// `mimeapps.list` lives.
const DEFAULT_CONFIG_DIRS: &[&str] = &["/etc/xdg"];

/// Why a system `.desktop` entry stops a per-user copy being written.
const SKIP_SYSTEM_ENTRY: &str = "A package installed this entry for every user; a per-user \
                                 copy would only shadow it.";

/// Why a system MIME package stops a per-user copy being written.
const SKIP_SYSTEM_PACKAGE: &str = "A package installed this MIME package for every user; a \
                                   per-user copy would define the same types twice.";

/// Why a per-user file with this product's name is neither overwritten nor
/// removed.
const SKIP_NOT_OURS: &str = "Set as Default did not write this file -- it is most likely a \
                             copy a menu editor made when it was customised -- so it is left \
                             as it is.";

/// Why something at a registration path that is not a regular file is left
/// alone.
const SKIP_NOT_A_FILE: &str = "This is not a regular file, so it is left as it is.";

/// Why `xdg-mime default` is the step that matters.
const WHY_SET_DEFAULT: &str = "Until this runs, BachelorPad+ is only offered in Open With; \
                               this is the step that makes it the default for these types.";

/// Why the desktop database is rebuilt after a removal.
const WHY_DESKTOP_DATABASE_AFTER_REMOVAL: &str =
    "Until the desktop database is rebuilt, Open With keeps listing the removed entry.";

/// Why the MIME database is rebuilt after a removal.
const WHY_MIME_DATABASE_AFTER_REMOVAL: &str = "Until the MIME database is rebuilt, the types \
                                               the removed package defined are still defined.";

/// The environment Linux desktop integration depends on, as a value.
///
/// [`EnvSnapshot`] carries the per-user roots every part of the product
/// needs; this adds the variables only desktop integration reads. A separate
/// struct rather than more fields on [`EnvSnapshot`], because that one is
/// built field by field in other crates' tests and every new field there
/// would be a compile error somewhere unrelated.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DesktopEnv {
    /// `$HOME` and the `$XDG_*_HOME` variables.
    pub dirs: EnvSnapshot,
    /// `$XDG_DATA_DIRS`, colon-separated.
    pub xdg_data_dirs: Option<String>,
    /// `$XDG_CONFIG_DIRS`, colon-separated.
    pub xdg_config_dirs: Option<String>,
    /// `$XDG_CURRENT_DESKTOP`, colon-separated, e.g. `ubuntu:GNOME`.
    pub xdg_current_desktop: Option<String>,
    /// `$APPIMAGE`: the path of the `.AppImage` file, set by its runtime.
    pub appimage: Option<String>,
    /// `$FLATPAK_ID`, set inside a Flatpak sandbox.
    pub flatpak_id: Option<String>,
    /// Whether `/.flatpak-info` exists -- the sandbox's own marker, present
    /// even when a launcher has scrubbed the environment.
    pub flatpak_info_present: bool,
}

impl DesktopEnv {
    /// Read the real process environment, and whether `/.flatpak-info`
    /// exists.
    ///
    /// The only function here that looks at the machine to build the value;
    /// everything else takes the value.
    #[must_use]
    pub fn from_environment() -> Self {
        let get = |key: &str| {
            std::env::var_os(key)
                .and_then(|value| value.into_string().ok())
                .filter(|value| !value.trim().is_empty())
        };
        Self {
            dirs: EnvSnapshot::from_environment(),
            xdg_data_dirs: get("XDG_DATA_DIRS"),
            xdg_config_dirs: get("XDG_CONFIG_DIRS"),
            xdg_current_desktop: get("XDG_CURRENT_DESKTOP"),
            appimage: get("APPIMAGE"),
            flatpak_id: get("FLATPAK_ID"),
            flatpak_info_present: Path::new("/.flatpak-info").exists(),
        }
    }

    /// An environment with these per-user roots and nothing else set.
    #[must_use]
    pub fn from_dirs(dirs: EnvSnapshot) -> Self {
        Self {
            dirs,
            ..Self::default()
        }
    }

    /// Whether the process is inside a Flatpak sandbox.
    #[must_use]
    pub fn is_flatpak(&self) -> bool {
        self.flatpak_info_present
            || self
                .flatpak_id
                .as_deref()
                .is_some_and(|id| !id.trim().is_empty())
    }

    /// The `.AppImage` file this process was started from, if any.
    ///
    /// Only an absolute value counts: a relative `$APPIMAGE` names a file
    /// relative to wherever the desktop happens to launch the entry from,
    /// which is nowhere in particular.
    #[must_use]
    pub fn appimage_path(&self) -> Option<&str> {
        self.appimage
            .as_deref()
            .filter(|path| is_absolute(Platform::Linux, path))
    }

    /// The desktops named by `$XDG_CURRENT_DESKTOP`, lowercased, in order.
    ///
    /// Lowercased because that is how the specification names the
    /// desktop-specific files (`gnome-mimeapps.list` for `GNOME`).
    #[must_use]
    pub fn current_desktops(&self) -> Vec<String> {
        self.xdg_current_desktop
            .as_deref()
            .unwrap_or_default()
            .split(':')
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(str::to_ascii_lowercase)
            .collect()
    }

    /// `$XDG_DATA_HOME`, or its `$HOME` fallback.
    #[must_use]
    pub fn data_home(&self) -> Option<String> {
        root_text(DirKind::Data, &self.dirs)
    }

    /// `$XDG_CONFIG_HOME`, or its `$HOME` fallback.
    #[must_use]
    pub fn config_home(&self) -> Option<String> {
        root_text(DirKind::Config, &self.dirs)
    }

    /// `$XDG_DATA_DIRS`, in precedence order.
    #[must_use]
    pub fn data_dirs(&self) -> Vec<String> {
        search_path(self.xdg_data_dirs.as_deref(), DEFAULT_DATA_DIRS)
    }

    /// `$XDG_CONFIG_DIRS`, in precedence order.
    #[must_use]
    pub fn config_dirs(&self) -> Vec<String> {
        search_path(self.xdg_config_dirs.as_deref(), DEFAULT_CONFIG_DIRS)
    }

    /// Every `mimeapps.list` the specification consults, highest precedence
    /// first, whether or not it exists.
    ///
    /// In each directory, one `$desktop-mimeapps.list` per entry of
    /// `$XDG_CURRENT_DESKTOP` comes before the generic file. The directories
    /// are the user's configuration, the system configuration (`/etc/xdg`),
    /// then the user's and the system's `applications` data directories --
    /// the last two deprecated for writing, still honoured for reading.
    #[must_use]
    pub fn mimeapps_lists(&self) -> Vec<PathBuf> {
        let mut directories: Vec<String> = Vec::new();
        directories.extend(self.config_home());
        directories.extend(self.config_dirs());
        directories.extend(self.application_dir_texts());
        let desktops = self.current_desktops();
        directories
            .iter()
            .flat_map(|directory| {
                desktops
                    .iter()
                    .map(|desktop| format!("{desktop}-mimeapps.list"))
                    .chain(std::iter::once("mimeapps.list".to_owned()))
                    .map(move |name| PathBuf::from(join(Platform::Linux, directory, &[&name])))
            })
            .collect()
    }

    /// Every `applications` directory, user first, then `$XDG_DATA_DIRS` in
    /// order -- where desktop entries and `mimeinfo.cache` live.
    #[must_use]
    pub fn application_dirs(&self) -> Vec<PathBuf> {
        self.application_dir_texts()
            .into_iter()
            .map(PathBuf::from)
            .collect()
    }

    /// Whether the desktop file id `id` names an installed application.
    ///
    /// A desktop file id is the entry's path under `applications` with `/`
    /// turned into `-`, so `kde4-kate.desktop` may be
    /// `applications/kde4/kate.desktop`. One level of subdirectory is tried,
    /// which covers every layout distributions use.
    #[must_use]
    pub fn is_installed(&self, id: &str) -> bool {
        let candidates = desktop_id_candidates(id);
        self.application_dirs()
            .iter()
            .any(|dir| candidates.iter().any(|c| under_root(dir, c).is_file()))
    }

    /// `applications` under the data home and every data directory, as text.
    fn application_dir_texts(&self) -> Vec<String> {
        self.data_home()
            .into_iter()
            .chain(self.data_dirs())
            .map(|dir| join(Platform::Linux, &dir, &["applications"]))
            .collect()
    }

    /// The first system data directory holding `relative_path`, as a path.
    fn system_file(&self, relative_path: &str) -> Option<PathBuf> {
        self.data_dirs()
            .iter()
            .map(|dir| under_root(Path::new(dir), relative_path))
            .find(|path| path.is_file())
    }
}

/// The Linux root for `kind` as text.
fn root_text(kind: DirKind, env: &EnvSnapshot) -> Option<String> {
    crate::dirs::root(Platform::Linux, kind, env).and_then(|p| p.to_str().map(str::to_owned))
}

/// A colon-separated XDG search path, or `default` when it is unset or blank.
///
/// Relative entries are dropped rather than used, as the specification
/// requires: a relative entry is relative to wherever the editor was
/// launched from, which is not a system directory.
fn search_path(value: Option<&str>, default: &[&str]) -> Vec<String> {
    match value.map(str::trim).filter(|v| !v.is_empty()) {
        None => default.iter().map(|&d| d.to_owned()).collect(),
        Some(value) => value
            .split(':')
            .map(|entry| entry.trim_end_matches('/'))
            .filter(|entry| is_absolute(Platform::Linux, entry))
            .map(str::to_owned)
            .collect(),
    }
}

/// The relative paths a desktop file id may be installed at.
fn desktop_id_candidates(id: &str) -> Vec<String> {
    let mut candidates = vec![id.to_owned()];
    for (index, _) in id.match_indices('-') {
        candidates.push(format!("{}/{}", &id[..index], &id[index + 1..]));
    }
    candidates
}

/// Whether Set as Default can be offered here, and why not when not.
///
/// The run-time answer to [`Capability::AssociationInstall`], which is
/// [`Availability::Conditional`] on Linux precisely because the condition --
/// not being in a Flatpak -- is a fact about the environment. The shell greys
/// the row, and "Remove Registration" with it, on anything but
/// [`Availability::Available`], and shows [`Availability::reason`] beside it.
#[must_use]
pub fn set_as_default_availability(platform: Platform, env: &DesktopEnv) -> Availability {
    match platform {
        Platform::Linux if env.is_flatpak() => Availability::Unsupported {
            reason: FLATPAK_REASON,
        },
        Platform::Linux => Availability::Available,
        Platform::Windows => Capability::AssociationInstall.availability(platform),
    }
}

/// What currently opens the selected file types, following the whole
/// freedesktop chain on Linux.
///
/// Reads, in order: `$XDG_CONFIG_HOME/$desktop-mimeapps.list` and
/// `$XDG_CONFIG_HOME/mimeapps.list`; the same pair in each `$XDG_CONFIG_DIRS`
/// entry (`/etc/xdg`); the same pair in `$XDG_DATA_HOME/applications` and
/// each `$XDG_DATA_DIRS/applications`; and, only when no
/// `[Default Applications]` entry names an installed application, the
/// `[Added Associations]`, `[Removed Associations]` and `mimeinfo.cache`
/// fallback -- see [`AssociationChain::default_for`]. A file that does not
/// exist or cannot be read is skipped, as the desktop skips it.
///
/// On Windows, "cannot tell" for everything, as [`super::association_report`]
/// says.
#[must_use]
pub fn association_report_in(
    platform: Platform,
    env: &DesktopEnv,
    app: &AppInfo,
    selection: &AssociationSelection,
) -> AssociationReport {
    match platform {
        Platform::Linux => {
            let read = |path: &PathBuf| fs::read_to_string(path).ok();
            let lists: Vec<String> = env.mimeapps_lists().iter().filter_map(read).collect();
            let caches: Vec<String> = env
                .application_dirs()
                .iter()
                .map(|dir| dir.join("mimeinfo.cache"))
                .filter_map(|path| read(&path))
                .collect();
            let chain = AssociationChain::from_texts(
                lists.iter().map(String::as_str),
                caches.iter().map(String::as_str),
            );
            state::report_from_chain(&chain, |id| env.is_installed(id), app, selection)
        }
        Platform::Windows => super::association_report(platform, None, app, selection),
    }
}

/// A file a plan will not write, or a removal will not remove, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skipped {
    /// The file.
    pub path: PathBuf,
    /// A sentence for the dialog.
    pub why: &'static str,
}

/// Set as Default on this Linux machine: what to write, where, and the
/// commands that finish the job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinuxPlan {
    /// What [`super::install`] is handed. Its artefacts are only the files
    /// this machine lacks -- possibly none -- and its `follow_up` names
    /// absolute directories under [`Self::root`].
    pub registration: RegistrationPlan,
    /// `$XDG_DATA_HOME`: the root to pass to [`super::install`].
    pub root: PathBuf,
    /// Files the plan deliberately does not write, for the dialog.
    pub skipped: Vec<Skipped>,
    /// `xdg-mime default <id>.desktop <mime>...`.
    pub set_default: FollowUp,
}

impl LinuxPlan {
    /// Every command to run once [`super::install`] has succeeded, in order:
    /// the database rebuilds for what was written, then `xdg-mime default`.
    ///
    /// The shell shows these exact lines in the consent dialog, runs them
    /// only after "Set as Default" is confirmed, and runs every one even if
    /// an earlier one fails -- the rebuilds refresh caches, and a missing
    /// `update-desktop-database` is no reason not to record the user's
    /// choice.
    #[must_use]
    pub fn commands(&self) -> Vec<FollowUp> {
        self.registration
            .follow_up
            .iter()
            .cloned()
            .chain(std::iter::once(self.set_default.clone()))
            .collect()
    }
}

/// Set as Default for `selection`, resolved against `env`.
///
/// Builds [`super::plan`]'s Linux registration and then drops what this
/// machine already has:
///
/// * the `.desktop` entry, when a system data directory holds one with this
///   id whose `MimeType=` covers the selection and no per-user copy exists
///   yet. An existing per-user copy this product wrote is rewritten, because
///   it shadows the system one either way and should at least be current;
/// * the MIME package, when a system one with this id defines every type the
///   per-user one would;
/// * any per-user file with this product's name that the product did not
///   write (see [`desktop::PER_USER_MARKER`]), which is never overwritten.
///
/// # Errors
///
/// [`InstallRefusal::NotSupported`] inside a Flatpak, with [`FLATPAK_REASON`];
/// [`InstallRefusal::NothingSelected`] for an empty selection;
/// [`InstallRefusal::NoDataDirectory`] when the environment names no data
/// home.
pub fn linux_plan(
    app: &AppInfo,
    selection: &AssociationSelection,
    env: &DesktopEnv,
) -> Result<LinuxPlan, InstallRefusal> {
    refuse_in_flatpak(env)?;
    if selection.is_empty() {
        return Err(InstallRefusal::NothingSelected);
    }
    let root = env.data_home().ok_or(InstallRefusal::NoDataDirectory)?;

    let mut registration = super::plan(Platform::Linux, app, selection);
    let mut skipped = Vec::new();
    registration.artefacts.retain(|artefact| {
        let existing = under_root(Path::new(&root), &artefact.relative_path);
        if let Some(why) = per_user_obstacle(&existing) {
            skipped.push(Skipped {
                path: existing,
                why,
            });
            return false;
        }
        if existing.exists() {
            return true;
        }
        let provided = match artefact.kind {
            ArtefactKind::DesktopEntry => env
                .system_file(&artefact.relative_path)
                .filter(|path| desktop_entry_covers(path, selection))
                .map(|path| (path, SKIP_SYSTEM_ENTRY)),
            ArtefactKind::MimePackage => env
                .system_file(&artefact.relative_path)
                .filter(|path| mime_package_covers(path, selection))
                .map(|path| (path, SKIP_SYSTEM_PACKAGE)),
            ArtefactKind::RegistryScript => None,
        };
        match provided {
            Some((path, why)) => {
                skipped.push(Skipped { path, why });
                false
            }
            None => true,
        }
    });

    registration.follow_up = registration
        .artefacts
        .iter()
        .filter_map(|artefact| match artefact.kind {
            ArtefactKind::DesktopEntry => Some(FollowUp {
                program: UPDATE_DESKTOP_DATABASE,
                args: vec![join(Platform::Linux, &root, &["applications"])],
                why: WHY_DESKTOP_DATABASE,
            }),
            ArtefactKind::MimePackage => Some(FollowUp {
                program: UPDATE_MIME_DATABASE,
                args: vec![join(Platform::Linux, &root, &["mime"])],
                why: WHY_MIME_DATABASE,
            }),
            ArtefactKind::RegistryScript => None,
        })
        .collect();

    let set_default = FollowUp {
        program: "xdg-mime",
        args: ["default".to_owned(), desktop::desktop_file_name(app)]
            .into_iter()
            .chain(selection.mime_types().into_iter().map(str::to_owned))
            .collect(),
        why: WHY_SET_DEFAULT,
    };

    Ok(LinuxPlan {
        registration,
        root: PathBuf::from(root),
        skipped,
        set_default,
    })
}

/// What Remove Registration would remove, before anything is removed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unregistration {
    /// `$XDG_DATA_HOME`: the root to pass to [`uninstall`].
    pub root: PathBuf,
    /// Paths under the root to remove, `/`-separated. Only files a per-user
    /// registration wrote -- nothing in a system directory is ever a
    /// candidate, because the root is the user's own data home.
    pub remove: Vec<String>,
    /// Files with this product's name that will be left alone, and why.
    pub skipped: Vec<Skipped>,
}

impl Unregistration {
    /// Whether there is nothing to remove -- the shell greys "Remove
    /// Registration" then, rather than offering a dialog that does nothing.
    /// True after a package install, whose files the package manager owns.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.remove.is_empty()
    }
}

/// What Remove Registration would remove on this machine.
///
/// Exactly the files a per-user registration writes -- the `.desktop` entry
/// and the MIME package under `$XDG_DATA_HOME` -- and only those that carry
/// [`desktop::PER_USER_MARKER`]. It does not touch `mimeapps.list`: the
/// defaults recorded there are the user's choices, a desktop skips one that
/// names an application no longer installed, and the desktop's own settings
/// are where a choice is changed.
///
/// # Errors
///
/// [`InstallRefusal::NotSupported`] inside a Flatpak;
/// [`InstallRefusal::NoDataDirectory`] when the environment names no data
/// home.
pub fn unregistration_plan(
    app: &AppInfo,
    env: &DesktopEnv,
) -> Result<Unregistration, InstallRefusal> {
    refuse_in_flatpak(env)?;
    let root = env.data_home().ok_or(InstallRefusal::NoDataDirectory)?;
    let root = PathBuf::from(root);
    let mut remove = Vec::new();
    let mut skipped = Vec::new();
    for relative_path in registration_paths(app) {
        let path = under_root(&root, &relative_path);
        // `symlink_metadata` rather than `exists`, which follows links: a
        // dangling link at our path is still something to report.
        if fs::symlink_metadata(&path).is_err() {
            continue;
        }
        match per_user_obstacle(&path) {
            Some(why) => skipped.push(Skipped { path, why }),
            None => remove.push(relative_path),
        }
    }
    Ok(Unregistration {
        root,
        remove,
        skipped,
    })
}

/// What an uninstall actually did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Uninstalled {
    /// The files removed, as absolute paths under the root that was given.
    pub removed: Vec<PathBuf>,
    /// Files left alone because they were no longer this product's to
    /// remove by the time the removal ran.
    pub skipped: Vec<Skipped>,
    /// The database rebuilds for the directories something was removed from,
    /// for the shell to run.
    pub follow_up: Vec<FollowUp>,
}

/// Remove what [`super::install`] wrote, as `plan` describes it, under
/// `root`.
///
/// The inverse of [`super::install`], with the same shape and the same
/// guarantees: [`Consent`] is checked before anything else, every path is
/// checked before anything is removed, and nothing outside `root` can be
/// reached. Each file is read again immediately before it is removed, and
/// removed only if it still carries [`desktop::PER_USER_MARKER`] -- so a
/// hand-built plan naming another application's entry removes nothing, and
/// a file the user replaced since the plan was shown is left alone. A file
/// that is already gone is not an error.
///
/// # Errors
///
/// [`InstallRefusal::ConsentWithheld`], [`InstallRefusal::NotSupported`],
/// [`InstallRefusal::UnsafeArtefactPath`], or [`InstallRefusal::Failed`] when
/// the filesystem refuses. As with install, a failure part-way through leaves
/// what was already removed removed.
pub fn uninstall(
    plan: &Unregistration,
    root: &Path,
    consent: Consent,
) -> Result<Uninstalled, InstallRefusal> {
    if consent != Consent::Granted {
        return Err(InstallRefusal::ConsentWithheld);
    }
    refuse_unsupported(Platform::Linux)?;
    for relative_path in &plan.remove {
        check_relative_path(relative_path)?;
    }

    let mut removed = Vec::new();
    let mut skipped = Vec::new();
    let mut desktop_removed = false;
    let mut mime_removed = false;
    for relative_path in &plan.remove {
        let target = under_root(root, relative_path);
        match fs::symlink_metadata(&target) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                return Err(InstallRefusal::Failed {
                    path: target,
                    message: error.to_string(),
                });
            }
            Ok(_) => {}
        }
        if let Some(why) = per_user_obstacle(&target) {
            skipped.push(Skipped { path: target, why });
            continue;
        }
        fs::remove_file(&target).map_err(|error| InstallRefusal::Failed {
            path: target.clone(),
            message: error.to_string(),
        })?;
        desktop_removed |= relative_path.starts_with("applications/");
        mime_removed |= relative_path.starts_with("mime/");
        removed.push(target);
    }

    let root_text = root.to_string_lossy();
    let mut follow_up = Vec::new();
    if desktop_removed {
        follow_up.push(FollowUp {
            program: UPDATE_DESKTOP_DATABASE,
            args: vec![join(Platform::Linux, &root_text, &["applications"])],
            why: WHY_DESKTOP_DATABASE_AFTER_REMOVAL,
        });
    }
    if mime_removed {
        follow_up.push(FollowUp {
            program: UPDATE_MIME_DATABASE,
            args: vec![join(Platform::Linux, &root_text, &["mime"])],
            why: WHY_MIME_DATABASE_AFTER_REMOVAL,
        });
    }
    Ok(Uninstalled {
        removed,
        skipped,
        follow_up,
    })
}

/// [`InstallRefusal::NotSupported`] with [`FLATPAK_REASON`] inside a Flatpak.
fn refuse_in_flatpak(env: &DesktopEnv) -> Result<(), InstallRefusal> {
    match set_as_default_availability(Platform::Linux, env) {
        Availability::Unsupported { reason } => Err(InstallRefusal::NotSupported {
            platform: Platform::Linux,
            reason,
        }),
        _ => Ok(()),
    }
}

/// The two paths a per-user registration can write, relative to the data
/// home.
fn registration_paths(app: &AppInfo) -> [String; 2] {
    [
        format!("applications/{}", desktop::desktop_file_name(app)),
        format!("mime/packages/{}", desktop::mime_package_file_name(app)),
    ]
}

/// Why an existing file at a per-user registration path must not be
/// overwritten or removed, or `None` when it is this product's own (or does
/// not exist).
///
/// Looks at the link itself rather than through it, so a symbolic link
/// planted at the path is refused rather than followed.
fn per_user_obstacle(path: &Path) -> Option<&'static str> {
    let metadata = fs::symlink_metadata(path).ok()?;
    if !metadata.file_type().is_file() {
        return Some(SKIP_NOT_A_FILE);
    }
    match fs::read_to_string(path) {
        Ok(contents) if is_per_user_registration(&contents) => None,
        _ => Some(SKIP_NOT_OURS),
    }
}

/// Whether the `.desktop` entry at `path` claims every MIME type in
/// `selection`.
///
/// A system entry that claims less than the user asked for is not a
/// substitute: skipping the per-user copy would quietly register fewer types
/// than the dialog said, so a per-user copy is written instead and shadows
/// it.
fn desktop_entry_covers(path: &Path, selection: &AssociationSelection) -> bool {
    let Ok(text) = fs::read_to_string(path) else {
        return false;
    };
    let mut in_entry = false;
    let mut claimed: Vec<String> = Vec::new();
    for line in text.lines().map(str::trim) {
        if let Some(group) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            in_entry = group == "Desktop Entry";
            continue;
        }
        if !in_entry {
            continue;
        }
        if let Some((key, value)) = line.split_once('=')
            && key.trim() == "MimeType"
        {
            claimed = value
                .split(';')
                .map(str::trim)
                .filter(|m| !m.is_empty())
                .map(str::to_owned)
                .collect();
            break;
        }
    }
    selection
        .mime_types()
        .iter()
        .all(|mime| claimed.iter().any(|c| c == mime))
}

/// Whether the MIME package at `path` defines every type the per-user
/// package for `selection` would.
fn mime_package_covers(path: &Path, selection: &AssociationSelection) -> bool {
    let Ok(text) = fs::read_to_string(path) else {
        return false;
    };
    selection
        .file_types()
        .filter(|t| t.is_own_mime_type())
        .all(|t| text.contains(&format!("type=\"{}\"", desktop::escape_xml(t.mime))))
}

#[cfg(test)]
mod tests;
