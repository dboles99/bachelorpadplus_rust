@echo off
rem bpad: the short name for BachelorPad+ (release plan M2-16).
rem
rem Runs the bachelorpad.exe beside this file, whatever the current
rem directory, with every argument passed on unchanged. cmd waits for a GUI
rem program started from a batch file, so the exit code passed back is the
rem editor's own.
"%~dp0bachelorpad.exe" %*
exit /b %ERRORLEVEL%
