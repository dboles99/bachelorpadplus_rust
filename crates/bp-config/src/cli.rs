//! The command line: which words are flags, which are files, and what each
//! flag asked for.
//!
//! Owns the one definition of the flag set ([`FLAGS`]). [`crate::resolve`]
//! takes its configuration overrides from [`parse`] rather than scanning the
//! arguments itself, and the help text is generated from the same table, so a
//! flag cannot be understood by one and unknown to the other.
//!
//! Deliberately does not own:
//!
//! * what a setting's value *means* -- `--font-size=999` is a well-formed
//!   flag here, and [`crate::resolve`] is where it is range-checked and
//!   reported;
//! * printing, exiting, or touching the filesystem or the environment.
//!   [`parse`] is pure so every case below is a unit test rather than a
//!   process launch.
//!
//! The rule that shapes it: **a mistyped flag is never a file and never
//! fatal.** Before this module existed `bachelorpad --line 427 server.log`
//! opened a document called `427`. An unknown flag now becomes a
//! [`Warning`] the shell shows in its status bar, and the editor still starts.

use std::fmt;
use std::num::NonZeroUsize;
use std::path::PathBuf;

/// What the invocation asks the binary to do.
///
/// Ordered by precedence: `--help` beats `--version`, which beats running the
/// editor, so `bachelorpad --version --help` answers the more useful question.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Action {
    /// Open the editor.
    #[default]
    Run,
    /// Print `bachelorpad <version>` and exit.
    Version,
    /// Print [`usage`] and exit.
    Help,
}

/// A configuration setting the command line may override.
///
/// Carried as a name and a raw value: interpreting the value belongs to
/// [`crate::resolve`], which applies the same rules to the file, the
/// environment and the command line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Setting {
    Theme,
    Renderer,
    Log,
    FontSize,
    TabWidth,
    IndentSpaces,
}

impl Setting {
    /// Every setting, for tests that must not miss one.
    pub const ALL: &'static [Self] = &[
        Self::Theme,
        Self::Renderer,
        Self::Log,
        Self::FontSize,
        Self::TabWidth,
        Self::IndentSpaces,
    ];

    /// The flag's name without its leading `--`.
    #[must_use]
    pub const fn flag(self) -> &'static str {
        match self {
            Self::Theme => "theme",
            Self::Renderer => "renderer",
            Self::Log => "log",
            Self::FontSize => "font-size",
            Self::TabWidth => "tab-width",
            Self::IndentSpaces => "indent-spaces",
        }
    }
}

/// A command-line problem worth showing the user. Never fatal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Warning {
    /// Looked like a flag, is not one. Kept verbatim, as typed.
    UnknownFlag { flag: String },
    /// A flag that needs a value was last, or was followed by another flag.
    MissingValue { flag: &'static str },
    /// A switch was given a value, as in `--readonly=yes`.
    UnexpectedValue { flag: &'static str, value: String },
    /// `--line` was given something that is not a line number. Lines count
    /// from 1, so `0` lands here too.
    BadLine { value: String },
}

impl fmt::Display for Warning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownFlag { flag } => write!(f, "unknown option '{flag}'; ignored"),
            Self::MissingValue { flag } => write!(f, "--{flag} needs a value; ignored"),
            Self::UnexpectedValue { flag, value } => {
                write!(f, "--{flag} takes no value, not '{value}'; ignored")
            }
            Self::BadLine { value } => write!(
                f,
                "'{value}' is not a line number (lines count from 1); ignored"
            ),
        }
    }
}

/// Everything the command line said, sorted into what it means.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Invocation {
    pub action: Action,
    /// Files to open, in the order given. Repeats are kept: whether a second
    /// mention of one file is a second tab is the shell's decision.
    pub files: Vec<PathBuf>,
    /// `--line N`: where the caret should land in the first file.
    pub line: Option<NonZeroUsize>,
    /// `--readonly`: open the files without allowing edits.
    pub readonly: bool,
    /// `--large-file`: open the files in the large-file viewer.
    pub large_file: bool,
    /// `--self-check`: print a marker and exit, for the local CI gate.
    pub self_check: bool,
    /// `--latency-probe`: run the typing-latency probe and exit.
    pub latency_probe: bool,
    /// `--measure-exit`: report time to first frame and quit.
    pub measure_exit: bool,
    /// `--editor-view`: draw with the custom editor surface (ADR-0018).
    pub editor_view: bool,
    /// Configuration overrides, in the order given. Applied in order by
    /// [`crate::resolve`], so a repeated flag's last value wins -- the same
    /// answer a later layer gives over an earlier one.
    pub settings: Vec<(Setting, String)>,
    /// Problems, in the order they were met.
    pub warnings: Vec<Warning>,
}

