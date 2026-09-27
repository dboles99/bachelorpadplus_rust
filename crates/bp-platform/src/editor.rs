//! Default-editor registration, under ADR-0012.
//!
//! ADR-0012 accepts one way of becoming the user's text editor and refuses
//! another. Accepted: register as a supported handler for the types the
//! product understands, and guide the user to the operating system's own
//! default-app selection. Refused: replacing, patching, shimming or
//! redirecting `notepad.exe`; and, by extension, any mechanism that decides on
//! the user's behalf what opens their files.
//!
//! ## The line between asking and doing
//!
//! Several things happen in this module and only two of them can change the
//! user's machine.
//!
//! | | needs consent | available |
//! |---|---|---|
//! | [`state`], [`association_report_in`] -- what opens `.txt` today | no | Linux; Windows through a [`ProgIdQuery`] the shell supplies |
//! | [`plan`], [`linux_plan`], [`unregistration_plan`] -- what a change would consist of | no | both / Linux |
//! | [`install`] -- putting the artefacts where the desktop reads them | **yes** | Linux, outside Flatpak |
//! | [`uninstall`] -- removing exactly what [`install`] wrote | **yes** | Linux, outside Flatpak |
//!
//! Reporting and generating read at most the files the desktop itself reads
//! (the `mimeapps.list` chain, the system `.desktop` entry), write nothing,
//! and are safe to call from a settings screen that is merely being drawn.
//! [`install`] and [`uninstall`] are the two functions that change anything,
//! and each takes two things nothing else does: an explicit [`Consent`]
//! value, and an explicit root. A caller cannot reach them by accident and a
//! test cannot reach the user's real directories at all.
//!
//! Neither of them sets the default, and nothing in this crate runs a
//! program. [`LinuxPlan::commands`] names the exact command lines that
//! finish the job -- rebuilding the two databases, and `xdg-mime default` --
//! as data. The shell runs them, and only after the user has seen those
//! exact lines and agreed: the user choosing "Set as Default" in a dialog that
//! shows the command is the user's own choice, which is what ADR-0012 asks
//! for; a library that spawned it on its own would be deciding for them.
//!
//! ## Windows: generated here, applied by the installer or the user
//!
//! [`plan`] on Windows produces the complete `HKCU\Software\Classes` shape and
//! renders it as a pair of `.reg` scripts -- one to register, one to take
//! exactly that away again -- and [`install`] refuses. By decision rather
//! than by limitation (ADR-0032): the per-user installer writes these values
//! and removes them, the application never writes the registry, and a
//! portable build offers the pair through a Save dialog. A file the user
//! chose where to put and can read before running is a defensible answer,
//! and arguably a better one than an installer that writes the same keys
//! invisibly. The installer's `[Registry]` section is rendered from the same
//! values by [`inno`], so the two cannot disagree.
//! [`windows::registration_route`] tells the shell which of
//! "Open Windows Settings" and "Save registration file…" to lead with.
//!
//! Every generated key is checked against [`windows::key_objection`], which is
//! ADR-0012 written as a predicate rather than as a paragraph.

pub mod desktop;
pub mod file_type;
pub mod inno;
pub mod linux;
pub mod state;
pub mod windows;

use std::fs;
use std::path::{Path, PathBuf};

use crate::capabilities::{Availability, Capability};
use crate::paths::{self, is_absolute};
use crate::{DirKind, EnvSnapshot, Platform};

pub use file_type::{
    AssociationPreset, AssociationSelection, FILE_TYPES, FileType, TypeGroup, file_type,
    unknown_extensions,
};
pub use linux::{
    DesktopEnv, FLATPAK_REASON, LinuxPlan, Uninstalled, Unregistration, association_report_in,
    linux_plan, set_as_default_availability, uninstall, unregistration_plan,
};
pub use state::{AssociationEntry, AssociationReport, AssociationState, ProgIdQuery};
pub use windows::{KeyObjection, RegistrationRoute, RegistryValue, ValueName};

