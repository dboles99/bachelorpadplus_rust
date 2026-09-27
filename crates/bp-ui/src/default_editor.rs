//! File ▸ Set as Default Editor and File ▸ Remove Registration: the shell's
//! half of ADR-0012, ADR-0032 and release plan M2-04/M2-06.
//!
//! Every decision here belongs to `bp-platform`. Which file types may be
//! claimed, what a `.desktop` entry and a `.reg` script contain, which
//! registry keys this product refuses to write, which files this machine
//! already has, what a removal may touch, and the exact command lines that
//! finish a Linux registration -- all of it is answered there, by functions
//! that take the platform and the environment as values and are therefore
//! tested by both legs of CI.
//!
//! What is left for the shell is the part that is genuinely the shell's:
//! reading what opens these file types **today** and showing it before
//! offering to change anything, putting the whole plan in front of the user,
//! turning their answer into the one [`Consent`] value `install` and
//! `uninstall` accept, and -- only after that answer -- doing the one thing
//! a library should not: running programs.
//!
//! * **It picks no destination root of its own.** The root comes from
//!   `bp-platform` (`$XDG_DATA_HOME`) and is named in the consent dialog,
//!   because a root chosen out of sight is a file written somewhere nobody
//!   agreed to.
//! * **It runs exactly the commands the dialog showed.** On Linux the
//!   database rebuilds and `xdg-mime default` are listed, line for line, in
//!   the dialog whose button says "Set as Default"; confirming it is the
//!   user choosing the default, which is ADR-0012's line. Every command runs
//!   even if one before it failed, and each result is reported.
//! * **It never writes the Windows registry.** ADR-0032: the installer does
//!   that, and a portable build saves the `.reg` pair for the user to read
//!   and run. Once registered, the dialog opens Windows' own Default apps
//!   page, because choosing a default there is Windows' step, not ours.
//!
//! [`Consent`]: bp_platform::editor::Consent

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use bp_platform::editor::desktop::PACKAGED_PRESET;
use bp_platform::editor::state::{ProgIdQuery, report_from_query};
use bp_platform::editor::windows::{
    default_apps_uri, registered_application_name, registration_route,
};
use bp_platform::editor::{
    AppInfo, ArtefactKind, AssociationReport, AssociationSelection, AssociationState,
    CommandOutcome, DesktopEnv, FollowUp, Handoff, InstallRefusal, Installed, RegistrationPlan,
    RegistrationRoute, Uninstalled, Unregistration, association_report_in, linux_plan,
    set_as_default_availability, unregistration_plan,
};
use bp_platform::{Capability, Platform};

/// What confirming the dialog does, which differs by platform and, on
/// Windows, by whether this product is registered yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Route {
    /// Linux: write the plan's files under `root` (possibly none), then run
    /// `commands` in order.
    Install {
        root: PathBuf,
        commands: Vec<FollowUp>,
    },
    /// Windows, not registered: save the `.reg` pair where the user chooses.
    SaveFiles,
    /// Windows, registered: open Default apps at `uri`.
    OpenSettings { uri: String },
}

impl Route {
    /// The confirm button's label: what pressing it will do, in its words.
    pub(crate) fn button(&self) -> &'static str {
        match self {
            Self::Install { .. } => "Set as Default",
            Self::SaveFiles => "Save Files...",
            Self::OpenSettings { .. } => "Open Settings",
        }
    }
}

/// Everything the user has to see before anything happens.
#[derive(Debug)]
pub(crate) struct Offer {
    /// What `install` would be handed, or what the `.reg` pair is rendered
    /// from. Built by `bp_platform`, never here.
    pub(crate) plan: RegistrationPlan,
    pub(crate) route: Route,
    /// The consent dialog: what opens these types today, then exactly what
    /// would change.
    pub(crate) body: String,
}

