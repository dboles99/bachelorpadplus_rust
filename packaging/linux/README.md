# Linux packaging

Everything that turns a release build into what Linux users install
(release plan M3-02 and the Linux half of M2-16): a `.deb` for Debian 12 and
Ubuntu 22.04 or later, an `.rpm` for Fedora, a relocatable `.tar.gz`, and a
`PKGBUILD` for the AUR's `bachelorpad-bin` (decision 18; AppImage and Flatpak
come later). All of them are built in a pinned Debian 12 container, so the
binary runs on glibc 2.36 and newer (decision 19, exit criterion L2).

The application id is `io.github.dboles99.BachelorPad` (ADR-0032); every
desktop file and icon is named for it.

| File | What it is |
| --- | --- |
| `io.github.dboles99.BachelorPad.desktop` | The desktop entry: `Exec=bachelorpad %F`, resolved through `PATH` (generated) |
| `io.github.dboles99.BachelorPad.xml` | The shared-mime-info package defining the product's own type, `.bpadx` (generated) |
| `io.github.dboles99.BachelorPad.metainfo.xml` | AppStream metadata: id, licences, OARS rating, `provides/mediatype`, releases (generated) |
| `copyright` | `/usr/share/doc/bachelorpad/copyright`, in Debian's machine-readable format |
| `Dockerfile.release` | The build environment: Debian 12, Rust 1.94.1, cargo-deb 3.8.0, rpmbuild |
| `build-in-container.sh` | Builds the image if needed and runs the release build in it -- the one command a release runs |
| `release-entrypoint.sh` | The container's entrypoint: `cargo build --release --locked`, then the two scripts below |
| `build-packages.sh` | Splits the debuginfo and makes the `.deb`, `.rpm`, `.tar.gz` and `PKGBUILD` from one binary |
| `check-glibc.sh` | Prints the newest `GLIBC_x.y` a binary needs; fails above a ceiling (default 2.36) |
| `bachelorpad.spec.in` | The `.rpm`'s spec, filled in by `build-packages.sh` |
| `PKGBUILD.in` | The AUR `bachelorpad-bin` recipe, filled in with the version and the tarball's checksum |
| `install.sh` | Ships in the tarball: installs to `~/.local` (or `--prefix`), and `--uninstall`s |
| `test-packages.sh` | Installs, runs and removes the packages in Ubuntu 22.04/24.04, Fedora and Arch containers |

The `.deb`'s contents are declared in `apps/bachelorpad/Cargo.toml`
(`[package.metadata.deb]`).

## Building the packages

```sh
packaging/linux/build-in-container.sh --out artifacts/releases/<tag>/linux
```

This builds the image from `Dockerfile.release` (once; the tag is a hash of
the Dockerfile), mounts the repository **read-only** at `/src` and the
output directory writable at `/out`, and runs `release-entrypoint.sh` from
the mounted repository, which

1. runs `cargo build --release --locked -p bachelorpad` with
   `--config profile.release.strip=false` (the symbols are split off and
   kept, not discarded) and `--remap-path-prefix` for the container's fixed
   paths (`/usr/local/cargo` to `/cargo`, `/src` to `/bachelorpad`, the target
   directory to `/target`), which `.cargo/config.toml` leaves to the release
   build because only it knows the paths;
2. runs `check-glibc.sh` on the binary -- above 2.36 it stops before
   packaging anything;
3. runs `build-packages.sh --no-build --out /out`;
4. writes `build-environment.txt` (rustc, cargo, cargo-deb, rpmbuild, glibc,
   OS) for the release manifest.

Options: `--target-dir DIR` and `--cargo-cache DIR` keep the target
directory and the crate downloads between runs (by default in docker
volumes); `-- --require-notices` makes a missing `THIRD-PARTY-NOTICES.txt` an
error rather than a warning, which a release wants. `--base-image` and
`--extra-ca` exist only to prove the Dockerfile on a machine that cannot
reach Debian's mirrors or sits behind a TLS-intercepting proxy; a release is
built from the pinned Debian 12 image.

`build-packages.sh` also runs outside the container, to try the packaging
on a workstation -- but the binary it packages then needs the workstation's
glibc (2.39 on Ubuntu 24.04), and `check-glibc.sh` says so.

### What comes out

In the output directory, for version `<v>`:

| File | |
| --- | --- |
| `bachelorpad_<v>-1_amd64.deb` | Debian 12, Ubuntu 22.04 and later |
| `bachelorpad-<v>-1.x86_64.rpm` | Fedora (and openSUSE: the requirements are sonames) |
| `bachelorpad-<v>-x86_64-linux.tar.gz` | Any glibc 2.36+ distribution: `bin/`, `share/`, the licences and `install.sh` |
| `PKGBUILD` | `bachelorpad-bin`, downloading the tarball from the GitHub release `v<v>` |
| `bachelorpad-<v>-x86_64-linux.debug` | The split debuginfo, for the release archive -- in no package |
| `glibc-required.txt` | The newest `GLIBC_x.y` the binary needs, from `check-glibc.sh` (container runs only) |
| `build-environment.txt` | What built it (container runs only) |