/// What the product is, from the point of view of a registration.
///
/// A value the caller fills in rather than a set of constants, for one reason
/// that is not negotiable: `executable` is the path to the running binary, and
/// only the running process knows it. A hard-coded `/usr/bin/bachelorpad` is a
/// registration that launches the wrong program for anyone who installed to
/// `~/.local/bin`, ran a portable build, or is a developer with a `target/`
/// directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppInfo {
    /// Shown in Open With and in Default Apps.
    pub display_name: String,
    /// One line, shown beside the name.
    pub description: String,
    /// Absolute path to the executable to launch.
    pub executable: String,
    /// The identifier that names the `.desktop` file, and therefore the
    /// product's identity in every `mimeapps.list` on the machine. Changing it
    /// orphans every association the user has already made, so it is a value
    /// with a compatibility promise attached rather than a label.
    pub app_id: String,
    /// The icon name (Linux) -- on Windows the icon comes from the executable.
    pub icon: String,
}

impl AppInfo {
    /// This product, launched from `executable`.
    ///
    /// Everything but the path is fixed, because everything but the path is a
    /// promise: the `app_id` in particular must not drift between releases.
    /// The icon shares the id, because that is the name the hicolor theme
    /// files are installed under and the name a desktop looks up.
    #[must_use]
    pub fn bachelorpad(executable: impl Into<String>) -> Self {
        Self {
            display_name: "BachelorPad+".to_owned(),
            description: "Text editor for notes, data and logs".to_owned(),
            executable: executable.into(),
            app_id: crate::APP_ID.to_owned(),
            icon: crate::APP_ID.to_owned(),
        }
    }

    /// This product as a distribution package installs it: `bachelorpad` on
    /// `PATH`.
    ///
    /// The one place a bare program name is right. A package's `.desktop`
    /// entry is written before the package knows where it will be installed
    /// (`/usr/bin`, `/usr/local/bin`, a Nix store path), and `Exec=bachelorpad
    /// %F` is what the specification and every packaged editor use: the
    /// desktop resolves it through `PATH` exactly as a terminal would.
    /// [`Self::problems`] still reports it as relative, on purpose -- a
    /// *per-user* registration naming a bare program launches whatever
    /// `PATH` finds first, which for a developer is not this build.
    #[must_use]
    pub fn packaged() -> Self {
        Self::bachelorpad("bachelorpad")
    }

    /// This product, as the running process should register it.
    ///
    /// `current_exe` is what the process reports about itself, and inside an
    /// AppImage that is a path under a temporary FUSE mount
    /// (`/tmp/.mount_BachelXXXX/usr/bin/bachelorpad`) that disappears when
    /// the program exits -- a `.desktop` entry naming it launches nothing
    /// tomorrow. The AppImage runtime puts the path of the `.AppImage` file
    /// itself in `$APPIMAGE`, and that is the file the user keeps, so it wins
    /// when it is set and absolute.
    #[must_use]
    pub fn launched_from(current_exe: impl Into<String>, env: &DesktopEnv) -> Self {
        match env.appimage_path() {
            Some(appimage) => Self::bachelorpad(appimage),
            None => Self::bachelorpad(current_exe),
        }
    }

    /// What is wrong with this description of the application, if anything.
    ///
    /// Worth calling before a plan is built rather than after it is installed:
    /// a relative `executable` produces a `.desktop` entry that launches
    /// nothing, and the failure surfaces as "double-clicking a text file does
    /// nothing", which is about as far from its cause as a defect gets.
    #[must_use]
    pub fn problems(&self, platform: Platform) -> Vec<String> {
        let mut problems = Vec::new();
        if self.executable.trim().is_empty() {
            problems.push("The path to the executable is empty.".to_owned());
        } else if !is_absolute(platform, &self.executable) {
            problems.push(format!(
                "The executable path \"{}\" is relative; a registration must name an absolute path.",
                self.executable
            ));
        }
        if self.app_id.trim().is_empty() {
            problems.push("The application id is empty.".to_owned());
        } else if !paths::is_portable_file_name(&self.app_id) {
            problems.push(format!(
                "The application id \"{}\" cannot be used as a file name.",
                self.app_id
            ));
        }
        if self.display_name.trim().is_empty() {
            problems.push("The display name is empty.".to_owned());
        }
        problems
    }
}

/// The program that rebuilds `mimeinfo.cache` for an `applications`
/// directory.
const UPDATE_DESKTOP_DATABASE: &str = "update-desktop-database";

/// The program that compiles `mime/packages/*.xml` into the MIME database.
const UPDATE_MIME_DATABASE: &str = "update-mime-database";

