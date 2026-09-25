# Linux packaging assets

The static files a Linux package installs so that BachelorPad+ appears in
the application menu, in Open With, and in software centres. The application
id is `io.github.dboles99.BachelorPad` (ADR-0032); every file here is named
for it, and so are the icons.

| File | What it is | Installed to |
| --- | --- | --- |
| `io.github.dboles99.BachelorPad.desktop` | The desktop entry: `Exec=bachelorpad %F`, resolved through `PATH` | `/usr/share/applications/` |
| `io.github.dboles99.BachelorPad.xml` | The shared-mime-info package defining the product's own type (`.bpadx`) | `/usr/share/mime/packages/` |
| `io.github.dboles99.BachelorPad.metainfo.xml` | AppStream metadata: id, licences, OARS rating, `provides/mediatype`, releases | `/usr/share/metainfo/` |

## These files are generated -- do not edit them

All three come from `bp_platform::editor::desktop::packaged_assets()`, built
from `AppInfo::packaged()` and `PACKAGED_PRESET` (Notepad Replacement: plain
text, logs and `.bpadx`). The test
`crates/bp-platform/tests/linux_assets.rs` fails if a shipped file differs
from the generated one by a single byte. To change one, change the generator
in `crates/bp-platform/src/editor/desktop.rs`, then regenerate:

```sh
cargo run -p bp-platform --example regenerate_linux_assets
```

and commit the result. A release adds a line to `desktop::RELEASES` first; a
test fails until the newest entry there is the workspace version.

The claim is kept to the Notepad Replacement set on purpose. When the
packaged entry is installed, File ▸ Set as Default Editor writes no per-user
copy of it, so the packaged `MimeType=` line is the whole claim -- and a
package claiming `.csv` or `.html` would become their fallback handler on any
desktop where nothing else is set, which is choosing for the user.

## Icons

Not here. The icon set lives in `assets/icon/`: hicolor PNGs from 16 to 512
pixels and a scalable SVG, all named `io.github.dboles99.BachelorPad`. A
package script installs them as

```text
/usr/share/icons/hicolor/<N>x<N>/apps/io.github.dboles99.BachelorPad.png   (N = 16 ... 512)
/usr/share/icons/hicolor/scalable/apps/io.github.dboles99.BachelorPad.svg
```

which is the name the `.desktop` entry's `Icon=` and the metainfo's
`<icon type="stock">` look up.

## After installing

A package's post-install step (usually a trigger the distribution already
provides) runs `update-desktop-database -q /usr/share/applications`,
`update-mime-database /usr/share/mime` and
`gtk-update-icon-cache -q /usr/share/icons/hicolor`.

## Validation

```sh
desktop-file-validate packaging/linux/io.github.dboles99.BachelorPad.desktop
appstreamcli validate --no-net packaging/linux/io.github.dboles99.BachelorPad.metainfo.xml
```

Both pass. `appstreamcli --pedantic` adds one note,
`cid-contains-uppercase-letter`: the capitals in `BachelorPad` are the
decided id (ADR-0032), and the note is pedantic-only.
