//! What the passphrase bar is currently asking for, and what to do with the
//! answer.
//!
//! A small state machine rather than a flag, because the same one-field bar
//! serves three questions and answering the wrong one is expensive: unlocking
//! with a typo costs a retry, while *setting* one with a typo costs the
//! document, permanently and with no way to notice until it is too late.
//!
//! That is the whole reason [`Ask::Confirm`] exists. Encrypting asks twice and
//! compares, using one field and two submits rather than two fields, which
//! keeps the bar a bar.

use std::path::PathBuf;

use bp_core::DocumentId;
use zeroize::Zeroizing;

/// Said on the bar when Encrypt Document first asks for a passphrase.
///
/// Release plan decision 20 (D4, option b): RC1 ships with the `.bpadx`
/// envelope not yet independently reviewed, and says so where somebody is
/// about to rely on it as well as in `SECURITY.md`. One short sentence,
/// because the bar is a status line.
pub(crate) const UNREVIEWED_FORMAT: &str =
    "this encryption format has not yet had an independent security review";

/// What the bar is asking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Ask {
    /// A `.bpadx` file was opened and needs its passphrase before it can
    /// become a tab. The path is held here rather than opened first, because
    /// a document nobody can read should not appear as an empty tab.
    ///
    /// `agreed_cost` records that the user agreed to an unusually expensive
    /// unlock (decision 15). It travels with the question for the reason
    /// [`Ask::Confirm`]'s first entry does: it cannot outlive it, so an
    /// agreement about one file is never applied to the next.
    Unlock { path: PathBuf, agreed_cost: bool },
    /// Encrypting the active document: the first of two entries.
    Set { id: DocumentId, target: PathBuf },
    /// The second entry, compared against the first.
    ///
    /// The first passphrase travels in this variant rather than in a separate
    /// field, so it cannot outlive the question it belongs to -- when the ask
    /// is dropped or replaced, so is the passphrase.
    Confirm {
        id: DocumentId,
        target: PathBuf,
        first: Zeroizing<String>,
    },
    /// Sign Document with the key already kept: its passphrase unseals it
    /// (release plan M2-10). `document` is the file on disk that is signed.
    Sign { document: PathBuf },
    /// Sign Document with no key yet: the first of two entries of the new
    /// key's passphrase.
    NewKey { document: PathBuf },
    /// The second entry, compared against the first -- for the reason
    /// [`Ask::Confirm`] exists, and more so: a mistyped key passphrase costs
    /// the signing identity rather than one document.
    ConfirmKey {
        document: PathBuf,
        first: Zeroizing<String>,
    },
}

impl Ask {
    /// The words on the bar.
    pub(crate) fn prompt(&self) -> &'static str {
        match self {
            Self::Unlock { .. } => "Passphrase",
            Self::Set { .. } => "New passphrase",
            // Named differently on purpose. "Passphrase" twice gives no sign
            // that the first entry was accepted, and a user who thinks the
            // first did not register types something else the second time.
            Self::Confirm { .. } => "Confirm passphrase",
            Self::Sign { .. } => "Signing key passphrase",
            Self::NewKey { .. } => "New signing key passphrase",
            Self::ConfirmKey { .. } => "Confirm signing key passphrase",
        }
    }

    pub(crate) fn action(&self) -> &'static str {
        match self {
            Self::Unlock { .. } => "Unlock",
            Self::Set { .. } => "Next",
            Self::Confirm { .. } => "Encrypt",
            Self::Sign { .. } => "Sign",
            Self::NewKey { .. } => "Next",
            Self::ConfirmKey { .. } => "Create Key and Sign",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `DocumentId` is `bp-core`'s to mint, so a real workspace supplies one
    /// rather than this module inventing a value the type does not permit.
    fn an_id() -> DocumentId {
        let mut workspace = bp_core::Workspace::new();
        workspace.open_new(time::OffsetDateTime::UNIX_EPOCH)
    }

    fn set() -> Ask {
        Ask::Set {
            id: an_id(),
            target: PathBuf::from("/notes/a.bpadx"),
        }
    }

    #[test]
    fn the_two_entries_of_a_new_passphrase_are_worded_differently() {
        // A user who sees "Passphrase" twice has no sign the first entry
        // registered, and types something else the second time -- which turns
        // a working encryption into a mismatch they cannot explain.
        let confirm = Ask::Confirm {
            id: an_id(),
            target: PathBuf::from("/notes/a.bpadx"),
            first: Zeroizing::new("first".to_owned()),
        };
        assert_ne!(set().prompt(), confirm.prompt());
        assert_ne!(set().action(), confirm.action());
    }

    #[test]
    fn unlocking_and_setting_do_not_look_the_same() {
        // The two have opposite consequences for a typo, so they must not be
        // mistakable for each other.
        let unlock = Ask::Unlock {
            path: PathBuf::from("/a.bpadx"),
            agreed_cost: false,
        };
        assert_ne!(unlock.prompt(), set().prompt());
        assert_eq!(unlock.action(), "Unlock");
    }
}