/// Why [`UPDATE_DESKTOP_DATABASE`] has to run after an install.
const WHY_DESKTOP_DATABASE: &str =
    "Until the desktop database is rebuilt, the new entry does not appear in Open With.";

/// Why [`UPDATE_MIME_DATABASE`] has to run after an install.
const WHY_MIME_DATABASE: &str = "Until the MIME database is rebuilt, the product's own file \
                                 types have no MIME type and no association can match them.";

/// What kind of artefact a generated file is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtefactKind {
    /// A freedesktop `.desktop` entry.
    DesktopEntry,
    /// A `shared-mime-info` package defining the product's own types.
    MimePackage,
    /// A `.reg` script that registers. Generated, never applied by the
    /// application -- see the module docs. UTF-16LE on disk.
    RegistryScript,
    /// The `.reg` script that removes exactly what [`Self::RegistryScript`]
    /// adds, and nothing belonging to anybody else. UTF-16LE on disk.
    UnregisterScript,
}

/// A file a registration would consist of.
///
/// The path is *relative*, always, and that is a safety property rather than
/// a convenience: an artefact carrying an absolute path is one that can be
/// written anywhere, and a plan is a value that travels. [`install`] re-checks
/// it before writing regardless.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Artefact {
    /// Where it goes, relative to the install root.
    pub relative_path: String,
    /// The whole file, as text: what a dialog shows and a test reads. What
    /// goes to disk is [`Self::bytes`], which is not always this text's UTF-8.
    pub contents: String,
    /// What it is.
    pub kind: ArtefactKind,
}

impl Artefact {
    /// The file as it must be written to disk.
    ///
    /// UTF-8 for the Linux artefacts, whose formats are defined as UTF-8.
    /// UTF-16LE with a byte-order mark for the `.reg` scripts, which is what
    /// `regedit` exports and the only encoding it reads a non-ASCII path from
    /// without going through the ANSI code page (see
    /// [`windows::utf16le_with_bom`]). Anything that saves an artefact should
    /// write these bytes, never [`Self::contents`] directly.
    #[must_use]
    pub fn bytes(&self) -> Vec<u8> {
        match self.kind {
            ArtefactKind::DesktopEntry | ArtefactKind::MimePackage => {
                self.contents.as_bytes().to_vec()
            }
            ArtefactKind::RegistryScript | ArtefactKind::UnregisterScript => {
                windows::utf16le_with_bom(&self.contents)
            }
        }
    }
}

/// A command that must run before the registration takes effect, named rather
/// than run.
///
/// Nothing in this crate spawns a process. A library that runs commands on the
/// user's behalf is a library that can be talked into running a different one,
/// and the value of these is mostly that a user can see them: "the association
/// will not appear until `update-desktop-database` has run" is the single most
/// common reason a correct `.desktop` file appears to do nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FollowUp {
    /// The program, by name -- resolved through `PATH` by whoever runs it.
    pub program: &'static str,
    /// Its arguments.
    pub args: Vec<String>,
    /// What goes wrong if it is skipped.
    pub why: &'static str,
}

/// How one [`FollowUp`] ended when the shell ran it.
///
/// Filled in by the shell from `std::process::Command::output()`, and handed
/// back to [`FollowUp::report`] so the sentence the user reads is written
/// here, next to the reason the command exists, rather than improvised at the
/// call site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandOutcome {
    /// Exit status zero.
    Succeeded,
    /// It ran and failed. `exit_code` is `None` when a signal ended it;
    /// `stderr` is whatever it printed, which the report trims to one line.
    Failed {
        exit_code: Option<i32>,
        stderr: String,
    },
    /// The program is not installed (`io::ErrorKind::NotFound`). Worth its own
    /// variant because it is the common case -- `update-desktop-database` is
    /// in `desktop-file-utils`, which a minimal install may lack -- and
    /// because the fix is a package, not a retry.
    NotFound,
    /// It could not be started for any other reason.
    CouldNotStart { message: String },
}

