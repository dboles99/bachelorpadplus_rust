#!/usr/bin/env bash
# Entrypoint of the Linux release container (Dockerfile.release).
#
# Builds bachelorpad with `--release --locked` from the read-only repository
# at /src, then hands the binary to build-packages.sh and check-glibc.sh,
# which write the packages to /out. Run it through build-in-container.sh
# rather than by hand; that script sets up the mounts this one expects.
#
# Usage (inside the container):
#   release-entrypoint.sh [--glibc-ceiling X.Y] [--require-notices]
#
# Mounts:
#   /src           the repository, read-only
#   /out           the output directory, writable
#   /build/target  optional: a writable target directory to keep between runs
#
# Writes, besides build-packages.sh's outputs, glibc-required.txt (the newest
# GLIBC_x.y the binary needs) and build-environment.txt (the tool versions).
#
# Exit status: 0 packages written and every check passed; non-zero otherwise.

set -euo pipefail

src=/src
out=/out
ceiling=2.36
package_args=()

while (($#)); do
    case $1 in
        --glibc-ceiling) ceiling=${2:?--glibc-ceiling needs a version}; shift 2 ;;
        --require-notices) package_args+=(--require-notices); shift ;;
        -h | --help) sed -n '2,/^$/s/^# \{0,1\}//p' "$0"; exit 0 ;;
        *) echo "release-entrypoint: unknown argument: $1" >&2; exit 2 ;;
    esac
done

[[ -f $src/Cargo.toml ]] || { echo "release-entrypoint: no repository mounted at $src" >&2; exit 2; }
[[ -d $out && -w $out ]] || { echo "release-entrypoint: $out is not a writable directory" >&2; exit 2; }

# The repository belongs to the host user, not to root in here; without this
# git refuses to read it ("dubious ownership") and build-packages.sh loses
# the commit time. Run from / because a git worktree's `.git` file points at
# a host path that is not mounted, and git stops on it before reading
# `--global`. build-in-container.sh passes SOURCE_DATE_EPOCH for that case.
git -C / config --global --add safe.directory "$src"

target=${CARGO_TARGET_DIR:?}
cargo_home=${CARGO_HOME:?}

# Build paths leak into a release binary through panic locations (see
# .cargo/config.toml). In here they are fixed, so the remap can be too: the
# same two prefixes the config comment names, plus the target directory for
# the code Slint generates into OUT_DIR. `--config` adds to the configured
# rustflags rather than replacing them, as RUSTFLAGS would.
remap="target.'cfg(all())'.rustflags = [\
'--remap-path-prefix=$cargo_home=/cargo', \
'--remap-path-prefix=$src=/bachelorpad', \
'--remap-path-prefix=$target=/target']"

# Symbols are kept here (strip=false) so build-packages.sh can split them
# into a .debug file with objcopy and ship the stripped binary; the profile's
# own strip would throw them away before anything could keep them.
cd "$src"
cargo build --release --locked -p bachelorpad \
    --config profile.release.strip=false \
    --config "$remap"

# The glibc ceiling first: a binary above it must not become packages. The
# dynamic symbol table it reads is the same before and after stripping.
"$src/packaging/linux/check-glibc.sh" "$target/release/bachelorpad" "$ceiling" > "$out/glibc-required.txt"
"$src/packaging/linux/build-packages.sh" --no-build --out "$out" "${package_args[@]}"

# A record of what built this, for the release manifest (M3-03).
{
    echo "rustc: $(rustc --version)"
    echo "cargo: $(cargo --version)"
    echo "cargo-deb: $(cargo deb --version 2>/dev/null | head -n1)"
    echo "rpmbuild: $(rpmbuild --version)"
    echo "glibc (build): $(ldd --version | head -n1)"
    echo "os: $(. /etc/os-release && echo "$PRETTY_NAME")"
} > "$out/build-environment.txt"
echo "release-entrypoint: done; outputs in $out"
