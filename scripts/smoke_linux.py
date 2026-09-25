#!/usr/bin/env python3
"""Drive the real bachelorpad binary on an X display with synthetic input.

Called by ``scripts/smoke-linux.sh``, which owns the virtual display. This
file owns one app run per view: launch with a throwaway home, find the
window, type, Ctrl+S, check the bytes on disk, then walk the in-window
"Unsaved changes" sheet through a real WM_DELETE_WINDOW and check that the
process exits.

Why XTEST rather than ``XSendEvent``: toolkits may drop events that carry
the ``send_event`` flag, and XTEST events are indistinguishable from a
keyboard -- they go through the server's own focus, keymap and
input-method path, which is the path under test. The close request is the
exception: WM_DELETE_WINDOW *is* a sent client message when a real window
manager delivers it, so sending it ourselves is the faithful version.

Exit codes: 0 pass, 1 a check failed, 2 the harness could not do its job
(no window, no XTEST, no DISPLAY).
"""

from __future__ import annotations

import argparse
import os
import shutil
import signal
import subprocess
import sys
import tempfile
import time
from pathlib import Path

from Xlib import X, XK, display, error, protocol
from Xlib.ext import xtest

EXIT_PASS = 0
EXIT_FAIL = 1
EXIT_HARNESS = 2

INITIAL_TEXT = "BachelorPad smoke fixture\n"
# Lower-case letters, digits and spaces only: each is one unshifted key on
# the default keymap Xvfb ships, so a failure here is the app's and not a
# keymap guess gone wrong.
TYPED = "smoke typed 42"
# Typed after the save so the document is dirty for the close checks. It
# must never reach the disk: the sheet is answered Don't Save.
UNSAVED = " never saved"

# Every wait has a bound, so a hung app fails the run instead of hanging it.
WINDOW_TIMEOUT = 20.0
SAVE_TIMEOUT = 8.0
EXIT_TIMEOUT = 8.0
# How long a process has to stay alive to count as "did not exit". Long
# enough for a debug build to have handled the close request and hidden its
# window if it were going to.
STAYS_ALIVE = 2.0
# Settling time after the window maps: mapped only says the first frame is
# under way, not that the document is loaded and the editor focused.
FIRST_FRAME = 1.0
KEY_GAP = 0.02

# Views with a known product defect this harness observes, keyed by view.
# The run reports XFAIL, works around it and carries on, so the rest of the
# path stays covered; when the defect is fixed the run says XPASS and the
# entry should go. Anything else that loses keys is a FAIL.
KNOWN_DEFECTS: dict[str, str] = {
    # Empty. "editor-view" was here -- keys typed before any click were lost,
    # because nothing gave the custom surface the keyboard at launch -- until
    # `run_with` started calling `focus-editor` after showing the window.
}


class HarnessError(Exception):
    """The harness could not drive the app; says nothing about the app."""


class CheckFailed(Exception):
    """The app did something other than what the check expects."""


def log(status: str, label: str, detail: str) -> None:
    print(f"{status:<5} [{label}] {detail}", flush=True)


def wait_for(predicate, timeout: float, interval: float = 0.1):
    """Poll until ``predicate`` returns something truthy, or ``timeout``."""
    deadline = time.monotonic() + timeout
    while True:
        value = predicate()
        if value:
            return value
        if time.monotonic() >= deadline:
            return None
        time.sleep(interval)


class Keyboard:
    """Synthetic keys through XTEST, resolved against the server's keymap."""

    def __init__(self, dpy: display.Display) -> None:
        self.dpy = dpy
        if not dpy.has_extension("XTEST"):
            raise HarnessError("the X server has no XTEST extension")

    def code(self, keysym_name: str) -> int:
        sym = XK.string_to_keysym(keysym_name)
        if sym == X.NoSymbol:
            raise HarnessError(f"no keysym called {keysym_name!r}")
        code = self.dpy.keysym_to_keycode(sym)
        if not code:
            raise HarnessError(f"keysym {keysym_name!r} is not on this keymap")
        return code

    def chord(self, *names: str) -> None:
        """Press every key in order, then release in reverse: Ctrl+S etc."""
        codes = [self.code(n) for n in names]
        for c in codes:
            xtest.fake_input(self.dpy, X.KeyPress, c)
            self.dpy.sync()
            time.sleep(KEY_GAP)
        for c in reversed(codes):
            xtest.fake_input(self.dpy, X.KeyRelease, c)
            self.dpy.sync()
            time.sleep(KEY_GAP)

    def type(self, text: str) -> None:
        for ch in text:
            self.chord("space" if ch == " " else ch)


