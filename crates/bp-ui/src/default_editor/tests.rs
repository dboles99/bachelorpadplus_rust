use std::collections::BTreeMap;

use super::*;
use bp_platform::EnvSnapshot;
use bp_platform::editor::Consent;

/// The application, described for a named platform.
///
/// The executable has to be absolute in *that* platform's spelling:
/// `AppInfo::problems` refuses a relative path, and a relative path is what
/// a Windows path looks like to the Linux rules.
fn app_for(platform: Platform) -> AppInfo {
    AppInfo::bachelorpad(match platform {
        Platform::Windows => r"C:\Program Files\BachelorPad\bachelorpad.exe",
        Platform::Linux => "/usr/bin/bachelorpad",
    })
}

/// An environment whose every root, system ones included, is inside `dir`.
///
/// Used only with [`Platform::HOST`], because a temporary directory is
/// absolute in the host's spelling and in no other. Everything that has to
/// be asserted for *both* platforms goes through [`linux_body`] and
/// [`windows_body`] instead, which touch no filesystem at all.
fn env_under(dir: &Path) -> DesktopEnv {
    let at = |name: &str| Some(dir.join(name).display().to_string());
    DesktopEnv {
        xdg_data_dirs: at("system-data"),
        xdg_config_dirs: at("system-config"),
        ..DesktopEnv::from_dirs(EnvSnapshot {
            appdata: at("roaming"),
            local_appdata: at("local"),
            user_profile: at("profile"),
            home: at("home"),
            xdg_config_home: at("config"),
            xdg_data_home: at("data"),
            xdg_cache_home: at("cache"),
            xdg_state_home: at("state"),
        })
    }
}

fn selection() -> AssociationSelection {
    AssociationSelection::preset(PACKAGED_PRESET)
}

/// The command a Linux plan ends with, built by hand: the real one comes
/// from `linux_plan`, which needs a filesystem.
fn set_default_step() -> FollowUp {
    FollowUp {
        program: "xdg-mime",
        args: vec![
            "default".to_owned(),
            "io.github.dboles99.BachelorPad.desktop".to_owned(),
            "text/plain".to_owned(),
        ],
        why: "records the choice",
    }
}

/// The Linux consent dialog, built from values.
fn linux_body(mimeapps: &str) -> String {
    let app = app_for(Platform::Linux);
    let plan = bp_platform::editor::plan(Platform::Linux, &app, &selection());
    let report = bp_platform::editor::state::report_from_mimeapps(mimeapps, &app, &selection());
    let mut commands = plan.follow_up.clone();
    commands.push(set_default_step());
    let mut body = report_text(&report);
    body.push_str(&claimed_line(&plan));
    body.push_str(&linux_steps(
        &plan,
        Path::new("/home/tester/.local/share"),
        &[],
        &commands,
    ));
    body
}

/// The Windows consent dialog, built from values.
fn windows_body(windows: &WindowsFacts) -> Offer {
    offer_for(
        Platform::Windows,
        &app_for(Platform::Windows),
        &DesktopEnv::default(),
        windows,
    )
    .expect("a Windows offer can always be described")
}

#[test]
fn the_dialog_says_what_opens_these_file_types_before_it_offers_to_change_anything() {
    // ADR-0012 is about the user keeping control. A user cannot keep
    // control of a change they were not shown the starting point of.
    let body = linux_body("");
    assert!(
        body.contains("Nothing is set to open .txt files."),
        "an empty mimeapps.list means nothing claims .txt; got:\n{body}"
    );
    assert!(body.contains("Registering claims"), "got:\n{body}");
}

#[test]
fn an_existing_association_is_reported_as_the_application_that_holds_it() {
    let body = linux_body("[Default Applications]\ntext/plain=org.gnome.gedit.desktop;\n");
    assert!(
        body.contains("org.gnome.gedit.desktop opens .txt files."),
        "the current handler has to be named; got:\n{body}"
    );
}

#[test]
fn the_linux_dialog_names_the_root_the_files_and_every_command_it_will_run() {
    // The root, because `install` takes an explicit one and a root the
    // dialog does not name is one picked out of sight. The commands,
    // because confirming this dialog runs them, and a command the user was
    // not shown is one run behind their back.
    let body = linux_body("");
    assert!(body.contains("/home/tester/.local/share"), "got:\n{body}");
    assert!(
        body.contains("applications/io.github.dboles99.BachelorPad.desktop"),
        "got:\n{body}"
    );
    let promise = body
        .find("After you confirm, BachelorPad+ will run:")
        .expect("the commands are introduced as what confirming does");
    assert!(
        body[promise..].contains("xdg-mime default io.github.dboles99.BachelorPad.desktop"),
        "got:\n{body}"
    );
    assert!(
        body[promise..].contains("update-desktop-database"),
        "got:\n{body}"
    );
}