impl FollowUp {
    /// One line saying how this command ended, for the dialog that follows
    /// the change.
    ///
    /// Failure is worded as a consequence, using [`Self::why`], because "exit
    /// status 2" means nothing to the person who clicked "Set as Default"
    /// and "the new entry does not appear in Open With" does.
    #[must_use]
    pub fn report(&self, outcome: &CommandOutcome) -> String {
        let line = self.command_line();
        match outcome {
            CommandOutcome::Succeeded => format!("Ran {line}"),
            CommandOutcome::Failed { exit_code, stderr } => {
                let status = exit_code.map_or_else(
                    || "was stopped by a signal".to_owned(),
                    |code| format!("failed with exit status {code}"),
                );
                let first = stderr.lines().map(str::trim).find(|l| !l.is_empty());
                match first {
                    Some(detail) => format!("{line} {status} ({detail}). {}", self.why),
                    None => format!("{line} {status}. {}", self.why),
                }
            }
            CommandOutcome::NotFound => format!(
                "{} is not installed, so this step was skipped. {}",
                self.program, self.why
            ),
            CommandOutcome::CouldNotStart { message } => {
                format!("Could not run {line} ({message}). {}", self.why)
            }
        }
    }

    /// The command as a user would type it, for display and for a log.
    ///
    /// Not for execution: arguments are joined with spaces and a path
    /// containing one would be ambiguous. Anything running these should use
    /// [`Self::program`] and [`Self::args`] as the separate values they are.
    #[must_use]
    pub fn command_line(&self) -> String {
        std::iter::once(self.program.to_owned())
            .chain(self.args.iter().cloned())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// The step that ends with the user, rather than with us.
///
/// ADR-0012's "guide the user through the OS's own default-app selection", as
/// a value the shell can act on. Neither variant is executed here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Handoff {
    /// Open the Windows Settings page. `ms-settings:` is the supported way to
    /// reach it on Windows 10 and 11; the older `control.exe /name
    /// Microsoft.DefaultPrograms` still works but lands on a page Windows 11
    /// redirects away from.
    WindowsDefaultApps { uri: &'static str },
    /// Run `xdg-mime default`, the standards-based way to set an association
    /// on Linux. It *is* the choice, so nothing in this crate runs it: the
    /// shell runs it (as [`LinuxPlan::commands`] lists it) only once the user
    /// has seen this exact command and said yes, and this variant is the
    /// same command for a user to run by hand.
    LinuxSetDefault {
        program: &'static str,
        args: Vec<String>,
    },
}

impl Handoff {
    /// A sentence telling the user what happens next.
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Self::WindowsDefaultApps { .. } => {
                "BachelorPad+ now appears in Windows' list of applications. Choose it in \
                 Settings ▸ Apps ▸ Default apps to make it open these file types."
                    .to_owned()
            }
            Self::LinuxSetDefault { program, args } => format!(
                "BachelorPad+ now appears in Open With. To make it the default, run: {program} {}",
                args.join(" ")
            ),
        }
    }
}

/// Everything a registration would do, as a value, before anything is done.
///
/// Building one is free of consequence, which is the point: a settings screen
/// can show the user the exact `.desktop` file and the exact registry values
/// before asking whether to proceed. Nothing else in this crate needs
/// permission, and this is the value the one thing that does operates on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistrationPlan {
    /// Which platform it was built for.
    pub platform: Platform,
    /// The file types it claims.
    pub selection: AssociationSelection,
    /// The files it would write, with relative paths.
    pub artefacts: Vec<Artefact>,
    /// The registry values it would set. Always empty on Linux.
    pub registry_values: Vec<RegistryValue>,
    /// Commands that must run afterwards for it to take effect.
    pub follow_up: Vec<FollowUp>,
    /// Where the user finishes the job.
    pub handoff: Handoff,
}

impl RegistrationPlan {
    /// Any generated registry key this crate refuses to write.
    ///
    /// Always empty for a plan this crate built -- a test asserts it over
    /// every preset on both platforms. It is public so that whatever
    /// eventually *applies* a plan checks the plan it was handed rather than
    /// trusting where it came from.
    #[must_use]
    pub fn registry_objections(&self) -> Vec<(String, KeyObjection)> {
        windows::objections(&self.registry_values)
    }

    /// The artefact of one kind, if the plan has one.
    #[must_use]
    pub fn artefact(&self, kind: ArtefactKind) -> Option<&Artefact> {
        self.artefacts.iter().find(|a| a.kind == kind)
    }
}

