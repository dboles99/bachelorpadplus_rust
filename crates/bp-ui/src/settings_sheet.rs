//! The Settings sheet's model (release plan M1-03).
//!
//! `ui/settings.slint` draws rows and reports "row N, one step this way".
//! Everything else is here: which settings there are, what values each can
//! take, what a step past the last one does, and what saving does. Kept
//! apart from the window so all of it is tested without one.
//!
//! The sheet edits a [`Draft`], never the live state, so Cancel is simply
//! dropping it. Save applies only what the draft changed: a setting the user
//! did not touch is neither applied nor written, so opening Settings and
//! pressing Save cannot turn a one-off `--font-size` into a permanent one,
//! nor write a theme into a config.toml that never named one.

use bp_config::SecurityProfile;
use bp_theme::ThemeId;

use crate::SettingRow;
use crate::state::AppState;

/// The largest-file choices, in MiB. Steps through a ladder rather than one
/// MiB at a time, which would take a hundred and ninety presses to cross.
/// Bounded by `bp-config`'s own 1..=192.
const LIMITS_MIB: [u8; 9] = [1, 2, 4, 8, 16, 32, 64, 128, 192];

const TAB_WIDTHS: [u8; 3] = [2, 4, 8];

/// Every setting the sheet offers, in the order it shows them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Row {
    Theme,
    FontSize,
    WordWrap,
    LineNumbers,
    IndentWith,
    TabWidth,
    DefaultProfile,
    RecordOtherPrograms,
    EditableLimit,
    EditorView,
}

const ROWS: [Row; 10] = [
    Row::Theme,
    Row::FontSize,
    Row::WordWrap,
    Row::LineNumbers,
    Row::IndentWith,
    Row::TabWidth,
    Row::DefaultProfile,
    Row::RecordOtherPrograms,
    Row::EditableLimit,
    Row::EditorView,
];

/// The settings as the sheet is editing them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Draft {
    /// `None` follows the desktop.
    theme: Option<ThemeId>,
    font_size: u8,
    word_wrap: bool,
    line_numbers: bool,
    indent_spaces: bool,
    tab_width: u8,
    default_profile: SecurityProfile,
    record_other_programs: bool,
    editable_limit_mib: u8,
    editor_view: bool,
}

/// An open sheet: what it started from, and where it has got to.
#[derive(Debug, Clone)]
pub(crate) struct Sheet {
    original: Draft,
    pub(crate) draft: Draft,
}

impl Sheet {
    /// Open the sheet on what is in force now.
    pub(crate) fn open(state: &AppState) -> Self {
        let draft = Draft::from_state(state);
        Self {
            original: draft.clone(),
            draft,
        }
    }

    /// The rows to draw.
    pub(crate) fn rows(&self) -> Vec<SettingRow> {
        ROWS.iter().map(|&row| self.draft.row(row)).collect()
    }

    /// Move `row` one step: `delta` below zero goes back, anything else on.
    /// Wraps at both ends, like the menu's own rows.
    pub(crate) fn step(&mut self, row: i32, delta: i32) {
        let Some(&row) = usize::try_from(row).ok().and_then(|i| ROWS.get(i)) else {
            return;
        };
        self.draft.step(row, if delta < 0 { -1 } else { 1 });
    }

    /// Apply what changed to `state` and save it to config.toml.
    pub(crate) fn save(self, state: &mut AppState) {
        let (was, now) = (self.original, self.draft);
        if was.theme != now.theme {
            state.follow_system_theme = now.theme.is_none();
            match now.theme {
                Some(theme) => state.theme = theme,
                None => {
                    if let Some(theme) = ThemeId::for_system(state.system_dark) {
                        state.theme = theme;
                    }
                }
            }
        }
        state.font_size = now.font_size;
        state.wrap_text = now.word_wrap;
        state.show_gutter = now.line_numbers;
        state.indent.spaces = now.indent_spaces;
        state.indent.width = usize::from(now.tab_width);
        state.default_profile = crate::state::profile_named(now.default_profile);
        state.record_other_programs = now.record_other_programs;
        state.editable_limit = u64::from(now.editable_limit_mib) * 1024 * 1024;

        state.remember(|c| {
            if was.theme != now.theme {
                c.theme = Some(
                    now.theme
                        .map_or(bp_theme::SYSTEM_THEME_NAME, ThemeId::name)
                        .to_owned(),
                );
            }
            if was.font_size != now.font_size {
                c.font_size = now.font_size;
            }
            if was.word_wrap != now.word_wrap {
                c.word_wrap = now.word_wrap;
            }
            if was.line_numbers != now.line_numbers {
                c.line_numbers = now.line_numbers;
            }
            if was.indent_spaces != now.indent_spaces {
                c.indent_spaces = now.indent_spaces;
            }
            if was.tab_width != now.tab_width {
                c.tab_width = now.tab_width;
            }
            if was.default_profile != now.default_profile {
                c.default_profile = now.default_profile;
            }
            if was.record_other_programs != now.record_other_programs {
                c.clipboard_record_other_programs = now.record_other_programs;
            }
            if was.editable_limit_mib != now.editable_limit_mib {
                c.editable_limit_mib = now.editable_limit_mib;
            }
            if was.editor_view != now.editor_view {
                c.editor_view = now.editor_view;
            }
        });
    }
}