A pre-release keeps its upstream spelling in the tarball and the tag, and is
respelt where a package manager would otherwise sort it after the release:
`1.0.0-rc.1` is `1.0.0~rc.1` for dpkg and rpm and `1.0.0_rc.1` for pacman.

Every format is built from, or checked against, one staged tree:
`build-packages.sh` fails if the `.deb` or the `.rpm` installs a single file
more or less than the tarball carries, so the Cargo.toml asset list and the
spec cannot drift apart. The tarball is reproducible for a given binary
(sorted, root-owned, stamped with the commit time, `gzip -n`).

### Why rpmbuild and not cargo-generate-rpm

The plan named `cargo-generate-rpm` (or nfpm). Its 0.21.0 cannot put a
symlink in a package: every asset goes through the rpm crate's `with_file`,
which follows the link, so `bpad` would ship as a second 20 MB copy of the
binary. rpmbuild is Debian-packaged (`rpm`), handles the symlink, and its
dependency generator adds the linked sonames with their `GLIBC_x.y` versions,
which the rpm crate's builder does not. nfpm would also do, but it is a Go
binary from GitHub releases rather than something pinned through cargo or
apt. cargo-deb stays for the `.deb`: it does symlinks and `$auto`.

### Debuginfo

The binary is built with its line tables, and objcopy splits them off
(`--only-keep-debug`, then `--strip-all --add-gnu-debuglink`). Every package
ships the stripped binary, whose `.gnu_debuglink` names `bachelorpad.debug`;
the debug file is kept beside the outputs, compressed, for M3-03 to archive.
To read a crash address from a shipped binary, put that file next to the
binary, or at `/usr/lib/debug/usr/bin/`, as `bachelorpad.debug`; gdb,
`addr2line` and std's backtrace then find it (they check the CRC the link
records).

## What the packages install

```text
/usr/bin/bachelorpad
/usr/bin/bpad -> bachelorpad                          (M2-16)
/usr/share/applications/io.github.dboles99.BachelorPad.desktop
/usr/share/metainfo/io.github.dboles99.BachelorPad.metainfo.xml
/usr/share/mime/packages/io.github.dboles99.BachelorPad.xml
/usr/share/icons/hicolor/<N>x<N>/apps/io.github.dboles99.BachelorPad.png  (N = 16 ... 512)
/usr/share/icons/hicolor/scalable/apps/io.github.dboles99.BachelorPad.svg
/usr/share/doc/bachelorpad/copyright
/usr/share/doc/bachelorpad/LICENSE-MIT, LICENSE-APACHE, THIRD-PARTY-NOTICES.txt
```

The `.rpm` and the AUR package put the licence files under
`/usr/share/licenses/<name>/` instead, as those distributions do. ADR-0031
has every package ship `LICENSE-MIT`, `LICENSE-APACHE` and
`THIRD-PARTY-NOTICES.txt`: `build-packages.sh` stops if either licence file is
missing from the repository root, and warns (or, with `--require-notices`,
stops) if the notices are.

### Dependencies

What the binary **links** the tools find: cargo-deb's `$auto` runs
`dpkg-shlibdeps` (giving `libc6` at the version its symbols need and
`libfontconfig1`, which brings `libfreetype6`; cargo-deb drops `libgcc-s1`,
which `libc6` depends on anyway), and rpmbuild's generator adds the same
as sonames with `GLIBC_x.y` versions.
What it loads with **dlopen** no tool can see. That list came from the
sonames embedded in a release binary (`strings bachelorpad | grep -o
'lib[A-Za-z0-9_+-]*\.so[.0-9]*'`) and the crate that asks for each:

| Soname | Loaded by | Debian / Ubuntu | Fedora (as a soname) | Arch |
| --- | --- | --- | --- | --- |
| `libxkbcommon.so.0` | winit (keyboard) | `libxkbcommon0` | `libxkbcommon.so.0()(64bit)` | `libxkbcommon` |
| `libxkbcommon-x11.so.0` | winit (X11 keyboard) | `libxkbcommon-x11-0` | `libxkbcommon-x11.so.0()(64bit)` | `libxkbcommon-x11` |
| `libX11.so.6`, `libX11-xcb.so.1`, `libxcb.so.1` | winit (X11) | `libx11-6`, `libx11-xcb1`, `libxcb1` | sonames | `libx11`, `libxcb` |
| `libXcursor.so.1`, `libXi.so.6` | winit (X11) | `libxcursor1`, `libxi6` | sonames | `libxcursor`, `libxi` |
| `libwayland-client.so.0` | winit (Wayland) | `libwayland-client0` | soname | `wayland` |
| `libdbus-1.so.3` | rfd's portal backend (file dialogs) | `libdbus-1-3` | soname | `dbus` |
| `libEGL.so.1`, `libGL.so.1`, `libwayland-egl.so.1` | glutin, only for `--renderer platform` | Recommends `libegl1`, `libgl1`, `libwayland-egl1` | Recommends | optdepends `libglvnd` |

