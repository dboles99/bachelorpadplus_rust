//! Tests for Set as Default on Linux.
//!
//! Two halves. The rules that are pure over a [`DesktopEnv`] or a set of
//! texts -- the lookup order, the precedence of one file over another,
//! Flatpak, `$APPIMAGE` -- run on every host. The ones that need files on
//! disk build a whole XDG layout (home, `/etc/xdg`, `/usr/share`) inside a
//! temporary directory and are `cfg(unix)`: a temporary directory is an
//! absolute *Linux* path only on a Unix host, and on Windows the XDG rules
//! would rightly refuse `C:\...` as relative.

use super::*;
use crate::editor::{AssociationPreset, CommandOutcome};

fn app() -> AppInfo {
    AppInfo::bachelorpad("/home/me/bin/bachelorpad")
}

fn notepad_replacement() -> AssociationSelection {
    AssociationSelection::preset(AssociationPreset::NotepadReplacement)
}

/// An environment made of plain strings, for the rules that read no file.
fn string_env() -> DesktopEnv {
    DesktopEnv {
        dirs: EnvSnapshot {
            home: Some("/home/me".into()),
            ..EnvSnapshot::default()
        },
        xdg_data_dirs: Some("/usr/local/share:/usr/share".into()),
        xdg_config_dirs: Some("/etc/xdg".into()),
        xdg_current_desktop: Some("ubuntu:GNOME".into()),
        ..DesktopEnv::default()
    }
}

// --- the environment -------------------------------------------------------

#[test]
fn the_mimeapps_lists_are_consulted_in_the_order_the_specification_gives() {
    let lists: Vec<String> = string_env()
        .mimeapps_lists()
        .iter()
        .map(|p| p.display().to_string())
        .collect();
    assert_eq!(
        lists,
        [
            "/home/me/.config/ubuntu-mimeapps.list",
            "/home/me/.config/gnome-mimeapps.list",
            "/home/me/.config/mimeapps.list",
            "/etc/xdg/ubuntu-mimeapps.list",
            "/etc/xdg/gnome-mimeapps.list",
            "/etc/xdg/mimeapps.list",
            "/home/me/.local/share/applications/ubuntu-mimeapps.list",
            "/home/me/.local/share/applications/gnome-mimeapps.list",
            "/home/me/.local/share/applications/mimeapps.list",
            "/usr/local/share/applications/ubuntu-mimeapps.list",
            "/usr/local/share/applications/gnome-mimeapps.list",
            "/usr/local/share/applications/mimeapps.list",
            "/usr/share/applications/ubuntu-mimeapps.list",
            "/usr/share/applications/gnome-mimeapps.list",
            "/usr/share/applications/mimeapps.list",
        ]
    );
}

#[test]
fn unset_search_paths_mean_the_specifications_defaults_and_a_relative_entry_is_ignored() {
    let unset = DesktopEnv::from_dirs(string_env().dirs);
    assert_eq!(unset.data_dirs(), ["/usr/local/share", "/usr/share"]);
    assert_eq!(unset.config_dirs(), ["/etc/xdg"]);
    assert!(unset.current_desktops().is_empty());

    let odd = DesktopEnv {
        xdg_data_dirs: Some("relative/share:/opt/share/:".into()),
        ..string_env()
    };
    assert_eq!(odd.data_dirs(), ["/opt/share"]);
}

#[test]
fn flatpak_makes_set_as_default_unavailable_with_the_reason_the_row_shows() {
    let env = DesktopEnv {
        flatpak_id: Some(crate::APP_ID.into()),
        ..string_env()
    };
    let availability = set_as_default_availability(Platform::Linux, &env);
    assert_eq!(
        availability,
        Availability::Unsupported {
            reason: FLATPAK_REASON
        }
    );
    assert!(!availability.is_usable(), "the shell greys the row on this");

    let refused = linux_plan(&app(), &notepad_replacement(), &env).expect_err("sandboxed");
    assert_eq!(
        refused,
        InstallRefusal::NotSupported {
            platform: Platform::Linux,
            reason: FLATPAK_REASON
        }
    );
    assert!(refused.describe().contains("Flatpak"));
    assert!(
        unregistration_plan(&app(), &env).is_err(),
        "Remove Registration is unavailable in the sandbox too"
    );
}

