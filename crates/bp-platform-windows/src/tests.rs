//! Tests for the safe half.
//!
//! Most run everywhere: the UTF-16 rules, the `ms-settings:` refusal and the
//! registry-path check are pure, and a Linux machine is as good a place as
//! any to prove them. The `windows` module at the end runs only where the
//! Win32 calls exist, and even there it only *reads* -- no test opens a
//! Settings page, shows a box or touches a console.

use super::*;
use crate::wide::WideString;

// --- UTF-16 ------------------------------------------------------------------

#[test]
fn to_wide_ends_in_exactly_one_nul() {
    assert_eq!(
        to_wide("ab"),
        Some(vec![u16::from(b'a'), u16::from(b'b'), 0])
    );
    assert_eq!(to_wide(""), Some(vec![0]));
}

#[test]
fn to_wide_refuses_a_nul_inside_rather_than_letting_win32_stop_there() {
    assert_eq!(to_wide("Software\\x\0\\y"), None);
}

#[test]
fn to_wide_writes_a_character_outside_the_basic_plane_as_a_surrogate_pair() {
    assert_eq!(to_wide("😀"), Some(vec![0xD83D, 0xDE00, 0]));
    assert_eq!(to_wide("Zoë"), Some(vec![0x5A, 0x6F, 0xEB, 0]));
}

#[test]
fn from_wide_stops_at_the_first_nul_however_many_there_are() {
    let units = to_wide("txtfile").expect("no NUL inside");
    assert_eq!(from_wide(&units).as_deref(), Some("txtfile"));
    let mut padded = units.clone();
    padded.extend([0, 0, u16::from(b'x')]);
    assert_eq!(from_wide(&padded).as_deref(), Some("txtfile"));
    // A value written with no terminator at all still reads whole.
    assert_eq!(from_wide(&units[..7]).as_deref(), Some("txtfile"));
}

#[test]
fn from_wide_refuses_a_lone_surrogate_rather_than_guessing() {
    assert_eq!(from_wide(&[0xD83D, 0]), None);
}

#[test]
fn text_survives_the_round_trip_through_utf16() {
    for text in ["", "BachelorPadPlus.txt", r"C:\Users\Zoë\東京\a.exe", "😀"] {
        let units = to_wide(text).expect("no NUL inside");
        assert_eq!(from_wide(&units).as_deref(), Some(text));
    }
}

#[test]
fn a_wide_string_is_terminated_once_and_a_lossy_one_keeps_the_rest_of_the_text() {
    let strict = WideString::new("ok").expect("no NUL inside");
    assert_eq!(strict.units(), &[u16::from(b'o'), u16::from(b'k'), 0]);
    assert_eq!(WideString::new("a\0b"), None);

    // A crash message with a stray NUL must still show the report path.
    let lossy = WideString::lossy("see\0C:\\crash.log");
    assert_eq!(lossy.units().iter().filter(|&&u| u == 0).count(), 1);
    assert_eq!(lossy.units().last(), Some(&0));
    assert_eq!(
        from_wide(lossy.units()).as_deref(),
        Some("see C:\\crash.log")
    );
}

// --- ms-settings: only -------------------------------------------------------

#[test]
fn only_a_settings_uri_is_passed_to_the_shell() {
    for good in [
        "ms-settings:",
        "ms-settings:defaultapps",
        "MS-Settings:defaultapps",
        "ms-settings:defaultapps?registeredAppUser=BachelorPad%2B",
    ] {
        assert!(is_settings_uri(good), "{good}");
    }
    for bad in [
        "",
        "ms-settings",
        "ms-settingsx:defaultapps",
        " ms-settings:defaultapps",
        "https://example.com/",
        "file:///C:/Windows/System32/cmd.exe",
        r"C:\Windows\System32\cmd.exe",
        "cmd.exe",
        "ms-settings:defaultapps cmd.exe",
        "ms-settings:defaultapps\0cmd.exe",
        "ms-settings:defaultapps\r\ncmd.exe",
    ] {
        assert!(!is_settings_uri(bad), "{bad:?}");
    }
}

#[test]
fn a_uri_that_is_not_a_settings_page_is_refused_before_anything_is_called() {
    // Safe to run on every platform, Windows included: the refusal happens
    // before the shell is reached, which is what is being asserted.
    assert_eq!(open_uri("cmd.exe"), Err(OpenUriError::NotASettingsUri));
    assert_eq!(
        open_uri("file:///C:/Windows/System32/cmd.exe"),
        Err(OpenUriError::NotASettingsUri)
    );
}