/// The Windows facts an offer depends on, read once, so a test can supply
/// them from Linux. Everything in it is `None` off Windows, where
/// `bp-platform-windows` answers "no" to every question.
pub(crate) struct WindowsFacts {
    /// The `RegisteredApplications` value naming this product, if any.
    pub(crate) registered: Option<String>,
    /// The Windows build number, which decides how specific the Settings
    /// link can be.
    pub(crate) build: Option<u32>,
    /// What opens each extension now, read from the registry.
    pub(crate) choices: Option<Box<dyn ProgIdQuery>>,
}

impl WindowsFacts {
    fn read(app: &AppInfo) -> Self {
        Self {
            registered: bp_platform_windows::registered_application(registered_application_name(
                app,
            )),
            build: bp_platform_windows::windows_build(),
            choices: bp_platform_windows::RegistryProgIds::new()
                .map(|query| Box::new(query) as Box<dyn ProgIdQuery>),
        }
    }

    /// Nothing known: what every question answers off Windows.
    #[cfg(test)]
    pub(crate) fn none() -> Self {
        Self {
            registered: None,
            build: None,
            choices: None,
        }
    }
}

/// This process, described for a registration.
///
/// The executable path is read from the running process because only the
/// running process knows it: a registration naming a hard-coded install
/// directory launches the wrong program for anybody who ran a portable
/// build. Inside an AppImage it is the AppImage file, not its temporary
/// mount, which `AppInfo::launched_from` decides.
fn this_app(env: &DesktopEnv) -> Result<AppInfo, String> {
    let executable = std::env::current_exe().map_err(|e| {
        format!(
            "cannot register file types — this application cannot find its own executable ({e})"
        )
    })?;
    Ok(AppInfo::launched_from(
        executable.display().to_string(),
        env,
    ))
}

/// Build the offer for this machine.
pub(crate) fn offer() -> Result<Offer, String> {
    let env = DesktopEnv::from_environment();
    let app = this_app(&env)?;
    let windows = WindowsFacts::read(&app);
    offer_for(Platform::HOST, &app, &env, &windows)
}

/// The offer for a named platform and a named environment.
///
/// Split from [`offer`] so a test can build the Windows offer while
/// standing on Linux and vice versa, and so no test ever points this at a
/// real user directory: the environment is a value, so `tempfile` roots
/// reach every path this function follows.
pub(crate) fn offer_for(
    platform: Platform,
    app: &AppInfo,
    env: &DesktopEnv,
    windows: &WindowsFacts,
) -> Result<Offer, String> {
    let problems = app.problems(platform);
    if !problems.is_empty() {
        return Err(format!(
            "cannot register file types — {}",
            problems.join(" ")
        ));
    }
    let selection = AssociationSelection::preset(PACKAGED_PRESET);

    match platform {
        Platform::Linux => {
            let linux = linux_plan(app, &selection, env).map_err(|r| r.describe())?;
            check(&linux.registration)?;
            let report = association_report_in(platform, env, app, &selection);
            let commands = linux.commands();
            let mut body = report_text(&report);
            body.push_str(&claimed_line(&linux.registration));
            body.push_str(&linux_steps(
                &linux.registration,
                &linux.root,
                &linux.skipped,
                &commands,
            ));
            Ok(Offer {
                plan: linux.registration,
                route: Route::Install {
                    root: linux.root,
                    commands,
                },
                body,
            })
        }
        Platform::Windows => {
            let plan = bp_platform::editor::plan(platform, app, &selection);
            check(&plan)?;
            // Read rather than assumed. Without a registry to ask, the
            // answer is "cannot tell", which is not the same fact as
            // "nothing is registered".
            let report = match &windows.choices {
                Some(query) => report_from_query(query.as_ref(), &selection),
                None => association_report_in(platform, env, app, &selection),
            };
            let route = match registration_route(windows.registered.as_deref()) {
                RegistrationRoute::OpenSettings => Route::OpenSettings {
                    uri: default_apps_uri(windows.build, app),
                },
                RegistrationRoute::SaveRegistrationFile => Route::SaveFiles,
            };
            let mut body = report_text(&report);
            body.push_str(&claimed_line(&plan));
            body.push_str(&windows_steps(&plan, &route));
            Ok(Offer { plan, route, body })
        }
    }
}