#[test]
fn the_flatpak_marker_file_is_enough_without_the_variable() {
    let env = DesktopEnv {
        flatpak_info_present: true,
        ..string_env()
    };
    assert!(env.is_flatpak());
    assert!(!set_as_default_availability(Platform::Linux, &env).is_usable());
}

#[test]
fn outside_a_sandbox_linux_can_set_the_default_and_windows_says_what_the_register_says() {
    assert_eq!(
        set_as_default_availability(Platform::Linux, &string_env()),
        Availability::Available
    );
    assert_eq!(
        set_as_default_availability(Platform::Windows, &DesktopEnv::default()),
        Capability::AssociationInstall.availability(Platform::Windows)
    );
}

#[test]
fn an_appimage_is_registered_by_the_appimage_file_and_not_by_its_temporary_mount() {
    let env = DesktopEnv {
        appimage: Some("/home/me/Applications/BachelorPad+ 0.1.0.AppImage".into()),
        ..string_env()
    };
    let app = AppInfo::launched_from("/tmp/.mount_BachelXYZ/usr/bin/bachelorpad", &env);
    assert_eq!(
        app.executable,
        "/home/me/Applications/BachelorPad+ 0.1.0.AppImage"
    );
    let entry = desktop::desktop_entry(&app, &notepad_replacement());
    assert!(
        entry.contains("Exec=\"/home/me/Applications/BachelorPad+ 0.1.0.AppImage\" %F\n"),
        "{entry}"
    );
    assert!(!entry.contains(".mount_"), "{entry}");
}

#[test]
fn without_an_appimage_the_running_executable_is_registered() {
    let relative = DesktopEnv {
        appimage: Some("BachelorPad.AppImage".into()),
        ..string_env()
    };
    for env in [string_env(), relative] {
        assert_eq!(
            AppInfo::launched_from("/usr/bin/bachelorpad", &env).executable,
            "/usr/bin/bachelorpad"
        );
    }
}

#[test]
fn no_data_home_is_a_refusal_rather_than_a_guess() {
    let env = DesktopEnv::default();
    assert_eq!(
        linux_plan(&app(), &notepad_replacement(), &env),
        Err(InstallRefusal::NoDataDirectory)
    );
    assert_eq!(
        unregistration_plan(&app(), &env),
        Err(InstallRefusal::NoDataDirectory)
    );
}

// --- the association chain, from text --------------------------------------

#[test]
fn a_desktop_specific_default_beats_the_generic_one_which_beats_the_system_one() {
    let desktop_specific = "[Default Applications]\ntext/plain=kate.desktop;\n";
    let generic = "[Default Applications]\ntext/plain=gedit.desktop;\ntext/x-log=gedit.desktop;\n";
    let system = "[Default Applications]\ntext/plain=vim.desktop;\ntext/markdown=vim.desktop;\n";
    let chain = AssociationChain::from_texts([desktop_specific, generic, system], []);
    let everything = |_: &str| true;
    assert_eq!(
        chain.default_for("text/plain", everything).as_deref(),
        Some("kate.desktop")
    );
    assert_eq!(
        chain.default_for("text/x-log", everything).as_deref(),
        Some("gedit.desktop")
    );
    assert_eq!(
        chain.default_for("text/markdown", everything).as_deref(),
        Some("vim.desktop")
    );
}

#[test]
fn the_mimeinfo_cache_is_consulted_only_when_no_list_names_a_default() {
    let list = "[Default Applications]\ntext/plain=gedit.desktop;\n";
    let cache = "[MIME Cache]\ntext/plain=kate.desktop;\ntext/x-log=kate.desktop;\n";
    let chain = AssociationChain::from_texts([list], [cache]);
    assert_eq!(
        chain.default_for("text/plain", |_| true).as_deref(),
        Some("gedit.desktop"),
        "an explicit default beats the cache"
    );
    assert_eq!(
        chain.default_for("text/x-log", |_| true).as_deref(),
        Some("kate.desktop"),
        "with no default anywhere, the cache decides"
    );
    assert_eq!(chain.default_for("text/csv", |_| true), None);
}