#[test]
fn windows_without_a_registry_to_ask_reports_that_it_cannot_tell() {
    // `Unknown` is not `Unclaimed`, and a dialog rendering "not the
    // default" out of "could not look" invites the user to fix something
    // that is not broken.
    let body = windows_body(&WindowsFacts::none()).body;
    assert!(body.contains("Cannot tell"), "got:\n{body}");
    assert!(!body.contains("Nothing is set to open"), "got:\n{body}");
}

#[test]
fn windows_with_a_registry_names_what_opens_each_type_now() {
    let choices: BTreeMap<String, String> =
        BTreeMap::from([("txt".to_owned(), "Notepad++_file".to_owned())]);
    let body = windows_body(&WindowsFacts {
        choices: Some(Box::new(choices)),
        ..WindowsFacts::none()
    })
    .body;
    assert!(body.contains("Notepad++_file"), "got:\n{body}");
    assert!(!body.contains("Cannot tell"), "got:\n{body}");
}

#[test]
fn an_unregistered_windows_install_is_offered_the_reg_pair_in_the_library_s_words() {
    // The refusal string is the capability register's, so this dialog and
    // `install`'s own answer cannot drift apart. This asserts the shell's
    // copy *is* `install`'s copy, by asking `install` for it.
    let dir = tempfile::tempdir().expect("a temporary directory");
    let offer = windows_body(&WindowsFacts::none());
    let refused = bp_platform::editor::install(&offer.plan, dir.path(), Consent::Granted)
        .expect_err("Windows has no supported install");

    assert_eq!(offer.route, Route::SaveFiles);
    assert_eq!(offer.route.button(), "Save Files...");
    assert!(
        offer.body.contains(&refused.describe()),
        "got:\n{}",
        offer.body
    );
    assert!(
        offer.body.contains("bachelorpad-file-types.reg")
            && offer.body.contains("bachelorpad-file-types-remove.reg"),
        "both files are named; got:\n{}",
        offer.body
    );
    assert_eq!(
        std::fs::read_dir(dir.path())
            .expect("the root is readable")
            .count(),
        0,
        "a refused install must write nothing"
    );
}

#[test]
fn a_registered_windows_install_is_sent_to_its_own_page_in_settings() {
    let offer = windows_body(&WindowsFacts {
        registered: Some(r"Software\BachelorPad+\Capabilities".to_owned()),
        build: Some(22631),
        choices: None,
    });
    let Route::OpenSettings { uri } = &offer.route else {
        panic!("a registered product goes to Settings, not to a Save dialog");
    };
    assert!(uri.starts_with("ms-settings:defaultapps"), "got {uri}");
    assert!(uri.contains("registeredAppUser="), "got {uri}");
    assert_eq!(offer.route.button(), "Open Settings");
    assert!(offer.body.contains(uri.as_str()), "got:\n{}", offer.body);
}

#[test]
fn a_plan_that_would_touch_another_application_is_refused_before_it_is_offered() {
    use bp_platform::editor::{RegistryValue, ValueName};

    let mut plan =
        bp_platform::editor::plan(Platform::Windows, &app_for(Platform::Windows), &selection());
    assert_eq!(registry_objection(&plan), None);

    plan.registry_values.push(RegistryValue {
        key: r"HKCU\Software\Classes\txtfile\shell\open\command".to_owned(),
        name: ValueName::Default,
        data: "notepad.exe".to_owned(),
    });
    let objection = registry_objection(&plan).expect("another application's ProgID");
    assert!(objection.contains("txtfile"), "got '{objection}'");

    let dir = tempfile::tempdir().expect("a temporary directory");
    let target = dir.path().join("out.reg");
    assert!(save_registry_scripts(&plan, &target).is_err());
    assert_eq!(
        std::fs::read_dir(dir.path())
            .expect("the directory is readable")
            .count(),
        0,
        "an objectionable plan must not reach a file"
    );
}

/// A `.reg` file's text: UTF-16LE after a byte-order mark.
fn decode_reg(bytes: &[u8]) -> String {
    assert_eq!(
        bytes.get(..2),
        Some(&[0xFF, 0xFE][..]),
        "regedit wants the BOM"
    );
    let units: Vec<u16> = bytes[2..]
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .collect();
    String::from_utf16(&units).expect("valid UTF-16")
}