/// Everything a registration for `selection` would consist of, on `platform`.
///
/// Pure: no environment, no filesystem, no clock. `platform` is a parameter
/// rather than a `cfg`, so the Windows plan is built and asserted by the Linux
/// leg of CI and the Linux plan by the Windows leg -- which is the only way
/// the `.reg` shape and the ADR-0012 guard get tested at all on a machine that
/// has no registry.
#[must_use]
pub fn plan(
    platform: Platform,
    app: &AppInfo,
    selection: &AssociationSelection,
) -> RegistrationPlan {
    let mut artefacts = Vec::new();
    let mut registry_values = Vec::new();
    let mut follow_up = Vec::new();

    let handoff = match platform {
        Platform::Linux => {
            artefacts.push(Artefact {
                relative_path: format!("applications/{}", desktop::desktop_file_name(app)),
                contents: desktop::per_user_desktop_entry(app, selection),
                kind: ArtefactKind::DesktopEntry,
            });
            follow_up.push(FollowUp {
                program: UPDATE_DESKTOP_DATABASE,
                args: vec!["applications".to_owned()],
                why: WHY_DESKTOP_DATABASE,
            });
            if let Some(package) = desktop::per_user_mime_package(selection) {
                artefacts.push(Artefact {
                    relative_path: format!(
                        "mime/packages/{}",
                        desktop::mime_package_file_name(app)
                    ),
                    contents: package,
                    kind: ArtefactKind::MimePackage,
                });
                follow_up.push(FollowUp {
                    program: UPDATE_MIME_DATABASE,
                    args: vec!["mime".to_owned()],
                    why: WHY_MIME_DATABASE,
                });
            }
            Handoff::LinuxSetDefault {
                program: "xdg-mime",
                args: std::iter::once("default".to_owned())
                    .chain(std::iter::once(desktop::desktop_file_name(app)))
                    .chain(selection.mime_types().into_iter().map(str::to_owned))
                    .collect(),
            }
        }
        Platform::Windows => {
            registry_values = windows::registry_values(app, selection);
            // Both named after the directory, not the application id: the id is a
            // Linux identity (ADR-0032), and the file a Windows user saves
            // should keep the name it has always had.
            artefacts.push(Artefact {
                relative_path: format!("{}-file-types.reg", crate::APP_DIR),
                contents: windows::registry_script(&registry_values),
                kind: ArtefactKind::RegistryScript,
            });
            artefacts.push(Artefact {
                relative_path: format!("{}-file-types-remove.reg", crate::APP_DIR),
                contents: windows::unregister_script(&registry_values),
                kind: ArtefactKind::UnregisterScript,
            });
            // The generic page, because `plan` is pure and the build number
            // is a fact about the machine. The shell that knows the build
            // asks `windows::default_apps_uri` for the Windows 11 deep link.
            Handoff::WindowsDefaultApps {
                uri: windows::DEFAULT_APPS_URI,
            }
        }
    };

    RegistrationPlan {
        platform,
        selection: selection.clone(),
        artefacts,
        registry_values,
        follow_up,
        handoff,
    }
}

/// Whether the caller has been told what will happen and said yes.
///
/// A two-variant enum rather than a `bool`, because `install(&plan, root,
/// true)` at a call site says nothing about what the `true` means, and the one
/// function in this crate that changes a machine is the one where a reader
/// should not have to check the signature. There is deliberately no `Default`:
/// consent that arrives by default is not consent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Consent {
    /// The user has seen what would be written and agreed to it.
    Granted,
    /// They have not. [`install`] refuses.
    Withheld,
}

/// What an install actually did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Installed {
    /// The files written, in the order they were written, as absolute paths
    /// under the root that was given.
    pub written: Vec<PathBuf>,
    /// What still has to run. Carried forward from the plan so the caller does
    /// not have to hold both.
    pub follow_up: Vec<FollowUp>,
}

/// Why an install did not happen.
///
/// Refusals, not errors, for everything except [`Self::Failed`]: they are
/// answers the caller can act on rather than things that went wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallRefusal {
    /// [`Consent::Withheld`]. The first thing checked, before the plan is even
    /// examined, so that a refused install cannot fail for some other reason
    /// and hide the fact that nobody agreed to it.
    ConsentWithheld,
    /// The plan claims no file types. Writing it would be a confusing no-op.
    NothingSelected,
    /// This platform cannot install through this crate. The reason comes from
    /// the capability register, so the refusal and the greyed-out menu item say
    /// the same thing.
    NotSupported {
        platform: Platform,
        reason: &'static str,
    },
    /// An artefact's relative path is not one this crate will write: absolute,
    /// empty, or containing `..`. Impossible for a plan built by [`plan`] and
    /// checked anyway, because a plan is a value and values arrive from
    /// elsewhere.
    UnsafeArtefactPath { relative_path: String },
    /// The environment does not say where the user's data directory is
    /// (`$XDG_DATA_HOME` and `$HOME` are both unusable), so there is nowhere
    /// a per-user registration could go. Refused rather than guessed, like
    /// every other directory in this crate.
    NoDataDirectory,
    /// The filesystem said no.
    Failed { path: PathBuf, message: String },
}

