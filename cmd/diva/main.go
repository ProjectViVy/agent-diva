// Command diva is the DN-W3 desktop host: one Wails window over one sealed
// VIVY runtime. The only VIVY surface is sdk/host/v1 via internal/desktop.
package main

import (
	"context"
	"flag"
	"fmt"
	"log"
	"os"

	divagui "github.com/ProjectViVy/agent-diva/agent-diva-gui"
	"github.com/ProjectViVy/agent-diva/internal/desktop"
)

func main() {
	configFlag := flag.String("config", "", "absolute path to vivy.yaml (else $DIVA_VIVY_CONFIG, else <config-dir>/DIVA/vivy.yaml)")
	flag.Parse()

	configPath, err := desktop.ResolveConfigPath(*configFlag, divagui.DefaultVivyConfig())
	if err != nil {
		fmt.Fprintln(os.Stderr, "diva:", err)
		os.Exit(2)
	}

	logger := log.New(os.Stderr, "diva: ", log.LstdFlags)
	d, err := desktop.Compose(context.Background(), desktop.Config{
		ConfigPath:  configPath,
		WithoutEars: true,
		Frontend:    divagui.Frontend(),
	}, logger)
	if err != nil {
		fmt.Fprintln(os.Stderr, "diva:", err)
		os.Exit(1)
	}
	if err := d.Run(); err != nil {
		logger.Print(err)
		os.Exit(1)
	}
}