/// ADR-0012 as a check rather than as a paragraph. `bp-platform` will not
/// build a plan touching another application's keys and re-checks before it
/// writes; the shell checks too, because the Windows path hands a `.reg` to
/// a Save dialog and never goes through `install` at all.
fn check(plan: &RegistrationPlan) -> Result<(), String> {
    registry_objection(plan).map_or(Ok(()), Err)
}

/// What opens these types today: the summary, then one line per type.
///
/// When nothing could be read, every row carries the identical reason, and
/// seven copies of a four-line explanation is a dialog nobody finishes; the
/// reason is given once instead.
fn report_text(report: &AssociationReport) -> String {
    let mut out = report.summary();
    out.push_str("\n\n");
    let unreadable = report
        .entries()
        .iter()
        .filter_map(|entry| match &entry.state {
            AssociationState::Unknown { reason } => Some(*reason),
            _ => None,
        })
        .next()
        .filter(|_| {
            report
                .entries()
                .iter()
                .all(|entry| matches!(entry.state, AssociationState::Unknown { .. }))
        });
    match unreadable {
        Some(reason) => {
            out.push_str(reason);
            out.push('\n');
        }
        None => {
            for entry in report.entries() {
                out.push_str("  ");
                out.push_str(&entry.state.describe(entry.extension));
                out.push('\n');
            }
        }
    }
    out
}

fn claimed_line(plan: &RegistrationPlan) -> String {
    let claimed: Vec<String> = plan
        .selection
        .extensions()
        .map(|extension| format!(".{extension}"))
        .collect();
    format!(
        "\nRegistering claims {} file types: {}\n",
        claimed.len(),
        claimed.join(", ")
    )
}

/// The Linux half of the consent dialog: the files, the files left alone,
/// and every command, exactly as they will run.
fn linux_steps(
    plan: &RegistrationPlan,
    root: &Path,
    skipped: &[bp_platform::editor::linux::Skipped],
    commands: &[FollowUp],
) -> String {
    let mut out = String::new();
    if plan.artefacts.is_empty() {
        out.push_str("\nIt writes no files: this machine already has what it needs.\n");
    } else {
        out.push_str(&format!(
            "\nIt writes {} files under {}:\n",
            plan.artefacts.len(),
            root.display()
        ));
        for artefact in &plan.artefacts {
            out.push_str(&format!("  {}\n", artefact.relative_path));
        }
    }
    if !skipped.is_empty() {
        out.push_str("\nIt leaves these alone:\n");
        for skip in skipped {
            out.push_str(&format!("  {}\n      {}\n", skip.path.display(), skip.why));
        }
    }
    out.push_str("\nAfter you confirm, BachelorPad+ will run:\n");
    for step in commands {
        out.push_str(&format!("  {}\n      {}\n", step.command_line(), step.why));
    }
    out.push_str(
        "\nYour desktop's own default-application settings can change this back at any \
         time, and File ▸ Remove Registration takes away the files written here.\n",
    );
    out
}

/// The Windows half: which of the two routes, and what it involves.
fn windows_steps(plan: &RegistrationPlan, route: &Route) -> String {
    match route {
        Route::OpenSettings { uri } => format!(
            "\nBachelorPad+ is registered with Windows. Choosing it for these file types is \
             done in Settings ▸ Apps ▸ Default apps, which Windows keeps for itself; Open \
             Settings takes you there.\n\nSettings page: {uri}\n"
        ),
        // The register's own sentence first, not a second one written here:
        // a refusal and this dialog have to say the same thing about the
        // same obstacle or one of them is wrong.
        _ => format!(
            "\n{}\n\nSave Files... writes two registry scripts for you to read before you run \
             them. {} adds {} values, every one under HKEY_CURRENT_USER: it puts BachelorPad+ \
             in the Open With list and takes no file type away from any other application \
             (ADR-0012). {} takes exactly those values away again.\n\n{}\n",
            cannot_apply(Platform::Windows).describe(),
            registry_script_name(plan),
            plan.registry_values.len(),
            unregister_script_name(plan),
            handoff_note(&plan.handoff),
        ),
    }
}