#[test]
fn a_default_naming_an_application_that_is_not_installed_falls_through_to_the_next() {
    let user = "[Default Applications]\ntext/plain=removed.desktop;also-removed.desktop;\n";
    let system = "[Default Applications]\ntext/plain=gedit.desktop;\n";
    let chain = AssociationChain::from_texts([user, system], []);
    assert_eq!(
        chain
            .default_for("text/plain", |id| id == "gedit.desktop")
            .as_deref(),
        Some("gedit.desktop")
    );
}

#[test]
fn a_removed_association_hides_a_fallback_but_never_an_explicit_default() {
    let user = "[Removed Associations]\ntext/plain=kate.desktop;\n";
    let cache = "[MIME Cache]\ntext/plain=kate.desktop;gedit.desktop;\n";
    let chain = AssociationChain::from_texts([user], [cache]);
    assert_eq!(
        chain.default_for("text/plain", |_| true).as_deref(),
        Some("gedit.desktop"),
        "the removal takes kate out of the fallback"
    );

    let explicit = "[Default Applications]\ntext/plain=kate.desktop;\n\
                    [Removed Associations]\ntext/plain=kate.desktop;\n";
    let chain = AssociationChain::from_texts([explicit], [cache]);
    assert_eq!(
        chain.default_for("text/plain", |_| true).as_deref(),
        Some("kate.desktop"),
        "an explicit default is not subject to removals"
    );
}

#[test]
fn an_added_association_is_preferred_to_the_cache_in_the_fallback() {
    let user = "[Added Associations]\ntext/plain=kate.desktop;\n";
    let cache = "[MIME Cache]\ntext/plain=gedit.desktop;\n";
    let chain = AssociationChain::from_texts([user], [cache]);
    assert_eq!(
        chain.default_for("text/plain", |_| true).as_deref(),
        Some("kate.desktop")
    );
}

// --- commands, as data -----------------------------------------------------

#[test]
fn a_command_outcome_is_reported_in_terms_of_what_it_means_for_the_user() {
    let step = FollowUp {
        program: "update-desktop-database",
        args: vec!["/home/me/.local/share/applications".into()],
        why: "Until the desktop database is rebuilt, the new entry does not appear in Open With.",
    };
    assert_eq!(
        step.report(&CommandOutcome::Succeeded),
        "Ran update-desktop-database /home/me/.local/share/applications"
    );
    let failed = step.report(&CommandOutcome::Failed {
        exit_code: Some(1),
        stderr: "\n  Could not parse file\nmore\n".into(),
    });
    assert!(failed.contains("exit status 1"), "{failed}");
    assert!(failed.contains("(Could not parse file)"), "{failed}");
    assert!(failed.contains("does not appear in Open With"), "{failed}");
    let missing = step.report(&CommandOutcome::NotFound);
    assert!(
        missing.starts_with("update-desktop-database is not installed"),
        "{missing}"
    );
    let signalled = step.report(&CommandOutcome::Failed {
        exit_code: None,
        stderr: String::new(),
    });
    assert!(signalled.contains("stopped by a signal"), "{signalled}");
}

// --- on disk ---------------------------------------------------------------

#[cfg(unix)]
mod on_disk {
    use super::*;
    use crate::editor::desktop::PER_USER_MARKER;
    use crate::editor::{AssociationState, install};
    use tempfile::{TempDir, tempdir};

    fn ours() -> String {
        format!("{}.desktop", crate::APP_ID)
    }

    /// A whole machine's XDG layout inside one temporary directory: a home,
    /// `/etc/xdg`, `/usr/local/share` and `/usr/share`.
    fn machine() -> (TempDir, DesktopEnv) {
        let dir = tempdir().unwrap();
        let at = |p: &str| dir.path().join(p).display().to_string();
        let env = DesktopEnv {
            dirs: EnvSnapshot {
                home: Some(at("home")),
                xdg_config_home: Some(at("home/.config")),
                xdg_data_home: Some(at("home/.local/share")),
                ..EnvSnapshot::default()
            },
            xdg_data_dirs: Some(format!("{}:{}", at("usr/local/share"), at("usr/share"))),
            xdg_config_dirs: Some(at("etc/xdg")),
            xdg_current_desktop: Some("GNOME".into()),
            ..DesktopEnv::default()
        };
        (dir, env)
    }

