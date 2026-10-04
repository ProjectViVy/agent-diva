package desktop

import (
	"fmt"
	"os"
	"path/filepath"
)

// ConfigEnvVar mirrors the retired Tauri shell convention.
const ConfigEnvVar = "DIVA_VIVY_CONFIG"

// ResolveConfigPath returns an absolute vivy.yaml path independent of the
// cwd: an explicit flag value wins, then DIVA_VIVY_CONFIG, then
// <os-config-dir>/DIVA/vivy.yaml. The file is seeded from defaultSeed when
// absent (same first-launch behavior as the Tauri shell).
func ResolveConfigPath(flagValue string, defaultSeed []byte) (string, error) {
	p := flagValue
	if p == "" {
		p = os.Getenv(ConfigEnvVar)
	}
	if p == "" {
		dir, err := os.UserConfigDir()
		if err != nil {
			return "", fmt.Errorf("user config dir: %w", err)
		}
		p = filepath.Join(dir, "DIVA", "vivy.yaml")
	}
	abs, err := filepath.Abs(p)
	if err != nil {
		return "", fmt.Errorf("config path: %w", err)
	}
	if err := ensureVivyConfig(abs, defaultSeed); err != nil {
		return "", err
	}
	return abs, nil
}

func ensureVivyConfig(path string, seed []byte) error {
	if _, err := os.Stat(path); err == nil {
		return nil
	} else if !os.IsNotExist(err) {
		return fmt.Errorf("stat config %s: %w", path, err)
	}
	if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
		return fmt.Errorf("config dir: %w", err)
	}
	if err := os.WriteFile(path, seed, 0o600); err != nil {
		return fmt.Errorf("write default config %s: %w", path, err)
	}
	return nil
}