/// The sentence and the address the user needs once a Windows registration
/// exists. The URI is printed so it can be pasted into Run.
pub(crate) fn handoff_note(handoff: &Handoff) -> String {
    let mut note = handoff.describe();
    if let Handoff::WindowsDefaultApps { uri } = handoff {
        note.push_str(&format!("\n\nSettings page: {uri}"));
    }
    note
}

/// Why this registration plan must not be written, if it must not.
///
/// `RegistrationPlan::registry_objections` is public precisely so that
/// whatever *applies* a plan checks the plan rather than trusting where it
/// came from. The Windows path here never goes through `install`, so this
/// is the only guard between a plan and a `.reg` file on the user's disk.
pub(crate) fn registry_objection(plan: &RegistrationPlan) -> Option<String> {
    plan.registry_objections().first().map(|(key, why)| {
        format!(
            "nothing was written — this registration would write {key}: {}",
            why.describe()
        )
    })
}

/// The refusal this platform's capability register produces for an install.
///
/// Built from [`Capability::AssociationInstall`] rather than written out, so
/// the dialog and whatever `install` itself returns are one string with one
/// place to change it.
pub(crate) fn cannot_apply(platform: Platform) -> InstallRefusal {
    InstallRefusal::NotSupported {
        platform,
        reason: Capability::AssociationInstall
            .availability(platform)
            .reason()
            .unwrap_or("This platform has no supported install."),
    }
}

/// Run one command the user agreed to, and say how it went.
///
/// Standard input is closed, so a program that unexpectedly asks a question
/// fails rather than waiting on a terminal nobody is looking at.
pub(crate) fn run_command(step: &FollowUp) -> CommandOutcome {
    run_command_on(step, None)
}

