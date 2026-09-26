#!/usr/bin/env bash
# Builds the Linux release packages from one release binary (release plan
# M3-02): a .deb, an .rpm, a relocatable .tar.gz, the Arch PKGBUILD for
# `bachelorpad-bin` rendered for that tarball, and the split debuginfo.
#
# Usage:
#   packaging/linux/build-packages.sh [--out DIR] [--no-build]
#                                     [--require-notices] [-- CARGO-BUILD-ARGS...]
#
#   --out DIR          where the outputs go (default: <target>/linux-packages)
#   --no-build         package the binary already at <target>/release/bachelorpad
#                      instead of building it; it must still carry its symbols
#                      (built with `--config profile.release.strip=false`)
#   --require-notices  fail, rather than warn, when THIRD-PARTY-NOTICES.txt is
#                      missing at the repository root (a release sets this)
#   -- ARGS            passed to `cargo build` after the script's own flags
#
# <target> is $CARGO_TARGET_DIR, else the repository's target/. The version
# is the workspace's, read through cargo; nothing here names one.
#
# Outputs, in DIR:
#   bachelorpad_<v>-1_amd64.deb          (<v> with a pre-release's "-" as "~")
#   bachelorpad-<v>-1.x86_64.rpm         (likewise)
#   bachelorpad-<v>-x86_64-linux.tar.gz
#   bachelorpad-<v>-x86_64-linux.debug   the split debuginfo; every package
#                                        ships the stripped binary, whose
#                                        .gnu_debuglink names "bachelorpad.debug"
#   PKGBUILD                             bachelorpad-bin, with the tarball's sha256
#
# Needs: cargo, cargo-deb, dpkg-deb, dpkg-shlibdeps, rpmbuild, objcopy,
# readelf, tar, gzip, sha256sum. Dockerfile.release has all of them.
#
# Exit status: 0 every output written and cross-checked; 1 a check failed;
# 2 a prerequisite or an argument was wrong.

set -euo pipefail

here=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
repo=$(cd -- "$here/../.." && pwd)

app_id=io.github.dboles99.BachelorPad
name=bachelorpad
# The package revision: bumped when the packaging changes but the program
# does not. Both formats call it the release and both start at 1.
pkg_release=1

out=
build=1
require_notices=0
cargo_args=()

die() { echo "build-packages: $*" >&2; exit 2; }
fail() { echo "build-packages: FAIL: $*" >&2; exit 1; }
note() { echo "build-packages: $*"; }

while (($#)); do
    case $1 in
        --out) out=${2:?--out needs a directory}; shift 2 ;;
        --no-build) build=0; shift ;;
        --require-notices) require_notices=1; shift ;;
        --) shift; cargo_args=("$@"); break ;;
        -h | --help) sed -n '2,/^$/s/^# \{0,1\}//p' "$0"; exit 0 ;;
        *) die "unknown argument: $1" ;;
    esac
done

for tool in cargo dpkg-deb dpkg-shlibdeps rpmbuild objcopy readelf tar gzip sha256sum; do
    command -v "$tool" >/dev/null || die "$tool is not on PATH (see Dockerfile.release for the full set)"
done
cargo deb --version >/dev/null 2>&1 || die "cargo-deb is not installed (cargo install --locked cargo-deb@3.8.0)"

cd "$repo"

