#!/usr/bin/env bash
# Prints the newest glibc symbol version a binary needs, and fails if it is
# newer than a ceiling (release plan M3-02, exit criterion L2).
#
# A binary built against glibc N references versioned symbols such as
# `memcpy@GLIBC_2.14`; the dynamic loader refuses to start it on a system
# whose glibc lacks the newest of those versions. The packages promise
# Debian 12 and Ubuntu 22.04, so the ceiling defaults to Debian 12's 2.36.
#
# Usage:
#   packaging/linux/check-glibc.sh BINARY [CEILING]
#
# Prints, e.g.:
#   GLIBC_2.34
# on standard output, and a verdict on standard error.
#
# Exit status: 0 at or below the ceiling; 1 above it; 2 the binary could not
# be read or needs no versioned glibc symbol at all.

set -euo pipefail

binary=${1:-}
ceiling=${2:-2.36}

if [[ -z $binary || $binary == -h || $binary == --help ]]; then
    sed -n '2,/^$/s/^# \{0,1\}//p' "$0"
    [[ -n $binary ]] && exit 0
    exit 2
fi
[[ -r $binary ]] || { echo "check-glibc: cannot read $binary" >&2; exit 2; }
[[ $ceiling =~ ^[0-9]+\.[0-9]+$ ]] || { echo "check-glibc: ceiling must look like 2.36, not '$ceiling'" >&2; exit 2; }
command -v objdump >/dev/null || { echo "check-glibc: objdump (binutils) is not on PATH" >&2; exit 2; }

# The dynamic symbol table's version references are what the loader checks.
# objdump -T shows them per symbol (`GLIBC_2.34`, or `(GLIBC_2.2.5)` for a
# hidden one); `sort -V` orders 2.9 before 2.10. The table survives
# stripping, so the stripped binary and the one with symbols give the same
# answer.
versions=$(objdump -T "$binary" | grep -o 'GLIBC_[0-9][0-9.]*' | sort -u -V) || true
if [[ -z $versions ]]; then
    echo "check-glibc: $binary references no GLIBC_ symbol versions (not a dynamically linked glibc binary?)" >&2
    exit 2
fi
newest=$(tail -n1 <<<"$versions")
echo "$newest"

needed=${newest#GLIBC_}
# Compare as versions, not strings: the ceiling passes if it sorts last.
if [[ $(printf '%s\n%s\n' "$needed" "$ceiling" | sort -V | tail -n1) == "$ceiling" ]]; then
    echo "check-glibc: OK -- $binary needs glibc $needed, at or below the $ceiling ceiling" >&2
    exit 0
fi
echo "check-glibc: FAIL -- $binary needs glibc $needed, above the $ceiling ceiling. Symbols that need more than $ceiling:" >&2
objdump -T "$binary" | awk -v c="$ceiling" '
    match($0, /GLIBC_[0-9][0-9.]*/) {
        v = substr($0, RSTART + 6, RLENGTH - 6)
        split(v, a, "."); split(c, b, ".")
        if (a[1] + 0 > b[1] + 0 || (a[1] + 0 == b[1] + 0 && a[2] + 0 > b[2] + 0)) print "  " $NF "@GLIBC_" v
    }' | sort -u >&2
echo "check-glibc: build in packaging/linux/Dockerfile.release (Debian 12) to meet it" >&2
exit 1
