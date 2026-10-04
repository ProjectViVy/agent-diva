// Package divagui exposes the bundled DIVA frontend and the seeded default
// VIVY config to the Go desktop host (DN-W3 Task 1). dist/ is produced by
// `pnpm --dir agent-diva-gui build`; dist/.gitkeep keeps the embed path valid
// before the first frontend build.
package divagui

import (
	"embed"
	"io/fs"
)

//go:embed all:dist
var dist embed.FS

//go:embed src-tauri/resources/vivy.default.yaml
var defaultVivyConfig []byte

// Frontend returns the bundled UI tree rooted at dist/.
func Frontend() fs.FS {
	sub, err := fs.Sub(dist, "dist")
	if err != nil {
		return dist
	}
	return sub
}

// DefaultVivyConfig is the default vivy.yaml seeded on first launch, copied
// verbatim from src-tauri/resources/vivy.default.yaml — single source.
func DefaultVivyConfig() []byte {
	out := make([]byte, len(defaultVivyConfig))
	copy(out, defaultVivyConfig)
	return out
}