`libXrender.so.1` is named too (Xcursor needs it); `libxcursor1` depends on
it on every distribution, so it is not listed. The plan's `freetype` is not
linked by the binary itself -- fontconfig brings it -- so it is not listed
either, and `dbus` means the client library, since rfd talks D-Bus through
libdbus.

**Recommends** `xdg-desktop-portal-gtk | xdg-desktop-portal-kde |
xdg-desktop-portal-gnome` (the rpm uses a rich `or` dependency, the AUR
package optdepends): the file pickers go through the portal, and message
dialogs are drawn in the window (L1), so nothing else needs a toolkit. A
Recommends rather than a Depends, because every desktop with a portal
already has a backend, and the editor runs without one -- only Open and
Save As fail, and say so.

### No maintainer scripts

The desktop database, the MIME database and the icon cache are refreshed by
the distributions' own triggers on the directories these files go to:
`desktop-file-utils`, `shared-mime-info` and `hicolor-icon-theme` declare dpkg
file triggers (`interest-noawait /usr/share/applications`, `.../mime/packages`,
`.../icons/hicolor`) on Debian and Ubuntu, Fedora's packages declare rpm file
triggers, and Arch's are pacman hooks. A postinst running
`update-mime-database` itself would do the same work twice, and on a system
without `shared-mime-info` would fail the install for nothing. So the
`.deb`, the `.rpm` and the PKGBUILD carry no scripts.

## The tarball

`bachelorpad-<v>-x86_64-linux/` holds `bin/` (`bachelorpad`, the `bpad`
symlink), `share/` laid out as under `/usr`, the licence files, and
`install.sh`:

```sh
tar xzf bachelorpad-<v>-x86_64-linux.tar.gz
cd bachelorpad-<v>-x86_64-linux
./install.sh                     # to ~/.local; no root needed
sudo ./install.sh --prefix /usr/local
./install.sh --uninstall         # removes exactly what it installed
```

It refreshes the MIME and desktop databases for that prefix where the tools
are installed, and if the prefix's `bin/` is not on `PATH`, writes the full
path into the installed menu entry. The runtime libraries are the user's to
install; `./install.sh --help` lists them for Debian and Ubuntu.

A `~/.local` install puts the menu entry where File ▸ Set as Default Editor
would write its per-user copy. The editor treats it as a file it did not
write (it carries no marker) and leaves it alone rather than overwrite it;
setting the default application still works.

## Testing the packages

```sh
packaging/linux/test-packages.sh --dir artifacts/releases/<tag>/linux
```

For each of Ubuntu 22.04, Ubuntu 24.04, Fedora and Arch (`--distro` picks):
installs the package with its hard dependencies only (no Recommends, no weak
dependencies, so a missing one shows), runs `bachelorpad --self-check` under
`xvfb-run` and `bpad --version`, opens the window under Xvfb for five
seconds (winit dlopens its libraries only then, which `--self-check` cannot
see), and validates the installed desktop entry; then removes the package
and fails if any of its files, or the generated MIME entry, is left. On
Arch it builds the rendered PKGBUILD with `makepkg` against the local
tarball.

## These files are generated -- do not edit them

The `.desktop`, MIME and metainfo files come from
`bp_platform::editor::desktop::packaged_assets()`, built from
`AppInfo::packaged()` and `PACKAGED_PRESET` (Notepad Replacement: plain
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

The icon set lives in `assets/icon/`: hicolor PNGs from 16 to 512 pixels and
a scalable SVG, all named `io.github.dboles99.BachelorPad` -- the name the
`.desktop` entry's `Icon=` and the metainfo's `<icon type="stock">` look up.

## Validation

```sh
desktop-file-validate packaging/linux/io.github.dboles99.BachelorPad.desktop
appstreamcli validate --no-net packaging/linux/io.github.dboles99.BachelorPad.metainfo.xml
```

Both pass. `appstreamcli --pedantic` adds one note,
`cid-contains-uppercase-letter`: the capitals in `BachelorPad` are the
decided id (ADR-0032), and the note is pedantic-only.