#[test]
fn the_registry_scripts_are_written_where_the_user_chose_and_nowhere_else() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let plan =
        bp_platform::editor::plan(Platform::Windows, &app_for(Platform::Windows), &selection());
    let target = dir.path().join(registry_script_name(&plan));

    let status = save_registry_scripts(&plan, &target).expect("the scripts are written");
    let register = decode_reg(&std::fs::read(&target).expect("the script is readable"));
    let remove = decode_reg(
        &std::fs::read(dir.path().join("bachelorpad-file-types-remove.reg"))
            .expect("the remove script sits beside it"),
    );

    for written in [&register, &remove] {
        assert!(written.starts_with("Windows Registry Editor Version 5.00"));
        assert!(
            !written.to_ascii_lowercase().contains("notepad"),
            "ADR-0012: this product never names Notepad in a registry script"
        );
        assert!(
            written
                .lines()
                .filter(|line| line.starts_with('['))
                .all(|line| line.starts_with("[HKEY_CURRENT_USER\\")
                    || line.starts_with("[-HKEY_CURRENT_USER\\")),
            "every key must be under the user hive"
        );
    }
    assert_eq!(
        std::fs::read_dir(dir.path())
            .expect("the directory is readable")
            .count(),
        2
    );
    assert!(
        status.contains("read them before you run them"),
        "got '{status}'"
    );
}

#[test]
fn every_command_runs_in_order_even_after_one_fails() {
    let commands = vec![
        FollowUp {
            program: "update-desktop-database",
            args: vec!["/d/applications".to_owned()],
            why: "",
        },
        set_default_step(),
    ];
    let mut order = Vec::new();
    let ran = run_all(&commands, |step| {
        order.push(step.program);
        if step.program == "xdg-mime" {
            CommandOutcome::Succeeded
        } else {
            CommandOutcome::NotFound
        }
    });
    assert_eq!(order, ["update-desktop-database", "xdg-mime"]);
    assert_eq!(ran.len(), 2);
}

#[test]
fn the_default_is_claimed_only_when_xdg_mime_succeeded() {
    let installed = Installed {
        written: vec![PathBuf::from("/d/applications/x.desktop")],
        follow_up: Vec::new(),
    };
    let after = bp_platform::editor::state::report_from_mimeapps(
        "",
        &app_for(Platform::Linux),
        &selection(),
    );
    let rebuild = FollowUp {
        program: "update-desktop-database",
        args: vec!["/d/applications".to_owned()],
        why: "",
    };

    let all_well = [
        (rebuild.clone(), CommandOutcome::Succeeded),
        (set_default_step(), CommandOutcome::Succeeded),
    ];
    let (status, _) = linux_outcome(&installed, Path::new("/d"), &all_well, 7, &after);
    assert_eq!(status, "BachelorPad+ is now the default for 7 file types");

    let rebuild_missing = [
        (rebuild.clone(), CommandOutcome::NotFound),
        (set_default_step(), CommandOutcome::Succeeded),
    ];
    let (status, body) = linux_outcome(&installed, Path::new("/d"), &rebuild_missing, 7, &after);
    assert!(status.contains("did not run"), "got '{status}'");
    assert!(body.contains("update-desktop-database"), "got:\n{body}");

    let refused = [
        (rebuild, CommandOutcome::Succeeded),
        (
            set_default_step(),
            CommandOutcome::Failed {
                exit_code: Some(4),
                stderr: "no such file".to_owned(),
            },
        ),
    ];
    let (status, _) = linux_outcome(&installed, Path::new("/d"), &refused, 7, &after);
    assert!(!status.contains("now the default"), "got '{status}'");
    assert!(status.contains("xdg-mime"), "got '{status}'");
}