impl InstallRefusal {
    /// A sentence for the user.
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Self::ConsentWithheld => {
                "Nothing was changed: registration was not confirmed.".to_owned()
            }
            Self::NothingSelected => "Nothing was changed: no file types were selected.".to_owned(),
            Self::NotSupported { platform, reason } => {
                format!(
                    "Registration cannot be applied on {}. {reason}",
                    platform.token()
                )
            }
            Self::NoDataDirectory => "Nothing was changed: this environment does not say where \
                                      the user's data directory is ($XDG_DATA_HOME or $HOME)."
                .to_owned(),
            Self::UnsafeArtefactPath { relative_path } => format!(
                "Nothing was changed: \"{relative_path}\" is not a path this application will \
                 write to."
            ),
            Self::Failed { path, message } => {
                format!("Could not write {}: {message}", path.display())
            }
        }
    }
}

/// Write a plan's artefacts under `root`.
///
/// **One of the two functions in this crate that change anything** -- the
/// other is [`uninstall`], its inverse. It writes the `.desktop` entry and
/// the MIME package into the places the desktop environment reads, which is
/// what makes the product appear in Open With. It does not set any default,
/// does not run any command, and does not touch the registry.
///
/// A plan whose artefacts are all provided by the system (see [`linux_plan`])
/// writes nothing and succeeds: the registration is already there.
///
/// `root` is a parameter with no default, and that is the mechanism by which
/// no test can reach a real user directory: a test passes a `tempfile`
/// directory, the shell passes [`install_root`]. There is no code path that
/// picks one for you.
///
/// # Errors
///
/// Returns [`InstallRefusal`] rather than writing when consent is withheld,
/// the selection is empty, the platform has no supported install (Windows --
/// see the module docs), an artefact path is not one this crate will write, or
/// the filesystem refuses. Nothing partial is cleaned up: a failure part-way
/// through leaves the files already written, because deleting files during
/// error handling in a directory the user named is a worse risk than an
/// incomplete registration that the next attempt overwrites.
pub fn install(
    plan: &RegistrationPlan,
    root: &Path,
    consent: Consent,
) -> Result<Installed, InstallRefusal> {
    if consent != Consent::Granted {
        return Err(InstallRefusal::ConsentWithheld);
    }
    if plan.selection.is_empty() {
        return Err(InstallRefusal::NothingSelected);
    }
    refuse_unsupported(plan.platform)?;

    // Every path is checked before anything is written, so a plan with one bad
    // artefact writes none of them rather than half of them.
    for artefact in &plan.artefacts {
        check_relative_path(&artefact.relative_path)?;
    }

    let mut written = Vec::new();
    for artefact in &plan.artefacts {
        let target = under_root(root, &artefact.relative_path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|error| InstallRefusal::Failed {
                path: parent.to_path_buf(),
                message: error.to_string(),
            })?;
        }
        fs::write(&target, artefact.bytes()).map_err(|error| InstallRefusal::Failed {
            path: target.clone(),
            message: error.to_string(),
        })?;
        written.push(target);
    }

    Ok(Installed {
        written,
        follow_up: plan.follow_up.clone(),
    })
}

/// The directory [`install`] should be given on this platform.
///
/// `$XDG_DATA_HOME` on Linux -- the per-user half of the shared data
/// hierarchy, which is where a `.desktop` entry belongs and is why nothing
/// here ever needs to write outside the user's own home. `None` on Windows,
/// which has no supported install through this crate at all.
///
/// Takes the environment as a value like everything else in this crate, so a
/// test can ask what the answer *would* be for an environment it makes up.
#[must_use]
pub fn install_root(platform: Platform, env: &EnvSnapshot) -> Option<PathBuf> {
    match platform {
        Platform::Linux => crate::dirs::root(platform, DirKind::Data, env),
        Platform::Windows => None,
    }
}