#[test]
fn every_link_bp_platform_chooses_is_one_this_crate_will_open() {
    // The two halves must agree, or "Open Windows Settings" is a button that
    // refuses its own address.
    let app = bp_platform::editor::AppInfo::bachelorpad(r"C:\BachelorPad+\bachelorpad.exe");
    for build in [None, Some(19045), Some(22000), Some(26100)] {
        let uri = bp_platform::editor::windows::default_apps_uri(build, &app);
        assert!(is_settings_uri(&uri), "{uri}");
    }
}

#[test]
fn every_refusal_says_what_happened_in_a_sentence_fragment() {
    for error in [
        OpenUriError::NotASettingsUri,
        OpenUriError::Unsupported,
        OpenUriError::Failed { code: 2 },
    ] {
        let text = error.to_string();
        assert!(text.len() > 10, "{text}");
        assert!(!text.ends_with('.'), "fragments join a status line: {text}");
    }
}

// --- registry paths ----------------------------------------------------------

#[test]
fn the_user_choice_key_is_found_with_or_without_the_dot_and_in_any_case() {
    let expected = r"Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\.txt\UserChoice";
    for spelling in ["txt", ".txt", "TXT", " .Txt "] {
        assert_eq!(
            user_choice_subkey(spelling).as_deref(),
            Some(expected),
            "{spelling:?}"
        );
    }
}

#[test]
fn an_extension_that_could_name_a_different_key_is_refused() {
    for bad in ["", ".", r"txt\..\..", "a/b", "t xt", "txt\0", "t\r\nxt"] {
        assert_eq!(user_choice_subkey(bad), None, "{bad:?}");
    }
    assert_eq!(class_default_prog_id(r"txt\..\.."), None);
}

#[test]
fn a_build_number_is_read_as_the_registry_stores_it() {
    assert_eq!(parse_build_number("22631"), Some(22631));
    assert_eq!(parse_build_number(" 19045\n"), Some(19045));
    for bad in ["", "10.0", "-1", "22631a", "99999999999"] {
        assert_eq!(parse_build_number(bad), None, "{bad:?}");
    }
}

#[test]
fn a_registered_application_name_with_a_control_character_is_not_looked_up() {
    assert_eq!(registered_application(""), None);
    assert_eq!(registered_application("BachelorPad+\r\n"), None);
}

#[test]
fn crate_name_is_the_package_name() {
    assert_eq!(CRATE_NAME, env!("CARGO_PKG_NAME"));
}

// --- off Windows: every answer is "no" ---------------------------------------

#[cfg(not(windows))]
#[test]
fn off_windows_every_function_answers_no_and_touches_nothing() {
    assert!(!attach_parent_console());
    assert!(
        standard_error_is_connected(),
        "stderr is the only answer here"
    );
    assert!(!show_fatal_message("title", "body"));
    assert!(!notify_associations_changed());
    assert_eq!(
        open_uri("ms-settings:defaultapps"),
        Err(OpenUriError::Unsupported)
    );
    assert_eq!(user_choice_prog_id("txt"), None);
    assert_eq!(class_default_prog_id("txt"), None);
    assert_eq!(registered_application("BachelorPad+"), None);
    assert_eq!(windows_build(), None);
    assert!(
        RegistryProgIds::new().is_none(),
        "a query that cannot look must not be obtainable"
    );
}

// --- on Windows: reads only ---------------------------------------------------

/// Run only on Windows, and only reads: nothing here changes the machine,
/// opens a window or attaches a console. Not yet run -- the development
/// container is Linux, where these compile for `x86_64-pc-windows-msvc` under
/// `cargo clippy --target` but cannot link; the Windows gate runs them.
#[cfg(windows)]
mod windows {
    use super::*;

    #[test]
    fn the_build_number_is_one_windows_10_or_later_would_report() {
        let build = windows_build().expect("CurrentBuildNumber is always present");
        assert!(build >= 10240, "{build} predates Windows 10");
    }

    #[test]
    fn an_extension_nobody_registers_has_no_recorded_choice() {
        let unlikely = "bp-platform-windows-test-extension-that-nobody-registers";
        assert_eq!(user_choice_prog_id(unlikely), None);
        assert_eq!(class_default_prog_id(unlikely), None);
    }

    #[test]
    fn the_registry_query_is_obtainable_here_and_answers_without_panicking() {
        let query = RegistryProgIds::new().expect("this is Windows");
        // `.txt` may or may not have a handler on a test machine; the
        // assertion is that asking is safe and yields a ProgID or nothing.
        if let Some(id) = query.current_prog_id("txt") {
            assert!(!id.is_empty());
            assert!(!id.contains('\0'));
        }
    }

    #[test]
    fn a_name_nobody_registered_is_not_a_registered_application() {
        assert_eq!(
            registered_application("bp-platform-windows test: nobody registers this"),
            None
        );
    }
}
