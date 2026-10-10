package app

import (
	"context"
	"strings"
	"testing"

	genassembly "agent-vivy/internal/generated/assembly"
	laputaevolution "github.com/dashimaki/laputa/evolution"
)

func TestMemoryLoopActivityContinuity(t *testing.T) {
	probe := genassembly.BuildDefault()
	if !probe.HasCognitiveFactory() {
		t.Skip("DIVA integration overlay required")
	}
	f := newMemoryLoopFixture(t, memoryLoopOptions{ConfigPath: memoryLoopConfig(t), ModelMode: "ack"})
	session := memoryLoopSession(t, f)
	facts := []string{memoryLoopRandomFact(t), memoryLoopRandomFact(t)}
	for _, fact := range facts {
		run := memoryLoopTurn(t, f, session, fact)
		if _, err := f.Wait(context.Background(), "canonical", run); err != nil {
			t.Fatal(err)
		}
	}
	var activity laputaevolution.ActivityResult
	memoryLoopAction(t, f, "diva.cognitive.actmem.read", map[string]any{"session_id": session, "sections": []string{"pulse", "recap"}, "max_chars": 1200}, &activity)
	counts := map[laputaevolution.EntrySection]int{}
	for _, entry := range activity.Entries {
		if entry.SessionID != session || entry.EventID == "" || len(entry.Sources) == 0 {
			t.Fatalf("activity source identity missing: %+v", entry)
		}
		counts[entry.Section]++
	}
	if counts[laputaevolution.SectionPulse] == 0 || counts[laputaevolution.SectionRecap] == 0 {
		t.Fatalf("MEM-S05-01: real completed turns left actual ACTMEM empty: revision=%d pulse=%d recap=%d (ingest rows are not Markdown activity)", activity.Revision, counts[laputaevolution.SectionPulse], counts[laputaevolution.SectionRecap])
	}
	for _, fact := range facts {
		found := false
		for _, entry := range activity.Entries {
			if entry.Section == laputaevolution.SectionRecap && strings.Contains(entry.Body, fact) {
				found = true
			}
		}
		if !found {
			t.Fatal("actual recap lost a short user-only fact")
		}
	}
}
