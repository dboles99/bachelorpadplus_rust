//! Which date an opened file was created on, for naming purposes.
//!
//! Locked decision 4: the creation date is the default filename date and
//! does not change on every save. For a document opened from disk that means
//! recovering the date it was *really* created, not stamping it with the
//! moment it was opened -- otherwise renaming last month's note gives it
//! today's date, and every open quietly rewrites its history.
//!
//! This module only decides between candidate dates. It never reads the
//! filesystem or the clock: the shell reads the file's metadata, converts
//! the timestamps to local dates, and falls back to today itself when this
//! returns `None`.

use time::Date;

use crate::name::SemanticName;

/// The date to treat as `file_name`'s creation date, from the best evidence
/// available, or `None` when there is none and the caller must use today.
///
/// In order:
///
/// 1. **The date in a semantic filename** (`Title_16AUG2026.txt`). The name
///    wins over the filesystem because it is the more durable record.
///    Copying a file, extracting it from an archive, restoring it from a
///    backup or syncing it to another machine all give it a new birth time,
///    but none of them change its name -- and the name is what the user
///    sees, what this crate's rename grammar preserves, and very likely a
///    date this program wrote in the first place. Only a name that parses
///    in the full grammar counts; a name that merely contains something
///    date-like is prose, not a record.
/// 2. **The filesystem creation time**, unless the modification time is
///    earlier. A file cannot have been changed before it existed, so a
///    birth time after the last write is itself evidence of a copy (Windows
///    keeps the modification time on copy but resets the creation time),
///    and the earlier date is the truer one.
/// 3. **The modification time**, for filesystems that do not record a birth
///    time at all -- common on Linux, where `Metadata::created` is an error
///    rather than a date. It is an upper bound on the creation date rather
///    than the date itself, but a far better one than today.
pub fn creation_date_for(
    file_name: &str,
    created: Option<Date>,
    modified: Option<Date>,
) -> Option<Date> {
    if let Some(name) = SemanticName::parse(file_name) {
        return Some(name.date());
    }
    match (created, modified) {
        (Some(created), Some(modified)) => Some(created.min(modified)),
        (created, modified) => created.or(modified),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::date;

    const IN_NAME: Date = date!(2026 - 03 - 02);
    const CREATED: Date = date!(2026 - 05 - 10);
    const MODIFIED: Date = date!(2026 - 07 - 21);

    #[test]
    fn the_date_in_a_semantic_name_wins_over_both_timestamps() {
        assert_eq!(
            creation_date_for("Budget Notes_02MAR2026.txt", Some(CREATED), Some(MODIFIED)),
            Some(IN_NAME),
            "a copy or a sync resets birth times; the name survives both"
        );
    }

    #[test]
    fn a_semantic_name_with_suffixes_still_supplies_its_date() {
        assert_eq!(
            creation_date_for("Budget Notes_v3_02MAR2026_02.md", Some(CREATED), None),
            Some(IN_NAME)
        );
    }

    #[test]
    fn a_plain_name_uses_the_filesystem_creation_time() {
        assert_eq!(
            creation_date_for("notes.txt", Some(CREATED), Some(MODIFIED)),
            Some(CREATED)
        );
    }

    #[test]
    fn without_a_birth_time_the_modification_time_is_used() {
        // Many Linux filesystems cannot report a birth time at all.
        assert_eq!(
            creation_date_for("notes.txt", None, Some(MODIFIED)),
            Some(MODIFIED)
        );
    }

    #[test]
    fn a_birth_time_after_the_last_write_gives_way_to_the_write() {
        // Windows resets the creation time on copy but keeps the
        // modification time, so the file appears born after it was edited.
        assert_eq!(
            creation_date_for("notes.txt", Some(MODIFIED), Some(CREATED)),
            Some(CREATED),
            "nothing is modified before it exists; the earlier date is the truer one"
        );
    }

    #[test]
    fn with_no_evidence_there_is_no_date() {
        assert_eq!(
            creation_date_for("notes.txt", None, None),
            None,
            "the shell falls back to today; this crate never reads the clock"
        );
        assert_eq!(creation_date_for("", None, None), None);
    }

    #[test]
    fn a_date_like_name_outside_the_grammar_is_not_trusted() {
        for name in [
            "2026-03-02 meeting.txt",       // ISO date, not DDMMMYYYY
            "Budget Notes 02MAR2026.txt",   // space, not the underscore separator
            "Budget Notes_02MAR2026",       // no extension
            "Budget Notes_31FEB2026.txt",   // not a calendar date
            "Budget Notes_02MAR2026_1.txt", // non-canonical collision suffix
            "02MAR2026.txt",                // the date alone is not a titled name
        ] {
            assert_eq!(
                creation_date_for(name, Some(CREATED), Some(MODIFIED)),
                Some(CREATED),
                "{name:?} is not in the filename grammar, so its date is prose"
            );
        }
    }
}