    fn write(path: &Path, text: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    /// A minimal installed application claiming `mimes`.
    fn install_app(applications: &Path, id: &str, mimes: &str) {
        write(
            &applications.join(id),
            &format!("[Desktop Entry]\nType=Application\nName={id}\nExec=x %F\nMimeType={mimes}\n"),
        );
    }

    fn system_applications(dir: &TempDir) -> PathBuf {
        dir.path().join("usr/share/applications")
    }

    fn user_data(dir: &TempDir) -> PathBuf {
        dir.path().join("home/.local/share")
    }

    /// What a distribution package would install.
    fn package_install(dir: &TempDir) {
        for asset in desktop::packaged_assets() {
            let sub = if asset.file_name.ends_with(".desktop") {
                "applications"
            } else if asset.file_name.ends_with(".metainfo.xml") {
                "metainfo"
            } else {
                "mime/packages"
            };
            write(
                &dir.path()
                    .join("usr/share")
                    .join(sub)
                    .join(&asset.file_name),
                &asset.contents,
            );
        }
    }

    #[test]
    fn a_system_desktop_entry_suppresses_the_per_user_copy_and_the_plan_only_sets_the_default() {
        let (dir, env) = machine();
        package_install(&dir);

        let plan = linux_plan(&app(), &notepad_replacement(), &env).unwrap();

        assert!(
            plan.registration.artefacts.is_empty(),
            "nothing per-user to write: {:?}",
            plan.registration.artefacts
        );
        assert_eq!(plan.skipped.len(), 2, "{:?}", plan.skipped);
        assert!(
            plan.skipped
                .iter()
                .all(|s| s.path.starts_with(dir.path().join("usr/share")))
        );
        let commands: Vec<String> = plan.commands().iter().map(FollowUp::command_line).collect();
        assert_eq!(
            commands,
            [format!(
                "xdg-mime default {} application/x-bachelorpad-encrypted text/plain text/x-log",
                ours()
            )]
        );

        let installed = install(&plan.registration, &plan.root, Consent::Granted).unwrap();
        assert!(installed.written.is_empty());
        assert!(!user_data(&dir).join("applications").exists());
    }

    #[test]
    fn a_system_entry_claiming_fewer_types_than_selected_does_not_suppress_the_per_user_copy() {
        let (dir, env) = machine();
        package_install(&dir);
        let developer = AssociationSelection::preset(AssociationPreset::Developer);

        let plan = linux_plan(&app(), &developer, &env).unwrap();

        assert!(
            plan.registration
                .artefact(ArtefactKind::DesktopEntry)
                .is_some(),
            "the packaged entry does not claim Markdown, so it is no substitute"
        );
        assert!(
            plan.registration
                .artefact(ArtefactKind::MimePackage)
                .is_some(),
            "nor does the packaged MIME package define PowerShell"
        );
    }

    #[test]
    fn without_a_system_entry_the_plan_writes_both_files_and_names_absolute_directories() {
        let (dir, env) = machine();
        let plan = linux_plan(&app(), &notepad_replacement(), &env).unwrap();
        let root = user_data(&dir);

        assert_eq!(plan.root, root);
        assert_eq!(plan.registration.artefacts.len(), 2);
        let commands: Vec<String> = plan.commands().iter().map(FollowUp::command_line).collect();
        assert_eq!(
            commands,
            [
                format!("update-desktop-database {}/applications", root.display()),
                format!("update-mime-database {}/mime", root.display()),
                format!(
                    "xdg-mime default {} application/x-bachelorpad-encrypted text/plain \
                     text/x-log",
                    ours()
                ),
            ]
        );

        let installed = install(&plan.registration, &plan.root, Consent::Granted).unwrap();
        assert_eq!(installed.follow_up, plan.registration.follow_up);
        let entry = fs::read_to_string(root.join("applications").join(ours())).unwrap();
        assert!(entry.starts_with(&format!("# {PER_USER_MARKER}\n[Desktop Entry]\n")));
        assert!(entry.contains("Exec=/home/me/bin/bachelorpad %F\n"));
    }

    #[test]
    fn a_per_user_copy_this_product_wrote_is_refreshed_even_beside_a_system_entry() {
        // It shadows the system entry whether or not it is rewritten, so it
        // may as well name the executable the user is running now.
        let (dir, env) = machine();
        let stale = AppInfo::bachelorpad("/old/build/bachelorpad");
        let first = linux_plan(&stale, &notepad_replacement(), &env).unwrap();
        install(&first.registration, &first.root, Consent::Granted).unwrap();
        package_install(&dir);

        let plan = linux_plan(&app(), &notepad_replacement(), &env).unwrap();

        assert!(
            plan.registration
                .artefact(ArtefactKind::DesktopEntry)
                .is_some()
        );
        install(&plan.registration, &plan.root, Consent::Granted).unwrap();
        let entry = fs::read_to_string(user_data(&dir).join("applications").join(ours())).unwrap();
        assert!(
            entry.contains("Exec=/home/me/bin/bachelorpad %F"),
            "{entry}"
        );
    }

    #[test]
    fn a_per_user_file_the_product_did_not_write_is_neither_overwritten_nor_removed() {
        let (dir, env) = machine();
        let customised = user_data(&dir).join("applications").join(ours());
        let text = "[Desktop Entry]\nType=Application\nName=My Editor\nExec=bachelorpad --x %F\n";
        write(&customised, text);

        let plan = linux_plan(&app(), &notepad_replacement(), &env).unwrap();
        assert!(
            plan.registration
                .artefact(ArtefactKind::DesktopEntry)
                .is_none()
        );
        assert!(plan.skipped.iter().any(|s| s.path == customised));
        install(&plan.registration, &plan.root, Consent::Granted).unwrap();

        let removal = unregistration_plan(&app(), &env).unwrap();
        assert!(!removal.remove.contains(&format!("applications/{}", ours())));
        uninstall(&removal, &removal.root, Consent::Granted).unwrap();
        assert_eq!(fs::read_to_string(&customised).unwrap(), text);
    }

    #[test]
    fn uninstall_removes_exactly_what_install_wrote_and_nothing_else() {
        let (dir, env) = machine();
        let root = user_data(&dir);
        let plan = linux_plan(&app(), &notepad_replacement(), &env).unwrap();
        let installed = install(&plan.registration, &plan.root, Consent::Granted).unwrap();

        // Neighbours that must survive: another application's per-user
        // entry, the user's recorded defaults, and a system entry.
        install_app(&root.join("applications"), "gedit.desktop", "text/plain;");
        let mimeapps = dir.path().join("home/.config/mimeapps.list");
        let choices = format!("[Default Applications]\ntext/plain={};\n", ours());
        write(&mimeapps, &choices);
        install_app(&system_applications(&dir), "vim.desktop", "text/plain;");

        let removal = unregistration_plan(&app(), &env).unwrap();
        assert_eq!(removal.remove.len(), 2, "{removal:?}");
        let done = uninstall(&removal, &removal.root, Consent::Granted).unwrap();

        let mut removed = done.removed.clone();
        let mut written = installed.written.clone();
        removed.sort();
        written.sort();
        assert_eq!(removed, written, "exactly the files install wrote");
        assert!(root.join("applications/gedit.desktop").is_file());
        assert!(system_applications(&dir).join("vim.desktop").is_file());
        assert_eq!(fs::read_to_string(&mimeapps).unwrap(), choices);
        let commands: Vec<String> = done.follow_up.iter().map(FollowUp::command_line).collect();
        assert_eq!(
            commands,
            [
                format!("update-desktop-database {}/applications", root.display()),
                format!("update-mime-database {}/mime", root.display()),
            ]
        );

        let again = unregistration_plan(&app(), &env).unwrap();
        assert!(again.is_empty(), "nothing is left to remove: {again:?}");
    }

    #[test]
    fn uninstall_without_consent_removes_nothing() {
        let (_dir, env) = machine();
        let plan = linux_plan(&app(), &notepad_replacement(), &env).unwrap();
        let installed = install(&plan.registration, &plan.root, Consent::Granted).unwrap();
        let removal = unregistration_plan(&app(), &env).unwrap();

        assert_eq!(
            uninstall(&removal, &removal.root, Consent::Withheld),
            Err(InstallRefusal::ConsentWithheld)
        );
        assert!(installed.written.iter().all(|p| p.is_file()));
    }

    #[test]
    fn a_hand_built_removal_naming_another_applications_entry_removes_nothing() {
        let (dir, _env) = machine();
        let root = user_data(&dir);
        install_app(&root.join("applications"), "gedit.desktop", "text/plain;");
        let removal = Unregistration {
            root: root.clone(),
            remove: vec!["applications/gedit.desktop".into()],
            skipped: Vec::new(),
        };

        let done = uninstall(&removal, &root, Consent::Granted).unwrap();

        assert!(done.removed.is_empty());
        assert_eq!(done.skipped.len(), 1);
        assert!(root.join("applications/gedit.desktop").is_file());

        let escaping = Unregistration {
            remove: vec!["../../usr/share/applications/vim.desktop".into()],
            ..removal
        };
        assert!(matches!(
            uninstall(&escaping, &root, Consent::Granted),
            Err(InstallRefusal::UnsafeArtefactPath { .. })
        ));
    }

    #[test]
    fn after_a_package_install_there_is_nothing_to_remove() {
        let (dir, env) = machine();
        package_install(&dir);
        let removal = unregistration_plan(&app(), &env).unwrap();
        assert!(removal.is_empty(), "{removal:?}");
        assert!(
            removal.skipped.is_empty(),
            "system files are never candidates"
        );
    }

    #[test]
    fn a_symbolic_link_at_our_path_is_refused_rather_than_followed() {
        let (dir, env) = machine();
        let target = dir.path().join("elsewhere.desktop");
        write(&target, &format!("# {PER_USER_MARKER}\n[Desktop Entry]\n"));
        let link = user_data(&dir).join("applications").join(ours());
        fs::create_dir_all(link.parent().unwrap()).unwrap();
        std::os::unix::fs::symlink(&target, &link).unwrap();

        let removal = unregistration_plan(&app(), &env).unwrap();
        assert!(removal.is_empty(), "{removal:?}");
        assert!(removal.skipped.iter().any(|s| s.path == link));
        assert!(target.is_file());
    }

    #[test]
    fn the_report_follows_the_chain_across_real_files() {
        let (dir, env) = machine();
        let config = dir.path().join("home/.config");
        let system = system_applications(&dir);
        for id in [
            "kate.desktop",
            "gedit.desktop",
            "vim.desktop",
            "less.desktop",
        ] {
            install_app(&system, id, "text/plain;");
        }
        let plan = linux_plan(&app(), &notepad_replacement(), &env).unwrap();
        install(&plan.registration, &plan.root, Consent::Granted).unwrap();

        // `.txt`: the desktop-specific list beats the generic one.
        write(
            &config.join("gnome-mimeapps.list"),
            &format!("[Default Applications]\ntext/plain={};\n", ours()),
        );
        write(
            &config.join("mimeapps.list"),
            "[Default Applications]\ntext/plain=gedit.desktop;\n\
             application/x-bachelorpad-encrypted=uninstalled.desktop;\n",
        );
        // `.log`: only the administrator's list says anything.
        write(
            &dir.path().join("etc/xdg/mimeapps.list"),
            "[Default Applications]\ntext/x-log=vim.desktop;\n",
        );
        // `.bpadx`: the user's default names nothing installed, so the cache
        // decides -- and the user's removal takes `less` out of it.
        write(
            &dir.path().join("etc/xdg/gnome-mimeapps.list"),
            "[Removed Associations]\napplication/x-bachelorpad-encrypted=less.desktop;\n",
        );
        write(
            &system.join("mimeinfo.cache"),
            &format!(
                "[MIME Cache]\napplication/x-bachelorpad-encrypted=less.desktop;{};\n",
                ours()
            ),
        );

        let report = association_report_in(Platform::Linux, &env, &app(), &notepad_replacement());

        assert_eq!(report.state_of("txt"), Some(&AssociationState::Ours));
        assert_eq!(
            report.state_of("log"),
            Some(&AssociationState::Other {
                handler: "vim.desktop".into()
            })
        );
        assert_eq!(report.state_of("bpadx"), Some(&AssociationState::Ours));
    }

    #[test]
    fn a_default_left_behind_by_a_removed_registration_is_not_reported_as_ours() {
        let (dir, env) = machine();
        write(
            &dir.path().join("home/.config/mimeapps.list"),
            &format!("[Default Applications]\ntext/plain={};\n", ours()),
        );
        let report = association_report_in(Platform::Linux, &env, &app(), &notepad_replacement());
        assert_eq!(report.state_of("txt"), Some(&AssociationState::Unclaimed));
    }
}