/// What a recognised flag does once it has been matched.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Meaning {
    Help,
    Version,
    Line,
    Readonly,
    LargeFile,
    SelfCheck,
    LatencyProbe,
    MeasureExit,
    EditorView,
    Setting(Setting),
}

/// One row of the flag table.
#[derive(Debug, Clone, Copy)]
pub struct Flag {
    /// Long name without the leading `--`.
    pub name: &'static str,
    /// Single-letter alias including its `-`, for the two flags people type
    /// from muscle memory.
    pub short: Option<&'static str>,
    /// Placeholder for the value, or `None` for a switch.
    pub value: Option<&'static str>,
    /// One line for `--help`.
    pub help: &'static str,
    meaning: Meaning,
}

const fn switch(name: &'static str, help: &'static str, meaning: Meaning) -> Flag {
    Flag {
        name,
        short: None,
        value: None,
        help,
        meaning,
    }
}

const fn setting(setting: Setting, value: &'static str, help: &'static str) -> Flag {
    Flag {
        name: setting.flag(),
        short: None,
        value: Some(value),
        help,
        meaning: Meaning::Setting(setting),
    }
}

/// The flag set. The one place a flag is defined: [`parse`] matches against
/// it and [`usage`] is printed from it.
pub const FLAGS: &[Flag] = &[
    Flag {
        name: "line",
        short: None,
        value: Some("N"),
        help: "put the caret on line N of the first file",
        meaning: Meaning::Line,
    },
    switch(
        "readonly",
        "open the files without allowing edits",
        Meaning::Readonly,
    ),
    switch(
        "large-file",
        "open the files in the large-file viewer",
        Meaning::LargeFile,
    ),
    setting(Setting::Theme, "NAME", "colour theme"),
    setting(
        Setting::Renderer,
        "software|platform",
        "which renderer draws the window",
    ),
    setting(Setting::Log, "FILTER", "log filter, e.g. warn or debug"),
    setting(Setting::FontSize, "POINTS", "editor font size"),
    setting(Setting::TabWidth, "COLUMNS", "tab width"),
    setting(
        Setting::IndentSpaces,
        "true|false",
        "whether Tab inserts spaces",
    ),
    switch(
        "editor-view",
        "draw with the custom editor view (experimental)",
        Meaning::EditorView,
    ),
    switch(
        "self-check",
        "print a start-up marker and exit",
        Meaning::SelfCheck,
    ),
    switch(
        "latency-probe",
        "measure typing latency and exit",
        Meaning::LatencyProbe,
    ),
    switch(
        "measure-exit",
        "report time to first frame and exit",
        Meaning::MeasureExit,
    ),
    Flag {
        name: "help",
        short: Some("-h"),
        value: None,
        help: "print this help and exit",
        meaning: Meaning::Help,
    },
    Flag {
        name: "version",
        short: Some("-V"),
        value: None,
        help: "print the version and exit",
        meaning: Meaning::Version,
    },
];

fn lookup_long(name: &str) -> Option<&'static Flag> {
    FLAGS.iter().find(|f| f.name == name)
}

fn lookup_short(arg: &str) -> Option<&'static Flag> {
    FLAGS.iter().find(|f| f.short == Some(arg))
}