/// [`run_command`] with the `PATH` the program is looked up on, for tests:
/// the standard library searches the child's `PATH` when one is set, which
/// lets a test put a fake `xdg-mime` first without touching this process's
/// environment.
fn run_command_on(step: &FollowUp, path: Option<&OsStr>) -> CommandOutcome {
    let mut command = std::process::Command::new(step.program);
    command.args(&step.args).stdin(std::process::Stdio::null());
    if let Some(path) = path {
        command.env("PATH", path);
    }
    match command.output() {
        Ok(output) if output.status.success() => CommandOutcome::Succeeded,
        Ok(output) => CommandOutcome::Failed {
            exit_code: output.status.code(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => CommandOutcome::NotFound,
        Err(error) => CommandOutcome::CouldNotStart {
            message: error.to_string(),
        },
    }
}

/// Run every command in order, whatever the one before did.
///
/// The rebuilds refresh caches the desktop would refresh eventually anyway,
/// and a missing `update-desktop-database` is no reason not to record the
/// user's choice with `xdg-mime`.
pub(crate) fn run_all(
    commands: &[FollowUp],
    mut run: impl FnMut(&FollowUp) -> CommandOutcome,
) -> Vec<(FollowUp, CommandOutcome)> {
    commands
        .iter()
        .map(|step| (step.clone(), run(step)))
        .collect()
}

/// What to say after a Linux Set as Default: a status line, and the whole
/// account for the dialog.
///
/// Success is `xdg-mime default` succeeding -- the last command, and the one
/// that records the choice. A rebuild that failed is still listed, because
/// it can make the change take effect late.
pub(crate) fn linux_outcome(
    installed: &Installed,
    root: &Path,
    ran: &[(FollowUp, CommandOutcome)],
    types: usize,
    after: &AssociationReport,
) -> (String, String) {
    let mut body = if installed.written.is_empty() {
        "Wrote no files: this machine already had them.\n".to_owned()
    } else {
        let mut out = format!(
            "Wrote {} files under {}:\n",
            installed.written.len(),
            root.display()
        );
        for path in &installed.written {
            out.push_str(&format!("  {}\n", path.display()));
        }
        out
    };
    body.push('\n');
    for (step, outcome) in ran {
        body.push_str(&step.report(outcome));
        body.push('\n');
    }
    body.push('\n');
    body.push_str(&after.summary());
    body.push('\n');

    let status = match ran.last() {
        Some((_, CommandOutcome::Succeeded)) => {
            let rebuilds_failed = ran
                .iter()
                .any(|(_, outcome)| *outcome != CommandOutcome::Succeeded);
            if rebuilds_failed {
                format!(
                    "BachelorPad+ is now the default for {types} file types — a database \
                     rebuild did not run; the dialog says which"
                )
            } else {
                format!("BachelorPad+ is now the default for {types} file types")
            }
        }
        Some((step, outcome)) => step.report(outcome),
        None => "nothing was run".to_owned(),
    };
    (status, body)
}

/// What opens `selection` on this machine now.
pub(crate) fn report_now(selection: &AssociationSelection) -> AssociationReport {
    let env = DesktopEnv::from_environment();
    let app = this_app(&env).unwrap_or_else(|_| AppInfo::packaged());
    association_report_in(Platform::HOST, &env, &app, selection)
}

/// Write the `.reg` pair where the user chose to put the first.
///
/// The Windows answer to a registration this product will not apply
/// (ADR-0032): files the user picked the location of and can read every
/// line of before running. The remove script goes beside the register one
/// under its own name. Both are written as `Artefact::bytes`, which is the
/// UTF-16 regedit expects.
pub(crate) fn save_registry_scripts(
    plan: &RegistrationPlan,
    target: &Path,
) -> Result<String, String> {
    if let Some(objection) = registry_objection(plan) {
        return Err(objection);
    }
    let (Some(register), Some(unregister)) = (
        plan.artefact(ArtefactKind::RegistryScript),
        plan.artefact(ArtefactKind::UnregisterScript),
    ) else {
        return Err(
            "nothing was written — this registration has no registry scripts in it".to_owned(),
        );
    };
    let beside = target.with_file_name(&unregister.relative_path);
    std::fs::write(target, register.bytes())
        .map_err(|e| format!("could not write {} — {e}", target.display()))?;
    std::fs::write(&beside, unregister.bytes())
        .map_err(|e| format!("could not write {} — {e}", beside.display()))?;
    Ok(format!(
        "wrote {} ({} values, every one under HKEY_CURRENT_USER) and {} beside it; read them \
         before you run them",
        target.display(),
        plan.registry_values.len(),
        unregister.relative_path,
    ))
}

/// The file name to suggest for the register script: `bp-platform`'s own,
/// so the file the user saves is the file the plan names.
pub(crate) fn registry_script_name(plan: &RegistrationPlan) -> String {
    plan.artefact(ArtefactKind::RegistryScript)
        .map_or_else(|| "file-types.reg".to_owned(), |a| a.relative_path.clone())
}

fn unregister_script_name(plan: &RegistrationPlan) -> String {
    plan.artefact(ArtefactKind::UnregisterScript).map_or_else(
        || "file-types-remove.reg".to_owned(),
        |a| a.relative_path.clone(),
    )
}

/// Everything Remove Registration has to show before removing anything.
#[derive(Debug)]
pub(crate) struct Removal {
    pub(crate) plan: Unregistration,
    pub(crate) body: String,
}

/// Build the removal for this machine.
pub(crate) fn removal() -> Result<Removal, String> {
    let env = DesktopEnv::from_environment();
    removal_for(&this_app(&env)?, &env)
}

/// The removal for a named environment. `Err` carries the status line: a
/// refusal, or that there is nothing of this product's to remove.
pub(crate) fn removal_for(app: &AppInfo, env: &DesktopEnv) -> Result<Removal, String> {
    let plan = unregistration_plan(app, env).map_err(|r| r.describe())?;
    if plan.is_empty() {
        return Err(format!(
            "nothing to remove — Set as Default has written nothing under {}",
            plan.root.display()
        ));
    }
    let mut body = format!(
        "Remove Registration removes {} files under {}:\n",
        plan.remove.len(),
        plan.root.display()
    );
    for path in &plan.remove {
        body.push_str(&format!("  {path}\n"));
    }
    if !plan.skipped.is_empty() {
        body.push_str("\nIt leaves these alone:\n");
        for skip in &plan.skipped {
            body.push_str(&format!("  {}\n      {}\n", skip.path.display(), skip.why));
        }
    }
    body.push_str(
        "\nThen it rebuilds the databases those files were listed in. Defaults recorded in \
         mimeapps.list are yours and are left as they are; your desktop skips one naming an \
         application that is no longer registered, and its own settings are where a choice is \
         changed.\n",
    );
    Ok(Removal { plan, body })
}

/// What to say after Remove Registration.
pub(crate) fn removal_outcome(
    result: Result<Uninstalled, InstallRefusal>,
    run: impl FnMut(&FollowUp) -> CommandOutcome,
) -> (String, Option<String>) {
    match result {
        Err(refusal) => (refusal.describe(), None),
        Ok(done) => {
            let mut body = format!("Removed {} files:\n", done.removed.len());
            for path in &done.removed {
                body.push_str(&format!("  {}\n", path.display()));
            }
            for skip in &done.skipped {
                body.push_str(&format!("Left {} — {}\n", skip.path.display(), skip.why));
            }
            let ran = run_all(&done.follow_up, run);
            if !ran.is_empty() {
                body.push('\n');
            }
            for (step, outcome) in &ran {
                body.push_str(&step.report(outcome));
                body.push('\n');
            }
            (
                format!(
                    "removed {} registration files; BachelorPad+ no longer appears in Open With",
                    done.removed.len()
                ),
                Some(body),
            )
        }
    }
}

/// Whether a platform-integration row can act, and if not, the short hint
/// the menu shows in its place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RowState {
    Live,
    Greyed(&'static str),
}

/// The File menu's two integration rows, read from the machine rather than
/// every time the menu is built: the answer changes only when one of the
/// two rows has just been used, and the shell reads it again then.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Rows {
    pub(crate) set_default: RowState,
    /// `None` where there is no such row: on Windows the `.reg` pair's
    /// remove script is the way back.
    pub(crate) remove: Option<RowState>,
}

impl Default for Rows {
    /// What a machine that has not been asked looks like: Set as Default
    /// live, nothing to remove. The tests' state, and the state before the
    /// first read.
    fn default() -> Self {
        Self {
            set_default: RowState::Live,
            remove: None,
        }
    }
}

impl Rows {
    /// Read this machine.
    pub(crate) fn read() -> Self {
        let env = DesktopEnv::from_environment();
        let app = this_app(&env).unwrap_or_else(|_| AppInfo::packaged());
        Self::read_for(Platform::HOST, &app, &env)
    }

    pub(crate) fn read_for(platform: Platform, app: &AppInfo, env: &DesktopEnv) -> Self {
        if platform == Platform::Windows {
            // The `.reg` route is always on offer, whatever the capability
            // register says about installing.
            return Self {
                set_default: RowState::Live,
                remove: None,
            };
        }
        if !set_as_default_availability(platform, env).is_usable() {
            let greyed = RowState::Greyed("managed by the Flatpak sandbox");
            return Self {
                set_default: greyed,
                remove: Some(greyed),
            };
        }
        let remove = match unregistration_plan(app, env) {
            Ok(plan) if !plan.is_empty() => RowState::Live,
            _ => RowState::Greyed("nothing to remove"),
        };
        Self {
            set_default: RowState::Live,
            remove: Some(remove),
        }
    }
}

#[cfg(test)]
mod tests;
