# Windows GUI Gate Portability

The E7 GUI automation recipe now uses PowerShell-compatible command
separation. PowerShell 5 rejected `&&` before either GUI test or build could
start.

The recipe now changes directory and invokes npm in one PowerShell statement,
then retains the existing Tauri cargo check.