/// Sort the command line into files, flags and warnings.
///
/// `args` excludes the program name (`std::env::args().skip(1)`): `argv[0]`
/// is not a file the user asked for.
///
/// A value flag accepts both `--flag=value` and `--flag value`. In the second
/// form the next argument is taken as the value unless it begins with `--`,
/// so `--theme --readonly` warns about the missing theme instead of looking
/// for a theme called `--readonly`, while `--font-size -5` still reaches the
/// range check that explains why `-5` is wrong. After a bare `--` every
/// argument is a file, which is how a file called `-notes.txt` is opened.
#[must_use]
pub fn parse(args: &[String]) -> Invocation {
    let mut invocation = Invocation::default();
    let mut rest = args.iter();

    while let Some(arg) = rest.next() {
        if arg == "--" {
            invocation.files.extend(rest.by_ref().map(PathBuf::from));
            break;
        }

        let (flag, inline) = if let Some(long) = arg.strip_prefix("--") {
            let (name, inline) = match long.split_once('=') {
                Some((name, value)) => (name, Some(value)),
                None => (long, None),
            };
            (lookup_long(name), inline)
        } else if arg.starts_with('-') {
            // A lone `-` lands here too. Conventionally it means standard
            // input, which an editor opening files by path cannot honour;
            // saying so beats opening a document called `-`.
            (lookup_short(arg), None)
        } else {
            invocation.files.push(PathBuf::from(arg));
            continue;
        };

        let Some(flag) = flag else {
            invocation
                .warnings
                .push(Warning::UnknownFlag { flag: arg.clone() });
            continue;
        };

        let value = if flag.value.is_some() {
            let taken = match inline {
                Some(value) => Some(value.to_owned()),
                None => match rest.as_slice().first() {
                    Some(next) if !next.starts_with("--") => {
                        rest.next();
                        Some(next.clone())
                    }
                    _ => None,
                },
            };
            let Some(taken) = taken else {
                invocation
                    .warnings
                    .push(Warning::MissingValue { flag: flag.name });
                continue;
            };
            Some(taken)
        } else {
            if let Some(value) = inline {
                invocation.warnings.push(Warning::UnexpectedValue {
                    flag: flag.name,
                    value: value.to_owned(),
                });
                continue;
            }
            None
        };

        apply(&mut invocation, flag.meaning, value);
    }

    invocation
}

fn apply(invocation: &mut Invocation, meaning: Meaning, value: Option<String>) {
    match meaning {
        Meaning::Help => invocation.action = Action::Help,
        Meaning::Version => {
            if invocation.action != Action::Help {
                invocation.action = Action::Version;
            }
        }
        Meaning::Line => {
            let value = value.unwrap_or_default();
            match value.trim().parse::<NonZeroUsize>() {
                Ok(line) => invocation.line = Some(line),
                Err(_) => invocation.warnings.push(Warning::BadLine { value }),
            }
        }
        Meaning::Readonly => invocation.readonly = true,
        Meaning::LargeFile => invocation.large_file = true,
        Meaning::SelfCheck => invocation.self_check = true,
        Meaning::LatencyProbe => invocation.latency_probe = true,
        Meaning::MeasureExit => invocation.measure_exit = true,
        Meaning::EditorView => invocation.editor_view = true,
        Meaning::Setting(setting) => invocation
            .settings
            .push((setting, value.unwrap_or_default())),
    }
}

