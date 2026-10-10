# Verification

Red: S01-assets-red.txt reproduces the missing index. Green: S01-assets-green.txt, 1 test. The actual SDK now reaches compilation; S01-staged-build.txt records absent GTK/glib/WebKit/libsoup development packages. The worktree staging command also needs GOFLAGS=-buildvcs=false; SDK provenance remains separately recorded. Sealed host build/race tests are blocked, not passed.