impl Draft {
    fn from_state(state: &AppState) -> Self {
        // The two settings with no live field of their own are read back
        // from what config.toml last said: the custom view because the
        // session's is also switched on by a flag, and the profile because
        // the state holds `bp-security`'s type and the file `bp-config`'s.
        let saved = state.settings.as_ref().map(|s| &s.baseline);
        let default_profile = SecurityProfile::ALL
            .iter()
            .copied()
            .find(|p| crate::state::profile_named(*p) == state.default_profile)
            .unwrap_or(SecurityProfile::Standard);
        Self {
            theme: (!state.follow_system_theme).then_some(state.theme),
            font_size: state.font_size,
            word_wrap: state.wrap_text,
            line_numbers: state.show_gutter,
            indent_spaces: state.indent.spaces,
            tab_width: u8::try_from(state.indent.width).unwrap_or(4),
            default_profile,
            record_other_programs: state.record_other_programs,
            editable_limit_mib: u8::try_from(state.editable_limit / (1024 * 1024))
                .unwrap_or(u8::MAX),
            editor_view: saved.is_some_and(|c| c.editor_view),
        }
    }

    fn row(&self, row: Row) -> SettingRow {
        let on_off = |on: bool| if on { "On" } else { "Off" };
        let (label, value, note): (&str, String, &str) = match row {
            Row::Theme => (
                "Theme",
                self.theme.map_or("System", ThemeId::name).to_owned(),
                "System follows the desktop's light or dark setting",
            ),
            Row::FontSize => ("Font size", format!("{} pt", self.font_size), ""),
            Row::WordWrap => ("Word wrap", on_off(self.word_wrap).to_owned(), ""),
            Row::LineNumbers => ("Line numbers", on_off(self.line_numbers).to_owned(), ""),
            Row::IndentWith => (
                "Indent with",
                if self.indent_spaces { "Spaces" } else { "Tabs" }.to_owned(),
                "",
            ),
            Row::TabWidth => ("Tab width", format!("{} columns", self.tab_width), ""),
            Row::DefaultProfile => (
                "Security profile for new documents",
                self.default_profile.name().to_owned(),
                "Documents already open keep their own",
            ),
            Row::RecordOtherPrograms => (
                "Clipboard history records other programs",
                on_off(self.record_other_programs).to_owned(),
                "Off keeps what a password manager copies out of the Edit menu",
            ),
            Row::EditableLimit => (
                "Largest file opened for editing",
                format!("{} MiB", self.editable_limit_mib),
                "Larger files are refused rather than slow to a crawl",
            ),
            Row::EditorView => (
                "Custom editor view",
                on_off(self.editor_view).to_owned(),
                "From the next launch",
            ),
        };
        SettingRow {
            label: label.into(),
            value: value.into(),
            note: note.into(),
        }
    }

    fn step(&mut self, row: Row, delta: i32) {
        match row {
            Row::Theme => {
                // System first, then the themes in their own order.
                let choices: Vec<Option<ThemeId>> = std::iter::once(None)
                    .chain(ThemeId::ALL.into_iter().map(Some))
                    .collect();
                self.theme = cycle(&choices, &self.theme, delta);
            }
            Row::FontSize => {
                self.font_size = bp_config::zoom(self.font_size, if delta < 0 { -1 } else { 1 });
            }
            Row::WordWrap => self.word_wrap = !self.word_wrap,
            Row::LineNumbers => self.line_numbers = !self.line_numbers,
            Row::IndentWith => self.indent_spaces = !self.indent_spaces,
            Row::TabWidth => self.tab_width = cycle(&TAB_WIDTHS, &self.tab_width, delta),
            Row::DefaultProfile => {
                self.default_profile = cycle(SecurityProfile::ALL, &self.default_profile, delta);
            }
            Row::RecordOtherPrograms => {
                self.record_other_programs = !self.record_other_programs;
            }
            Row::EditableLimit => {
                self.editable_limit_mib = step_ladder(self.editable_limit_mib, delta);
            }
            Row::EditorView => self.editor_view = !self.editor_view,
        }
    }
}

/// The value one step from `current` in `choices`, wrapping. A current value
/// not in the list -- a tab width of 3 from a hand-edited file -- steps to
/// the first choice, so the row is never stuck.
fn cycle<T: PartialEq + Clone>(choices: &[T], current: &T, delta: i32) -> T {
    let len = choices.len();
    let next = match choices.iter().position(|c| c == current) {
        None => 0,
        Some(at) if delta < 0 => (at + len - 1) % len,
        Some(at) => (at + 1) % len,
    };
    choices
        .get(next)
        .cloned()
        .unwrap_or_else(|| current.clone())
}