#[cfg(unix)]
#[test]
fn the_command_run_is_the_one_on_the_path_with_exactly_the_plan_s_arguments() {
    use std::os::unix::fs::PermissionsExt as _;

    let dir = tempfile::tempdir().expect("a temporary directory");
    let fake = dir.path().join("xdg-mime");
    // Shell builtins only: the PATH it runs on is this directory and
    // nothing else, so `dirname` would not be found.
    std::fs::write(
        &fake,
        "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"${0%/*}/ran\"\n",
    )
    .expect("the fake is written");
    std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755))
        .expect("the fake is executable");

    let outcome = run_command_on(&set_default_step(), Some(dir.path().as_os_str()));

    assert_eq!(outcome, CommandOutcome::Succeeded);
    assert_eq!(
        std::fs::read_to_string(dir.path().join("ran")).expect("the fake ran"),
        "default\nio.github.dboles99.BachelorPad.desktop\ntext/plain\n"
    );
}

#[cfg(unix)]
#[test]
fn a_command_that_is_not_installed_is_reported_as_missing_rather_than_failed() {
    let empty = tempfile::tempdir().expect("a temporary directory");
    assert_eq!(
        run_command_on(&set_default_step(), Some(empty.path().as_os_str())),
        CommandOutcome::NotFound
    );
}

#[test]
fn the_offer_takes_its_root_from_the_platform_rather_than_choosing_one() {
    // The one test that walks the real `offer_for` path on the host --
    // against an environment made of temporary directories, so nothing the
    // user owns is read or written.
    let dir = tempfile::tempdir().expect("a temporary directory");
    let env = env_under(dir.path());
    let offer = offer_for(
        Platform::HOST,
        &app_for(Platform::HOST),
        &env,
        &WindowsFacts::none(),
    )
    .expect("the host can always describe its own registration");

    match (&offer.route, Platform::HOST) {
        (Route::Install { root, commands }, Platform::Linux) => {
            assert_eq!(Some(root.display().to_string()), env.data_home());
            assert_eq!(
                commands.last().map(|step| step.program),
                Some("xdg-mime"),
                "the plan ends by recording the choice"
            );
        }
        (Route::SaveFiles, Platform::Windows) => {}
        (route, host) => panic!("{route:?} on {host:?}"),
    }
    assert!(
        offer.body.contains("Registering claims 7 file types"),
        "got:\n{}",
        offer.body
    );
}

#[test]
fn a_registration_naming_a_relative_executable_is_refused_before_anything_is_planned() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let app = AppInfo::bachelorpad("bachelorpad");
    let error = offer_for(
        Platform::HOST,
        &app,
        &env_under(dir.path()),
        &WindowsFacts::none(),
    )
    .expect_err("a relative executable cannot be registered");

    assert!(error.contains("relative"), "got '{error}'");
}

#[test]
fn linux_without_a_home_directory_says_so_rather_than_writing_somewhere_arbitrary() {
    let error = offer_for(
        Platform::Linux,
        &app_for(Platform::Linux),
        &DesktopEnv::default(),
        &WindowsFacts::none(),
    )
    .expect_err("there is nowhere to put a .desktop entry");

    assert!(error.contains("data directory"), "got '{error}'");
}

#[test]
fn inside_a_flatpak_both_rows_grey_and_say_why() {
    let env = DesktopEnv {
        flatpak_id: Some("io.github.dboles99.BachelorPad".to_owned()),
        ..DesktopEnv::default()
    };
    let rows = Rows::read_for(Platform::Linux, &app_for(Platform::Linux), &env);
    let greyed = RowState::Greyed("managed by the Flatpak sandbox");
    assert_eq!(rows.set_default, greyed);
    assert_eq!(rows.remove, Some(greyed));
}

#[test]
fn windows_offers_set_as_default_and_no_remove_row() {
    let rows = Rows::read_for(
        Platform::Windows,
        &app_for(Platform::Windows),
        &DesktopEnv::default(),
    );
    assert_eq!(rows.set_default, RowState::Live);
    assert_eq!(rows.remove, None);
}

#[cfg(unix)]
#[test]
fn remove_registration_comes_alive_after_set_as_default_and_takes_away_what_it_wrote() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let env = env_under(dir.path());
    let app = app_for(Platform::Linux);

    let before = Rows::read_for(Platform::Linux, &app, &env);
    assert_eq!(before.remove, Some(RowState::Greyed("nothing to remove")));
    assert!(removal_for(&app, &env).is_err(), "nothing to remove yet");

    let linux = linux_plan(&app, &selection(), &env).expect("a plan");
    bp_platform::editor::install(&linux.registration, &linux.root, Consent::Granted)
        .expect("installed under the temporary root");

    let after = Rows::read_for(Platform::Linux, &app, &env);
    assert_eq!(after.remove, Some(RowState::Live));

    let removal = removal_for(&app, &env).expect("something to remove now");
    assert!(
        removal
            .body
            .contains("applications/io.github.dboles99.BachelorPad.desktop"),
        "got:\n{}",
        removal.body
    );
    let mut ran = Vec::new();
    let (status, body) = removal_outcome(
        bp_platform::editor::uninstall(&removal.plan, &removal.plan.root, Consent::Granted),
        |step| {
            ran.push(step.program);
            CommandOutcome::Succeeded
        },
    );
    assert!(
        status.starts_with("removed 2 registration files"),
        "got '{status}'"
    );
    assert!(body.is_some());
    assert!(ran.contains(&"update-desktop-database"), "got {ran:?}");
    assert_eq!(
        Rows::read_for(Platform::Linux, &app, &env).remove,
        Some(RowState::Greyed("nothing to remove"))
    );
}

#[test]
fn no_message_this_module_writes_carries_a_wrapped_line_s_indentation() {
    // A `\` continuation strips the newline and the leading whitespace;
    // forgetting it leaves a run of spaces in the middle of a sentence the
    // user reads.
    let linux = linux_body("");
    let windows = windows_body(&WindowsFacts::none()).body;
    let registered = windows_body(&WindowsFacts {
        registered: Some(r"Software\BachelorPad+\Capabilities".to_owned()),
        ..WindowsFacts::none()
    })
    .body;
    for text in [&linux, &windows, &registered] {
        for line in text.lines() {
            let trimmed = line.trim_start();
            assert!(
                !trimmed.contains("  "),
                "a wrapped literal leaked its indentation: {line:?}"
            );
        }
    }
}