/// What currently opens the selected file types, read from `config_dir`.
///
/// The narrow form: it reads one `mimeapps.list`, which is where `xdg-mime
/// default` writes and so answers "did our change land". What a double-click
/// actually opens is decided by the whole XDG chain -- desktop-specific
/// lists, `/etc/xdg`, the system data directories and `mimeinfo.cache` --
/// and [`association_report_in`] follows it. Prefer that one for anything a
/// user reads.
///
/// The read half of registration, and the one that needs no permission. On
/// Linux `config_dir` is `$XDG_CONFIG_HOME` -- the directory holding
/// `mimeapps.list` -- and a missing file is not an error: a user who has never
/// chosen a handler has none recorded, which is
/// [`AssociationState::Unclaimed`], not a failure.
///
/// On Windows it returns [`AssociationState::Unknown`] for everything, with
/// the capability register's own reason attached. This crate reads no
/// registry; the Windows answer comes from [`state::report_from_query`] given
/// the [`ProgIdQuery`] `bp-platform-windows` implements, which only the shell
/// can hand over. Reporting "unclaimed" here instead would tell the user their
/// machine is unconfigured when it is merely unreadable from here.
#[must_use]
pub fn association_report(
    platform: Platform,
    config_dir: Option<&Path>,
    app: &AppInfo,
    selection: &AssociationSelection,
) -> AssociationReport {
    match platform {
        Platform::Linux => {
            let text = config_dir
                .map(|dir| dir.join("mimeapps.list"))
                .and_then(|path| fs::read_to_string(path).ok())
                .unwrap_or_default();
            state::report_from_mimeapps(&text, app, selection)
        }
        Platform::Windows => state::unknown_report(
            Capability::AssociationQuery
                .availability(platform)
                .reason()
                .unwrap_or(
                    "The current handler is read from the registry, and no registry reader was \
                     supplied here.",
                ),
            selection,
        ),
    }
}

/// The refusal the capability register implies for changing registration on
/// `platform`, if any.
///
/// [`Availability::Conditional`] passes: on Linux the condition is "not in a
/// Flatpak sandbox", which only the environment can answer, and
/// [`linux_plan`] -- the function that has the environment -- refuses there
/// with [`FLATPAK_REASON`] before a plan exists to be installed.
fn refuse_unsupported(platform: Platform) -> Result<(), InstallRefusal> {
    match Capability::AssociationInstall.availability(platform) {
        Availability::NotImplemented { reason } | Availability::Unsupported { reason } => {
            Err(InstallRefusal::NotSupported { platform, reason })
        }
        _ => Ok(()),
    }
}

/// [`InstallRefusal::UnsafeArtefactPath`] unless [`is_safe_relative_path`].
fn check_relative_path(relative_path: &str) -> Result<(), InstallRefusal> {
    if is_safe_relative_path(relative_path) {
        Ok(())
    } else {
        Err(InstallRefusal::UnsafeArtefactPath {
            relative_path: relative_path.to_owned(),
        })
    }
}

/// `relative_path` (always `/`-separated) under `root`, one component at a
/// time so the host's own separator is used.
fn under_root(root: &Path, relative_path: &str) -> PathBuf {
    relative_path
        .split('/')
        .fold(root.to_path_buf(), |acc, part| acc.join(part))
}

/// Whether this crate will write to `relative_path` under a given root.
///
/// The rules are the boring ones and they are all load-bearing: not empty, not
/// absolute on *either* platform (a plan built for one may be inspected on the
/// other), no `.` or `..` component, and every component a legal file name on
/// both. Together they mean the written path is under the root the caller
/// named and nowhere else.
fn is_safe_relative_path(relative_path: &str) -> bool {
    if relative_path.trim().is_empty() {
        return false;
    }
    if Platform::ALL
        .iter()
        .any(|&platform| is_absolute(platform, relative_path))
    {
        return false;
    }
    // Split on both separators whatever the platform, so a backslash cannot
    // hide a `..` from a check done with forward slashes.
    let components: Vec<&str> = relative_path
        .split(['/', '\\'])
        .filter(|part| !part.is_empty())
        .collect();
    !components.is_empty()
        && components
            .iter()
            .all(|part| paths::is_portable_file_name(part))
}

#[cfg(test)]
mod tests;