/// The next rung of [`LIMITS_MIB`] from `current`, stopping at the ends --
/// wrapping from 192 MiB to 1 MiB would be a surprise nobody wants.
fn step_ladder(current: u8, delta: i32) -> u8 {
    if delta < 0 {
        LIMITS_MIB
            .iter()
            .rev()
            .find(|&&rung| rung < current)
            .copied()
            .unwrap_or(LIMITS_MIB[0])
    } else {
        LIMITS_MIB
            .iter()
            .find(|&&rung| rung > current)
            .copied()
            .unwrap_or(LIMITS_MIB[LIMITS_MIB.len() - 1])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row_value(sheet: &Sheet, label: &str) -> String {
        sheet
            .rows()
            .into_iter()
            .find(|r| r.label.as_str() == label)
            .map(|r| r.value.to_string())
            .unwrap_or_else(|| panic!("no row {label:?}"))
    }

    fn index(row: Row) -> i32 {
        ROWS.iter()
            .position(|&r| r == row)
            .and_then(|i| i32::try_from(i).ok())
            .unwrap_or(-1)
    }

    #[test]
    fn the_sheet_opens_on_what_is_in_force() {
        let mut state = AppState::new();
        state.wrap_text = true;
        state.indent.width = 8;
        let sheet = Sheet::open(&state);
        assert_eq!(row_value(&sheet, "Word wrap"), "On");
        assert_eq!(row_value(&sheet, "Tab width"), "8 columns");
    }

    #[test]
    fn a_toggle_flips_either_way_and_a_choice_wraps() {
        let state = AppState::new();
        let mut sheet = Sheet::open(&state);
        sheet.step(index(Row::WordWrap), -1);
        assert!(sheet.draft.word_wrap);

        sheet.draft.tab_width = 8;
        sheet.step(index(Row::TabWidth), 1);
        assert_eq!(sheet.draft.tab_width, 2, "past the last wraps to the first");
        sheet.step(index(Row::TabWidth), -1);
        assert_eq!(sheet.draft.tab_width, 8);
    }

    #[test]
    fn the_theme_row_offers_following_the_system_first() {
        let state = AppState::new();
        let mut sheet = Sheet::open(&state);
        sheet.draft.theme = Some(ThemeId::ALL[ThemeId::ALL.len() - 1]);
        sheet.step(index(Row::Theme), 1);
        assert_eq!(sheet.draft.theme, None);
        assert_eq!(row_value(&sheet, "Theme"), "System");
        sheet.step(index(Row::Theme), 1);
        assert_eq!(sheet.draft.theme, Some(ThemeId::ALL[0]));
    }

    #[test]
    fn the_size_limit_climbs_a_ladder_and_stops_at_its_ends() {
        assert_eq!(step_ladder(32, 1), 64);
        assert_eq!(step_ladder(32, -1), 16);
        assert_eq!(step_ladder(192, 1), 192);
        assert_eq!(step_ladder(1, -1), 1);
        // A hand-written value between rungs goes to the neighbouring rung.
        assert_eq!(step_ladder(40, 1), 64);
        assert_eq!(step_ladder(40, -1), 32);
    }

    #[test]
    fn a_value_not_on_offer_steps_onto_the_first_choice() {
        assert_eq!(cycle(&TAB_WIDTHS, &3, 1), 2);
    }

    #[test]
    fn a_step_on_a_row_that_does_not_exist_changes_nothing() {
        let state = AppState::new();
        let mut sheet = Sheet::open(&state);
        let before = sheet.draft.clone();
        sheet.step(-1, 1);
        sheet.step(99, 1);
        assert_eq!(sheet.draft, before);
    }

    #[test]
    fn save_applies_the_draft_to_the_live_state() {
        let mut state = AppState::new();
        let mut sheet = Sheet::open(&state);
        sheet.step(index(Row::LineNumbers), 1);
        sheet.step(index(Row::DefaultProfile), 1);
        sheet.step(index(Row::RecordOtherPrograms), 1);
        sheet.step(index(Row::EditableLimit), -1);
        sheet.save(&mut state);

        assert!(!state.show_gutter);
        assert_eq!(state.default_profile, bp_security::Profile::Private);
        assert!(state.record_other_programs);
        assert_eq!(state.editable_limit, 16 * 1024 * 1024);
    }

    #[test]
    fn every_row_has_a_label_and_a_value() {
        let sheet = Sheet::open(&AppState::new());
        let rows = sheet.rows();
        assert_eq!(rows.len(), ROWS.len());
        for row in rows {
            assert!(!row.label.is_empty() && !row.value.is_empty(), "{row:?}");
        }
    }
}