target=${CARGO_TARGET_DIR:-target}
# A relative CARGO_TARGET_DIR is relative to where cargo runs, which is here.
[[ $target == /* ]] || target=$repo/$target
out=${out:-$target/linux-packages}
mkdir -p "$out"
out=$(cd -- "$out" && pwd)

# `cargo pkgid` prints e.g. `path+file:///src/apps/bachelorpad#0.1.0`; the
# version is what follows the last `#` or `@`.
pkgid=$(cargo pkgid --offline -p "$name" 2>/dev/null || cargo pkgid -p "$name")
version=${pkgid##*[#@]}
[[ $version =~ ^[0-9]+\.[0-9]+\.[0-9]+([-+.][0-9A-Za-z.+-]*)?$ ]] || die "could not read the version from '$pkgid'"
# A pre-release (1.0.0-rc.1) must sort before its release in every package
# manager, and none of them reads a semver hyphen that way: dpkg and rpm
# spell it with a tilde, pacman (which forbids hyphens) with an underscore.
# The tarball, the tag and the binary keep the upstream spelling.
pkg_version=${version/-/\~}
arch_version=${version//-/_}
note "version $version (deb/rpm $pkg_version, pacman $arch_version)"

# Packages and the tarball take their timestamps from the commit being
# built, so two builds of one commit differ only where the compiler does.
if [[ -z ${SOURCE_DATE_EPOCH:-} ]]; then
    SOURCE_DATE_EPOCH=$(git -C "$repo" log -1 --format=%ct 2>/dev/null || date +%s)
fi
export SOURCE_DATE_EPOCH

# --- the binary ---------------------------------------------------------------
binary=$target/release/$name
if ((build)); then
    # strip=false keeps the symbols for the split below; the profile's own
    # strip would discard them before anything could keep them.
    cargo build --release --locked -p "$name" --config profile.release.strip=false "${cargo_args[@]}"
fi
[[ -x $binary ]] || die "no binary at $binary (build it, or drop --no-build)"
# Captured first rather than piped into `grep -q`: grep stops at the first
# match, readelf takes SIGPIPE, and pipefail turns a found section into a
# failed check.
sections=$(readelf -S --wide "$binary")
if [[ $sections != *' .debug_line '* ]]; then
    die "$binary carries no debuginfo -- build it with --config profile.release.strip=false, so the symbols can be split off and kept"
fi

# Everything built here lives under the target directory: cargo-deb resolves
# a `target/release/...` asset path to the real target directory, so this is
# where the Cargo.toml metadata can find the stripped binary and licences.
work=$target/release/linux-package
rm -rf "$work"
mkdir -p "$work/licenses"

# objcopy's split, as .cargo/config.toml and the release profile describe:
# the symbols go to bachelorpad.debug, the shipped binary is stripped and
# records that file's name and CRC in .gnu_debuglink, so gdb, addr2line and
# std's backtrace find it when it sits beside the binary or under
# /usr/lib/debug/usr/bin/.
objcopy --only-keep-debug --compress-debug-sections=zlib "$binary" "$work/$name.debug"
objcopy --strip-all --add-gnu-debuglink="$work/$name.debug" "$binary" "$work/$name"
chmod 0755 "$work/$name"
debug_out=$out/$name-$version-x86_64-linux.debug
install -m 0644 "$work/$name.debug" "$debug_out"
note "debuginfo: $(du -h "$debug_out" | cut -f1)  stripped binary: $(du -h "$work/$name" | cut -f1)"

# --- licences ----------------------------------------------------------------
# ADR-0031: LICENSE-MIT, LICENSE-APACHE and THIRD-PARTY-NOTICES.txt ship in every
# package. The first two are required; the notices come from `cargo about`
# (release plan M1-14) and are only a warning until a release asks for them.
for f in LICENSE-MIT LICENSE-APACHE; do
    [[ -f $repo/$f ]] || fail "$f is missing at the repository root; every package ships it (ADR-0031)"
    install -m 0644 "$repo/$f" "$work/licenses/$f"
done
if [[ -f $repo/THIRD-PARTY-NOTICES.txt ]]; then
    install -m 0644 "$repo/THIRD-PARTY-NOTICES.txt" "$work/licenses/THIRD-PARTY-NOTICES.txt"
elif ((require_notices)); then
    fail "THIRD-PARTY-NOTICES.txt is missing at the repository root (generate it with cargo about, release plan M1-14)"
else
    echo "build-packages: WARNING: THIRD-PARTY-NOTICES.txt is missing; the packages ship without it" >&2
fi

# --- the staged tree -----------------------------------------------------------
# One tree, laid out as it is installed under /usr, is the reference every
# format is built from or checked against: the tarball and the .rpm are made
# from it, and the .deb (whose contents Cargo.toml declares) must match it.
root=$work/root
doc=$root/usr/share/doc/$name
install -D -m 0755 "$work/$name" "$root/usr/bin/$name"
# M2-16: `bpad` is the short name, a relative symlink so it survives a
# relocated tree.
ln -s "$name" "$root/usr/bin/bpad"
install -D -m 0644 "packaging/linux/$app_id.desktop" "$root/usr/share/applications/$app_id.desktop"
install -D -m 0644 "packaging/linux/$app_id.metainfo.xml" "$root/usr/share/metainfo/$app_id.metainfo.xml"
install -D -m 0644 "packaging/linux/$app_id.xml" "$root/usr/share/mime/packages/$app_id.xml"
for icon in assets/icon/hicolor/*/apps/"$app_id".*; do
    rel=${icon#assets/icon/hicolor/}
    install -D -m 0644 "$icon" "$root/usr/share/icons/hicolor/$rel"
done
install -D -m 0644 packaging/linux/copyright "$doc/copyright"
for f in "$work"/licenses/*; do
    install -m 0644 "$f" "$doc/"
done

# The desktop files are generated and tested in bp-platform; validating the
# staged copies here catches a validator that has grown stricter since.
# Optional outside the container, which has both tools.
if command -v desktop-file-validate >/dev/null; then
    desktop-file-validate "$root/usr/share/applications/$app_id.desktop" || fail "desktop-file-validate rejected the desktop entry"
fi
if command -v appstreamcli >/dev/null; then
    appstreamcli validate --no-net --explain "$root/usr/share/metainfo/$app_id.metainfo.xml" >/dev/null \
        || fail "appstreamcli rejected the metainfo (run it by hand for the reasons)"
fi

# The canonical listing: every path under usr/, with a trailing "@" for a
# symlink, sorted in the C locale so the comparisons below are exact.
list_tree() { (cd "$1" && find usr \( -type f -printf '%p\n' \) -o \( -type l -printf '%p@\n' \)) | LC_ALL=C sort; }
list_tree "$root" > "$work/expected.txt"

# --- .deb ----------------------------------------------------------------------
deb=$out/${name}_${pkg_version}-${pkg_release}_amd64.deb
rm -f "$deb"
cargo deb -p "$name" --no-build --no-strip --deb-version "$pkg_version-$pkg_release" --output "$deb"
[[ -f $deb ]] || fail "cargo-deb wrote no package at $deb"
# cargo-deb adds a changelog only when one is configured; everything else in
# the .deb must be exactly the staged tree.
dpkg-deb --fsys-tarfile "$deb" | tar -t -v | awk '{ p = $6; sub(/^\.\//, "", p); if ($1 ~ /^l/) p = p "@"; if ($1 !~ /^d/) print p }' \
    | LC_ALL=C sort > "$work/deb.txt"
if ! diff -u "$work/expected.txt" "$work/deb.txt"; then
    fail "the .deb's files differ from the staged tree (above: - staged, + .deb); update [package.metadata.deb] assets in apps/bachelorpad/Cargo.toml"
fi
[[ $(dpkg-deb --fsys-tarfile "$deb" | tar -x -O ./usr/bin/"$name" | sha256sum) == "$(sha256sum < "$root/usr/bin/$name")" ]] \
    || fail "the .deb's bachelorpad is not the stripped binary"
note "deb: $deb"
dpkg-deb -f "$deb" Depends Recommends | sed 's/^/build-packages:   /'

# --- .rpm ----------------------------------------------------------------------
# rpmbuild from a generated spec, not cargo-generate-rpm: that tool cannot
# put a symlink in a package (every asset goes through the rpm crate's
# with_file, which reads the link's target), so `bpad` would ship as a
# second copy of the binary. rpmbuild's own dependency generator also adds
# the soname and GLIBC_x.y requirements from the binary.
rpmtop=$work/rpmbuild
mkdir -p "$rpmtop"/{BUILD,RPMS,SPECS,SRPMS,BUILDROOT}
sed -e "s|@VERSION@|$pkg_version|g" -e "s|@RELEASE@|$pkg_release|g" \
    "$here/bachelorpad.spec.in" > "$rpmtop/SPECS/$name.spec"
if [[ ! -f $repo/THIRD-PARTY-NOTICES.txt ]]; then
    sed -i '/THIRD-PARTY-NOTICES.txt/d' "$rpmtop/SPECS/$name.spec"
fi
# --nodeps: rpmbuild checks build requirements against an rpm database, and
# a Debian build host has none (the spec declares no BuildRequires anyway).
# xz rather than rpmbuild's gzip default: a third smaller, and read by every
# rpm since 4.6 (zstd, Fedora's own choice, needs 4.14 and a zstd-enabled
# build on the reading side).
rpmbuild -bb --nodeps --target x86_64 \
    --define "_topdir $rpmtop" \
    --define "_stage $root" \
    --define "_buildhost bachelorpad-release" \
    --define "use_source_date_epoch_as_buildtime 1" \
    --define "clamp_mtime_to_source_date_epoch 1" \
    --define "_build_id_links none" \
    --define "_binary_payload w9.xzdio" \
    "$rpmtop/SPECS/$name.spec" > "$work/rpmbuild.log" 2>&1 \
    || { cat "$work/rpmbuild.log" >&2; fail "rpmbuild failed"; }
rpm_built=$rpmtop/RPMS/x86_64/$name-$pkg_version-$pkg_release.x86_64.rpm
[[ -f $rpm_built ]] || { cat "$work/rpmbuild.log" >&2; fail "rpmbuild wrote no $rpm_built"; }
rpm_out=$out/$name-$pkg_version-$pkg_release.x86_64.rpm
cp "$rpm_built" "$rpm_out"
# The .rpm moves the licences to /usr/share/licenses/, as Fedora does; map
# them back before comparing with the staged tree.
rpm -qp --qf '[%{FILENAMES}\t%{FILEMODES:perms}\n]' "$rpm_out" 2>/dev/null \
    | awk -F'\t' '{ p = substr($1, 2); if ($2 ~ /^d/) next; if ($2 ~ /^l/) p = p "@"; print p }' \
    | sed "s|^usr/share/licenses/$name/|usr/share/doc/$name/|" | LC_ALL=C sort > "$work/rpm.txt"
if ! diff -u "$work/expected.txt" "$work/rpm.txt"; then
    fail "the .rpm's files differ from the staged tree (above: - staged, + .rpm); update packaging/linux/bachelorpad.spec.in"
fi
note "rpm: $rpm_out"
rpm -qp --requires "$rpm_out" 2>/dev/null | sed 's/^/build-packages:   requires /'

# --- .tar.gz -------------------------------------------------------------------
# A relocatable tree: bin/ and share/ as they would sit under a prefix, the
# licences at the top where someone unpacking it looks, and install.sh.
tarname=$name-$version-x86_64-linux
tree=$work/tar/$tarname
mkdir -p "$tree"
cp -a "$root/usr/bin" "$root/usr/share" "$tree/"
for f in "$work"/licenses/*; do
    install -m 0644 "$f" "$tree/"
done
install -m 0755 "$here/install.sh" "$tree/install.sh"
tarball=$out/$tarname.tar.gz
# Sorted, owned by root, stamped with the commit time and gzip without a
# name or time in its header: the same inputs give the same bytes.
tar -C "$work/tar" --sort=name --mtime="@$SOURCE_DATE_EPOCH" \
    --owner=0 --group=0 --numeric-owner --format=gnu -cf - "$tarname" \
    | gzip -n -9 > "$tarball"
note "tar.gz: $tarball"

# --- PKGBUILD ------------------------------------------------------------------
tar_sha=$(sha256sum "$tarball" | cut -d' ' -f1)
sed -e "s|@VERSION@|$version|g" -e "s|@PKGVER@|$arch_version|g" \
    -e "s|@RELEASE@|$pkg_release|g" -e "s|@SHA256@|$tar_sha|g" \
    "$here/PKGBUILD.in" > "$out/PKGBUILD"
grep -q '@[A-Z0-9]*@' "$out/PKGBUILD" && fail "PKGBUILD.in has a placeholder this script does not fill"
note "PKGBUILD: $out/PKGBUILD (sha256 $tar_sha)"

note "done: $(find "$out" -maxdepth 1 -type f -printf '%f ' | tr -s ' ')"
