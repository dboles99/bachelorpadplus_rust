BachelorPad+ -- portable copy for Windows 10 and 11 (x64)
==========================================================

This folder is the whole program. Nothing is installed and nothing is
written to the registry until you choose to.

  bachelorpad.exe   the editor
  bpad.cmd          the short command: "bpad notes.txt" opens notes.txt.
                    Put this folder on your PATH to use it from any
                    terminal.
  LICENSE-MIT       BachelorPad+ is available under either licence,
  LICENSE-APACHE    at your option.

Running it
----------
Unpack the folder somewhere permanent (for example your Documents or a
USB stick) and run bachelorpad.exe. It keeps your settings in
%APPDATA%\bachelorpad and its recovery journal, history and cache in
%LOCALAPPDATA%\bachelorpad, the same places the installed version uses.

Opening files by double-clicking them
-------------------------------------
Windows only offers a program for a file type once the program is
registered, and a registration has to name where the program is. This
zip cannot know that until you have unpacked it, so it ships no
registration file. Instead:

  1. Unpack the folder where it will stay, and run bachelorpad.exe.
  2. Choose File > Set as Default Editor..., then Save Files...
     This saves two .reg files naming this copy's real location: one
     that registers BachelorPad+ for its file types, and one that removes
     exactly that registration again. Read them before running them.
  3. Double-click the first file to register, then pick BachelorPad+ in
     Windows Settings > Apps > Default apps.

Registration only adds BachelorPad+ to the "Open with" list and to
Default apps. It never takes a file type away from another program and
never changes Notepad; the choice stays in Windows Settings.

If you move the folder, save and run the files again. Before deleting the
folder, run the removal file.

Removing it
-----------
Run the removal .reg file if you registered, then delete this folder.
To remove your data as well, delete %LOCALAPPDATA%\bachelorpad (this
includes any unsaved work in the recovery journal) and, if no other copy
on any computer you sign in to still uses them, your settings in
%APPDATA%\bachelorpad.

Prefer an installer? The per-user installer registers and unregisters for
you, and needs no administrator rights.
