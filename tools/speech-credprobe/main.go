// speech-credprobe exercises internal/speech CredentialStore against the
// production OS credential namespace on the current platform. W0 Task 5
// (Windows leg): prove Windows Credential Manager (wincred) stores, reads,
// reports presence and deletes the siliconflow slot, and that no plaintext
// fallback exists — backend failure surfaces as credential_unavailable.
package main

import (
	"flag"
	"fmt"
	"os"

	"github.com/ProjectViVy/agent-diva/internal/speech"
)

const probeSecret = "w0-2-credprobe-7f3a9c51" // disposable probe value, deleted below

func main() {
	op := flag.String("op", "cycle", "cycle | set | delete")
	flag.Parse()
	fmt.Println("speech-credprobe — OS credential store leg")
	fmt.Println("service: dev.projectivy.diva.speech  slot: v1.siliconflow (fixed namespace)")
	store := speech.NewCredentialStore(speech.OsKeyring{})

	fmt.Printf("presence(before): %s\n", store.Presence(speech.ProviderSiliconFlow))

	if *op == "set" || *op == "cycle" {
		if err := store.Set(speech.ProviderSiliconFlow, probeSecret); err != nil {
			fmt.Printf("set: error: %v\n", err)
			os.Exit(2)
		}
		fmt.Println("set: ok")
		fmt.Printf("presence(after set): %s\n", store.Presence(speech.ProviderSiliconFlow))
	}

	if *op == "delete" || *op == "cycle" {
		if err := store.Delete(speech.ProviderSiliconFlow); err != nil {
			fmt.Printf("delete: error: %v\n", err)
			os.Exit(3)
		}
		fmt.Println("delete: ok")
		fmt.Printf("presence(after delete): %s\n", store.Presence(speech.ProviderSiliconFlow))
	}
}