/// The `--help` text, generated from [`FLAGS`] so it cannot fall behind them.
#[must_use]
pub fn usage() -> String {
    let rows: Vec<(String, &str)> = FLAGS
        .iter()
        .map(|flag| {
            let mut left = String::new();
            if let Some(short) = flag.short {
                left.push_str(short);
                left.push_str(", ");
            }
            left.push_str("--");
            left.push_str(flag.name);
            if let Some(value) = flag.value {
                left.push(' ');
                left.push_str(value);
            }
            (left, flag.help)
        })
        .chain(std::iter::once((
            "--".to_owned(),
            "treat every later argument as a file",
        )))
        .collect();
    let width = rows.iter().map(|(left, _)| left.len()).max().unwrap_or(0);

    let mut text = String::from(
        "Usage: bachelorpad [OPTIONS] [--] [FILE]...\n\n\
         Opens each FILE in a tab. Settings given here override the\n\
         environment and config.toml for this run only.\n\nOptions:\n",
    );
    for (left, help) in rows {
        text.push_str(&format!("  {left:<width$}  {help}\n"));
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_strs(list: &[&str]) -> Invocation {
        let args: Vec<String> = list.iter().map(|s| (*s).to_owned()).collect();
        parse(&args)
    }

    fn line(n: usize) -> Option<NonZeroUsize> {
        NonZeroUsize::new(n)
    }

    #[test]
    fn a_line_number_is_not_mistaken_for_a_file() {
        // The bug this module exists for: `427` used to open as a document.
        let got = parse_strs(&["--line", "427", "server.log"]);
        assert_eq!(got.line, line(427));
        assert_eq!(got.files, vec![PathBuf::from("server.log")]);
        assert!(got.warnings.is_empty(), "{:?}", got.warnings);
    }

    #[test]
    fn the_line_may_be_joined_to_its_flag_with_an_equals_sign() {
        let got = parse_strs(&["server.log", "--line=12"]);
        assert_eq!(got.line, line(12));
        assert_eq!(got.files, vec![PathBuf::from("server.log")]);
    }

    #[test]
    fn line_zero_is_refused_with_a_warning() {
        let got = parse_strs(&["--line=0", "a.txt"]);
        assert_eq!(got.line, None, "lines count from 1");
        assert_eq!(
            got.warnings,
            vec![Warning::BadLine {
                value: "0".to_owned()
            }]
        );
        assert_eq!(got.files, vec![PathBuf::from("a.txt")]);
    }

    #[test]
    fn a_line_that_is_not_a_number_is_a_warning_and_not_a_file() {
        let got = parse_strs(&["--line", "abc", "a.txt"]);
        assert_eq!(got.line, None);
        assert_eq!(
            got.files,
            vec![PathBuf::from("a.txt")],
            "the malformed value was consumed as the flag's, so it must not reappear as a file"
        );
        assert_eq!(
            got.warnings,
            vec![Warning::BadLine {
                value: "abc".to_owned()
            }]
        );
    }

    #[test]
    fn a_line_flag_with_nothing_after_it_warns() {
        let got = parse_strs(&["a.txt", "--line"]);
        assert_eq!(got.line, None);
        assert_eq!(got.warnings, vec![Warning::MissingValue { flag: "line" }]);
    }

    #[test]
    fn a_value_flag_does_not_swallow_the_next_flag() {
        let got = parse_strs(&["--line", "--readonly", "a.txt"]);
        assert_eq!(got.line, None);
        assert!(
            got.readonly,
            "--readonly is a flag, not the line number; it must still take effect"
        );
        assert_eq!(got.warnings, vec![Warning::MissingValue { flag: "line" }]);
        assert_eq!(got.files, vec![PathBuf::from("a.txt")]);
    }

    #[test]
    fn the_last_line_given_wins() {
        let got = parse_strs(&["--line", "3", "--line=9"]);
        assert_eq!(got.line, line(9));
    }

    #[test]
    fn readonly_and_large_file_are_switches() {
        let got = parse_strs(&["--readonly", "huge.log", "--large-file", "data.jsonl"]);
        assert!(got.readonly);
        assert!(got.large_file);
        assert_eq!(
            got.files,
            vec![PathBuf::from("huge.log"), PathBuf::from("data.jsonl")]
        );
        assert!(got.warnings.is_empty());
    }

    #[test]
    fn a_switch_given_a_value_warns_and_is_not_applied() {
        let got = parse_strs(&["--readonly=no", "a.txt"]);
        assert!(
            !got.readonly,
            "'--readonly=no' is ambiguous; applying it would read as the opposite of what was typed"
        );
        assert_eq!(
            got.warnings,
            vec![Warning::UnexpectedValue {
                flag: "readonly",
                value: "no".to_owned()
            }]
        );
    }

    #[test]
    fn files_are_kept_in_order_including_repeats() {
        let got = parse_strs(&["b.txt", "a.txt", "b.txt"]);
        assert_eq!(
            got.files,
            vec![
                PathBuf::from("b.txt"),
                PathBuf::from("a.txt"),
                PathBuf::from("b.txt")
            ]
        );
    }

    #[test]
    fn a_double_dash_makes_everything_after_it_a_file() {
        let got = parse_strs(&["--readonly", "--", "-weird.txt", "--line", "--help"]);
        assert!(got.readonly, "flags before the -- still count");
        assert_eq!(
            got.files,
            vec![
                PathBuf::from("-weird.txt"),
                PathBuf::from("--line"),
                PathBuf::from("--help")
            ]
        );
        assert_eq!(got.action, Action::Run, "--help after -- is a file name");
        assert_eq!(got.line, None);
        assert!(got.warnings.is_empty());
    }

    #[test]
    fn an_unknown_flag_warns_and_is_never_a_file() {
        let got = parse_strs(&["--frobnicate", "-x", "-", "--wat=1", "a.txt"]);
        assert_eq!(got.files, vec![PathBuf::from("a.txt")]);
        assert_eq!(
            got.warnings,
            ["--frobnicate", "-x", "-", "--wat=1"]
                .iter()
                .map(|flag| Warning::UnknownFlag {
                    flag: (*flag).to_owned()
                })
                .collect::<Vec<_>>()
        );
        assert_eq!(got.action, Action::Run, "an unknown flag is never fatal");
    }

    #[test]
    fn version_and_help_are_actions() {
        assert_eq!(parse_strs(&[]).action, Action::Run);
        assert_eq!(parse_strs(&["--version"]).action, Action::Version);
        assert_eq!(parse_strs(&["-V"]).action, Action::Version);
        assert_eq!(parse_strs(&["--help"]).action, Action::Help);
        assert_eq!(parse_strs(&["-h"]).action, Action::Help);
    }

    #[test]
    fn help_wins_over_version_whichever_comes_first() {
        assert_eq!(parse_strs(&["--version", "--help"]).action, Action::Help);
        assert_eq!(parse_strs(&["--help", "--version"]).action, Action::Help);
    }

    #[test]
    fn the_developer_switches_are_recognised() {
        let got = parse_strs(&[
            "--self-check",
            "--latency-probe",
            "--measure-exit",
            "--editor-view",
        ]);
        assert!(got.self_check && got.latency_probe && got.measure_exit && got.editor_view);
        assert!(got.files.is_empty());
        assert!(got.warnings.is_empty());
    }

    #[test]
    fn settings_accept_both_the_joined_and_the_separate_form() {
        let got = parse_strs(&[
            "--theme=Dark",
            "--font-size",
            "18",
            "--renderer",
            "gpu",
            "a.txt",
        ]);
        assert_eq!(
            got.settings,
            vec![
                (Setting::Theme, "Dark".to_owned()),
                (Setting::FontSize, "18".to_owned()),
                (Setting::Renderer, "gpu".to_owned()),
            ]
        );
        assert_eq!(
            got.files,
            vec![PathBuf::from("a.txt")],
            "a setting's value must not also open as a file"
        );
    }

    #[test]
    fn a_negative_value_is_still_taken_as_the_settings_value() {
        // So that resolve can say why -5 is wrong, rather than this module
        // calling it an unknown flag.
        let got = parse_strs(&["--font-size", "-5"]);
        assert_eq!(got.settings, vec![(Setting::FontSize, "-5".to_owned())]);
        assert!(got.warnings.is_empty());
    }

    #[test]
    fn every_setting_is_a_value_flag_in_the_table() {
        for setting in Setting::ALL {
            let flag = lookup_long(setting.flag())
                .unwrap_or_else(|| panic!("--{} is missing from FLAGS", setting.flag()));
            assert_eq!(flag.meaning, Meaning::Setting(*setting));
            assert!(flag.value.is_some(), "--{} needs a value", setting.flag());
        }
    }

    #[test]
    fn flag_names_are_unique() {
        for (i, a) in FLAGS.iter().enumerate() {
            for b in &FLAGS[i + 1..] {
                assert_ne!(a.name, b.name, "two rows define --{}", a.name);
                if a.short.is_some() {
                    assert_ne!(a.short, b.short, "two rows share {:?}", a.short);
                }
            }
        }
    }

    #[test]
    fn the_help_lists_every_flag() {
        let text = usage();
        for flag in FLAGS {
            assert!(
                text.contains(&format!("--{}", flag.name)),
                "--help does not mention --{}",
                flag.name
            );
            if let Some(short) = flag.short {
                assert!(text.contains(short), "--help does not mention {short}");
            }
        }
        assert!(
            text.contains("\n  --  "),
            "--help must explain the bare -- separator"
        );
    }

    #[test]
    fn warnings_read_as_sentences_that_say_what_was_ignored() {
        let got = parse_strs(&["--nope", "--line=x"]);
        let rendered: Vec<String> = got.warnings.iter().map(ToString::to_string).collect();
        assert_eq!(
            rendered,
            vec![
                "unknown option '--nope'; ignored".to_owned(),
                "'x' is not a line number (lines count from 1); ignored".to_owned(),
            ]
        );
    }
}