def window_pid(dpy: display.Display, win) -> int | None:
    atom = dpy.intern_atom("_NET_WM_PID")
    try:
        prop = win.get_full_property(atom, X.AnyPropertyType)
    except error.XError:
        return None
    return int(prop.value[0]) if prop and len(prop.value) else None


def find_window(dpy: display.Display, pid: int):
    """The app's top-level window: titled BachelorPad, mapped, owned by pid.

    There is no window manager on the virtual display, so there is no
    ``_NET_CLIENT_LIST`` to read; the app's window is a direct child of the
    root. The pid check keeps a stray window from another run from being
    mistaken for this one.
    """
    try:
        children = dpy.screen().root.query_tree().children
    except error.XError:
        return None
    for win in children:
        try:
            name = win.get_wm_name() or ""
            if isinstance(name, bytes):
                name = name.decode("utf-8", "replace")
            mapped = win.get_attributes().map_state == X.IsViewable
        except error.XError:
            continue
        if "BachelorPad" in name and mapped and window_pid(dpy, win) == pid:
            return win
    return None


def focus(dpy: display.Display, win) -> None:
    """Give the window the X keyboard focus, as a window manager would.

    With no window manager nothing else will. Xvfb starts with focus on
    PointerRoot, and a pointer merely resting over the window -- or even a
    click in it -- does not make winit treat the window as focused, so no
    key reaches Slint. An explicit XSetInputFocus does. This, not the input
    method, is why an earlier session saw the default view take no keys:
    XMODIFIERS=@im=none and empty GTK_IM_MODULE/QT_IM_MODULE changed
    nothing, and the default view works with no IM settings once this runs.
    """
    geom = win.get_geometry()
    xtest.fake_input(dpy, X.MotionNotify, x=geom.x + geom.width // 2, y=geom.y + geom.height // 2)
    win.raise_window()
    win.set_input_focus(X.RevertToPointerRoot, X.CurrentTime)
    dpy.sync()
    time.sleep(0.3)


def click_centre(dpy: display.Display, win) -> None:
    """Left-click the middle of the window, which is in the text area."""
    geom = win.get_geometry()
    xtest.fake_input(dpy, X.MotionNotify, x=geom.x + geom.width // 2, y=geom.y + geom.height // 2)
    xtest.fake_input(dpy, X.ButtonPress, 1)
    dpy.sync()
    time.sleep(KEY_GAP)
    xtest.fake_input(dpy, X.ButtonRelease, 1)
    dpy.sync()
    time.sleep(0.3)


def request_close(dpy: display.Display, win) -> None:
    """What a window manager sends when the user clicks the close button."""
    protocols = dpy.intern_atom("WM_PROTOCOLS")
    delete = dpy.intern_atom("WM_DELETE_WINDOW")
    event = protocol.event.ClientMessage(
        window=win,
        client_type=protocols,
        data=(32, [delete, X.CurrentTime, 0, 0, 0]),
    )
    win.send_event(event, event_mask=X.NoEventMask)
    dpy.sync()


def read_text(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return ""


def still_running(proc: subprocess.Popen, seconds: float) -> bool:
    try:
        proc.wait(timeout=seconds)
    except subprocess.TimeoutExpired:
        return True
    return False


def exits(proc: subprocess.Popen, seconds: float) -> bool:
    return not still_running(proc, seconds)


def isolated_env(home: Path) -> dict[str, str]:
    """An environment that cannot read or write the developer's own state.

    Every base directory the app resolves goes under the throwaway home, so
    config, recovery journals, profiles and logs (``$XDG_STATE_HOME``) all
    land there and are deleted with it.
    """
    env = dict(os.environ)
    for var, sub in (
        ("HOME", ""),
        ("XDG_CONFIG_HOME", "config"),
        ("XDG_STATE_HOME", "state"),
        ("XDG_DATA_HOME", "data"),
        ("XDG_CACHE_HOME", "cache"),
        ("XDG_RUNTIME_DIR", "runtime"),
    ):
        path = home / sub if sub else home
        path.mkdir(parents=True, exist_ok=True)
        env[var] = str(path)
    (home / "runtime").chmod(0o700)
    # Not read by the app today (logs follow XDG_STATE_HOME); set so that a
    # future override of that name cannot escape the throwaway home.
    env["BACHELORPAD_LOG_DIR"] = str(home / "logs")
    for var in ("BACHELORPAD_CONFIG", "BACHELORPAD_THEME", "BACHELORPAD_RENDERER",
                "BACHELORPAD_FONT_SIZE", "BACHELORPAD_TAB_WIDTH",
                "BACHELORPAD_INDENT_SPACES", "WAYLAND_DISPLAY"):
        env.pop(var, None)
    # No session bus on a bare Xvfb. Saying so up front stops the app's
    # desktop-settings watcher reaching for the developer's real bus through
    # an inherited address.
    env["DBUS_SESSION_BUS_ADDRESS"] = "disabled:"
    env.setdefault("RUST_BACKTRACE", "1")
    # The app's output is only ever read back as text, on failure.
    env["NO_COLOR"] = "1"
    return env


def run_view(binary: Path, view: str, keep: bool) -> int:
    work = Path(tempfile.mkdtemp(prefix=f"bp-smoke-{view}-"))
    doc = work / "doc" / "smoke.txt"
    doc.parent.mkdir()
    doc.write_text(INITIAL_TEXT, encoding="utf-8")
    env = isolated_env(work / "home")

    args = [str(binary)]
    if view == "editor-view":
        args.append("--editor-view")
    args.append(str(doc))

    app_log = work / "app.log"
    with app_log.open("wb") as sink:
        proc = subprocess.Popen(args, env=env, stdout=sink, stderr=subprocess.STDOUT, cwd=work)
    status = EXIT_HARNESS
    dpy = None
    try:
        dpy = display.Display()
        status = drive(dpy, proc, doc, view)
    except HarnessError as e:
        log("FAIL", view, f"harness: {e}")
        status = EXIT_HARNESS
    except CheckFailed as e:
        log("FAIL", view, str(e))
        status = EXIT_FAIL
    finally:
        if proc.poll() is None:
            proc.send_signal(signal.SIGTERM)
            if not exits(proc, 3):
                proc.kill()
                proc.wait()
        if dpy is not None:
            dpy.close()
        if status != EXIT_PASS:
            tail = read_text(app_log).splitlines()[-15:]
            if tail:
                print(f"      [{view}] last lines of the app's output:", flush=True)
                for line in tail:
                    print(f"        {line}", flush=True)
        if keep or status != EXIT_PASS:
            print(f"      [{view}] work directory kept: {work}", flush=True)
        else:
            shutil.rmtree(work, ignore_errors=True)
    return status


def type_and_save(keys: Keyboard, doc: Path) -> str:
    """Type at the end of the document and press Ctrl+S; return the disk."""
    # Ctrl+End so the typing lands after the fixture wherever the caret
    # started.
    keys.chord("Control_L", "End")
    keys.type(TYPED)
    keys.chord("Control_L", "s")
    expected = INITIAL_TEXT + TYPED
    wait_for(lambda: read_text(doc) == expected, SAVE_TIMEOUT)
    return read_text(doc)


def drive(dpy: display.Display, proc: subprocess.Popen, doc: Path, view: str) -> int:
    keys = Keyboard(dpy)
    expected = INITIAL_TEXT + TYPED

    win = wait_for(lambda: proc.poll() is None and find_window(dpy, proc.pid), WINDOW_TIMEOUT, 0.2)
    if proc.poll() is not None:
        raise CheckFailed(f"the app exited during start-up with status {proc.returncode}")
    if not win:
        raise HarnessError(f"no BachelorPad window for pid {proc.pid} within {WINDOW_TIMEOUT:.0f} s")
    log("PASS", view, f"window 0x{win.id:x} mapped")

    time.sleep(FIRST_FRAME)
    focus(dpy, win)

    # First without a click: a person who opens a file and starts typing
    # does not click first, so start-up focus is part of what is checked.
    on_disk = type_and_save(keys, doc)
    if on_disk == expected:
        if view in KNOWN_DEFECTS:
            log("XPASS", view, "known defect no longer reproduces; remove it from KNOWN_DEFECTS")
        log("PASS", view, "typed text and Ctrl+S reached the disk, no click needed")
    elif on_disk == INITIAL_TEXT and view in KNOWN_DEFECTS:
        log("XFAIL", view, KNOWN_DEFECTS[view])
        click_centre(dpy, win)
        on_disk = type_and_save(keys, doc)
        if on_disk != expected:
            raise CheckFailed(
                f"after a click, typing and Ctrl+S the file holds {on_disk!r}, expected {expected!r}"
            )
        log("PASS", view, "after a click, typed text and Ctrl+S reached the disk")
    elif on_disk == INITIAL_TEXT:
        # Nothing reached the document at all -- the signature of keys not
        # being delivered, as opposed to delivered wrongly.
        raise CheckFailed("no typed key reached the document: file unchanged after typing and Ctrl+S")
    else:
        raise CheckFailed(f"after typing and Ctrl+S the file holds {on_disk!r}, expected {expected!r}")

    # Dirty the document again, then close by the window manager's route.
    # The app must refuse and ask rather than lose the edit.
    keys.type(UNSAVED)
    time.sleep(0.3)
    request_close(dpy, win)
    if not still_running(proc, STAYS_ALIVE):
        raise CheckFailed(
            f"the app exited (status {proc.returncode}) on a close request with unsaved changes; "
            "expected the Unsaved changes sheet"
        )
    log("PASS", view, "close request with unsaved changes kept the window open")

    keys.chord("Escape")
    if not still_running(proc, STAYS_ALIVE):
        raise CheckFailed(f"Escape on the Unsaved changes sheet exited the app (status {proc.returncode})")
    log("PASS", view, "Escape (Cancel) on the sheet kept the app running")

    request_close(dpy, win)
    if not still_running(proc, STAYS_ALIVE):
        raise CheckFailed(
            f"the second close request exited without asking (status {proc.returncode}); "
            "Cancel should have left the document dirty"
        )
    # Buttons are Save, Don't Save, Cancel with Save highlighted; one Right
    # moves the highlight to Don't Save. If the sheet were not showing,
    # these keys would land in the document and the process would not exit.
    keys.chord("Right")
    keys.chord("Return")
    if not exits(proc, EXIT_TIMEOUT):
        raise CheckFailed(f"Right, Enter (Don't Save) did not end the process within {EXIT_TIMEOUT:.0f} s")
    if proc.returncode != 0:
        raise CheckFailed(f"the app exited with status {proc.returncode} after Don't Save")
    if read_text(doc) != expected:
        raise CheckFailed(f"Don't Save changed the file on disk: {read_text(doc)!r}")
    log("PASS", view, "Don't Save ended the process with status 0 and left the file as saved")
    return EXIT_PASS


def _terminated(*_) -> None:
    raise HarnessError("terminated by SIGTERM (the wrapper's time limit, or a person)")


def main() -> int:
    parser = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    parser.add_argument("binary", type=Path)
    parser.add_argument("--view", choices=["default", "editor-view"], required=True)
    parser.add_argument("--keep", action="store_true", help="keep the work directory")
    opts = parser.parse_args()
    # The shell wrapper's `timeout` sends SIGTERM. Python's default for it is
    # to die on the spot, skipping the `finally` that stops the app; raised
    # as a harness error instead, the app is stopped rather than orphaned and
    # the run says why it ended.
    signal.signal(signal.SIGTERM, _terminated)
    if not os.environ.get("DISPLAY"):
        log("FAIL", "harness", "DISPLAY is not set; run through scripts/smoke-linux.sh")
        return EXIT_HARNESS
    return run_view(opts.binary, opts.view, opts.keep)


if __name__ == "__main__":
    sys.exit(main())
