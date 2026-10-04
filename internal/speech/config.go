package speech

import (
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"strings"
)

// `diva.speech/v1` preferences: a closed schema, strict validation, and
// atomic file persistence behind a monotonic revision CAS. Readback never
// carries secrets — credentials live in credentials.go slots and only
// their presence is reported.

const SpeechSchema = "diva.speech/v1"

// Provider is the closed provider set.
type Provider string

const (
	ProviderSiliconFlow Provider = "siliconflow"
	ProviderMiniMax     Provider = "minimax"
)

func (p Provider) String() string { return string(p) }

func ParseProvider(v string) (Provider, error) {
	switch v {
	case "siliconflow":
		return ProviderSiliconFlow, nil
	case "minimax":
		return ProviderMiniMax, nil
	default:
		return "", speechErr(CodeInvalidInput, fmt.Sprintf("unknown provider %q", v))
	}
}

// Reference is the strict reference union: "system" |
// {"voice_id":...} | {"asset_id":...,"transcript":...}. Any other shape
// fails to decode and is rejected, never dropped.
type Reference struct {
	System     *struct{} `json:"-"` // sentinel: marshals as the string "system"
	VoiceID    string    `json:"voice_id,omitempty"`
	AssetID    string    `json:"asset_id,omitempty"`
	Transcript string    `json:"transcript,omitempty"`
	isSystem   bool
}

func (r *Reference) MarshalJSON() ([]byte, error) {
	if r.isSystem {
		return json.Marshal("system")
	}
	type alias Reference
	return json.Marshal((*alias)(r))
}

func (r *Reference) UnmarshalJSON(data []byte) error {
	var s string
	if err := json.Unmarshal(data, &s); err == nil {
		if s != "system" {
			return errors.New(`reference string must be "system"`)
		}
		r.isSystem = true
		return nil
	}
	type alias Reference
	var a alias
	dec := json.NewDecoder(strings.NewReader(string(data)))
	dec.DisallowUnknownFields()
	if err := dec.Decode(&a); err != nil {
		return errors.New(`reference string must be "system"`)
	}
	*r = Reference(a)
	return nil
}

func (r *Reference) IsSystem() bool { return r.isSystem }

type SttPreferences struct {
	Provider Provider `json:"provider"`
	BaseURL  string   `json:"base_url"`
	Model    string   `json:"model"`
}

type SiliconFlowTts struct {
	BaseURL   string     `json:"base_url"`
	Model     string     `json:"model"`
	Voice     string     `json:"voice"`
	Speed     float64    `json:"speed"`
	Reference *Reference `json:"reference,omitempty"`
}

type MiniMaxTts struct {
	BaseURL string  `json:"base_url"`
	Model   string  `json:"model"`
	VoiceID string  `json:"voice_id"`
	Speed   float64 `json:"speed"`
	Volume  float64 `json:"volume"`
}

type TtsPreferences struct {
	Provider    Provider        `json:"provider"`
	SiliconFlow *SiliconFlowTts `json:"siliconflow,omitempty"`
	MiniMax     *MiniMaxTts     `json:"minimax,omitempty"`
}

type Preferences struct {
	STT             SttPreferences `json:"stt"`
	TTS             TtsPreferences `json:"tts"`
	AutoReadReplies bool           `json:"auto_read_replies"`
}

type ConfigFile struct {
	Schema      string      `json:"schema"`
	Revision    uint64      `json:"revision"`
	Preferences Preferences `json:"preferences"`
}

func invalidInput(format string, args ...any) *SpeechError {
	return speechErr(CodeInvalidInput, fmt.Sprintf(format, args...))
}

func strictDecode(data []byte, v any) error {
	dec := json.NewDecoder(strings.NewReader(string(data)))
	dec.DisallowUnknownFields()
	return dec.Decode(v)
}

// validateBaseURL: http(s) origin only — no credentials in the URL, no
// query, no fragment (C2-4).
func validateBaseURL(field, url string) error {
	trimmed := strings.TrimSpace(url)
	if trimmed == "" || len(trimmed) > 512 {
		return invalidInput("%s: empty or overlong base_url", field)
	}
	scheme, rest, found := strings.Cut(trimmed, "://")
	if !found {
		return invalidInput("%s: base_url needs a scheme", field)
	}
	if scheme != "https" && scheme != "http" {
		return invalidInput("%s: base_url scheme must be http(s)", field)
	}
	if rest == "" {
		return invalidInput("%s: base_url has no host", field)
	}
	authority := rest
	if i := strings.Index(rest, "/"); i >= 0 {
		authority = rest[:i]
	}
	if authority == "" {
		return invalidInput("%s: base_url has no host", field)
	}
	if strings.Contains(authority, "@") {
		return invalidInput("%s: base_url must not carry credentials", field)
	}
	if strings.ContainsAny(trimmed, "#?") {
		return invalidInput("%s: base_url must not carry query/fragment", field)
	}
	return nil
}

func validateBoundedStr(field, value string, max int) error {
	if strings.TrimSpace(value) == "" {
		return invalidInput("%s is required", field)
	}
	if len(value) > max {
		return invalidInput("%s exceeds %d bytes", field, max)
	}
	return nil
}

// ValidatePreferences enforces semantics beyond shape: provider block
// must exist for the selected provider, numeric bounds hold, referenced
// assets exist (assetExists supplied by the AssetStore so a
// deleted/missing reference is an explicit error, not a stale pointer).
func ValidatePreferences(prefs *Preferences, assetExists func(string) bool) error {
	if prefs.STT.Provider != ProviderSiliconFlow {
		return speechErr(CodeUnsupportedReference, "stt.provider must be siliconflow")
	}
	if err := validateBaseURL("stt.base_url", prefs.STT.BaseURL); err != nil {
		return err
	}
	if err := validateBoundedStr("stt.model", prefs.STT.Model, 128); err != nil {
		return err
	}
	switch prefs.TTS.Provider {
	case ProviderSiliconFlow:
		block := prefs.TTS.SiliconFlow
		if block == nil {
			return invalidInput("tts.provider=siliconflow requires a siliconflow block")
		}
		if err := validateBaseURL("tts.siliconflow.base_url", block.BaseURL); err != nil {
			return err
		}
		if err := validateBoundedStr("tts.siliconflow.model", block.Model, 128); err != nil {
			return err
		}
		if err := validateBoundedStr("tts.siliconflow.voice", block.Voice, 256); err != nil {
			return err
		}
		if block.Speed < 0.5 || block.Speed > 2.0 {
			return invalidInput("tts.siliconflow.speed out of range 0.5..=2.0")
		}
		if block.Reference != nil {
			if err := validateReference(block.Reference, assetExists); err != nil {
				return err
			}
		}
	case ProviderMiniMax:
		block := prefs.TTS.MiniMax
		if block == nil {
			return invalidInput("tts.provider=minimax requires a minimax block")
		}
		if err := validateBaseURL("tts.minimax.base_url", block.BaseURL); err != nil {
			return err
		}
		if err := validateBoundedStr("tts.minimax.model", block.Model, 128); err != nil {
			return err
		}
		if err := validateBoundedStr("tts.minimax.voice_id", block.VoiceID, 256); err != nil {
			return err
		}
		if block.Speed < 0.5 || block.Speed > 2.0 {
			return invalidInput("tts.minimax.speed out of range 0.5..=2.0")
		}
		if block.Volume < 0.0 || block.Volume > 10.0 {
			return invalidInput("tts.minimax.volume out of range 0.0..=10.0")
		}
	default:
		return invalidInput("tts.provider must be siliconflow or minimax")
	}
	return nil
}

func validateReference(r *Reference, assetExists func(string) bool) error {
	switch {
	case r.isSystem:
		return nil
	case r.VoiceID != "":
		// SiliconFlow reusable uri shape: speech:<customName>:<id>
		if !strings.HasPrefix(r.VoiceID, "speech:") || len(r.VoiceID) > 256 {
			return speechErr(CodeUnsupportedReference, "reusable reference must be a speech:<name>:<id> uri")
		}
		return nil
	default:
		if err := ValidateAssetID(r.AssetID); err != nil {
			return err
		}
		if strings.TrimSpace(r.Transcript) == "" || len(r.Transcript) > 4096 {
			return speechErr(CodeUnsupportedReference, "inline reference transcript empty or overlong")
		}
		if !assetExists(r.AssetID) {
			return speechErr(CodeAssetNotFound, "configured reference asset is missing")
		}
		return nil
	}
}

func defaultPreferences() Preferences {
	return Preferences{
		STT: SttPreferences{
			Provider: ProviderSiliconFlow,
			BaseURL:  "https://api.siliconflow.cn",
			Model:    "FunAudioLLM/SenseVoiceSmall",
		},
		TTS: TtsPreferences{
			Provider: ProviderSiliconFlow,
			SiliconFlow: &SiliconFlowTts{
				BaseURL: "https://api.siliconflow.cn",
				Model:   "FunAudioLLM/CosyVoice2-0.5B",
				Voice:   "FunAudioLLM/CosyVoice2-0.5B:alex",
				Speed:   1.0,
			},
		},
	}
}

// atomicWrite writes bytes to path atomically (tmp file + rename) so a
// crash mid write never leaves a torn preferences or manifest file.
func atomicWrite(path string, bytes []byte) error {
	parent := filepath.Dir(path)
	if err := os.MkdirAll(parent, 0o700); err != nil {
		return invalidInput("create %s: %v", parent, err)
	}
	tmp := filepath.Join(parent, filepath.Base(path)+".tmp")
	if err := os.WriteFile(tmp, bytes, 0o600); err != nil {
		return speechErr(CodeBusy, fmt.Sprintf("write %s: %v", tmp, err)).retryable(true)
	}
	if err := os.Rename(tmp, path); err != nil {
		return speechErr(CodeBusy, fmt.Sprintf("replace %s: %v", path, err)).retryable(true)
	}
	return nil
}

// ConfigStore is a versioned preferences file with revision CAS. One
// file, one writer — the shell serializes access through a mutex.
type ConfigStore struct {
	path string
	file ConfigFile
}

// OpenConfigStore loads or seeds path. A missing file starts at
// revision 0 with default preferences; a corrupt file is an explicit
// error, never silently re-seeded over user data.
func OpenConfigStore(path string) (*ConfigStore, error) {
	var file ConfigFile
	if _, err := os.Stat(path); err == nil {
		data, err := os.ReadFile(path)
		if err != nil {
			return nil, invalidInput("read %s: %v", path, err)
		}
		if err := strictDecode(data, &file); err != nil {
			return nil, invalidInput("preferences file is corrupt: %v", err)
		}
		if file.Schema != SpeechSchema {
			return nil, invalidInput("preferences schema %q != %q", file.Schema, SpeechSchema)
		}
	} else {
		file = ConfigFile{Schema: SpeechSchema, Revision: 0, Preferences: defaultPreferences()}
	}
	return &ConfigStore{path: path, file: file}, nil
}

func (s *ConfigStore) Revision() uint64 { return s.file.Revision }

// Preferences returns a deep copy — callers may mutate the result
// without affecting the stored config (pointer fields are cloned).
func (s *ConfigStore) Preferences() Preferences {
	p := s.file.Preferences
	if p.TTS.SiliconFlow != nil {
		block := *p.TTS.SiliconFlow
		if block.Reference != nil {
			ref := *block.Reference
			block.Reference = &ref
		}
		p.TTS.SiliconFlow = &block
	}
	if p.TTS.MiniMax != nil {
		block := *p.TTS.MiniMax
		p.TTS.MiniMax = &block
	}
	return p
}

// Update is CAS: baseRevision must equal the current revision or the
// write is a conflict — never a last-writer-wins overwrite.
func (s *ConfigStore) Update(baseRevision uint64, prefs Preferences, assetExists func(string) bool) (uint64, error) {
	if baseRevision != s.file.Revision {
		return 0, speechErr(CodeRevisionConflict,
			fmt.Sprintf("preferences moved to revision %d — refetch before writing", s.file.Revision))
	}
	if err := ValidatePreferences(&prefs, assetExists); err != nil {
		return 0, err
	}
	next := ConfigFile{Schema: SpeechSchema, Revision: s.file.Revision + 1, Preferences: prefs}
	data, err := json.MarshalIndent(next, "", "  ")
	if err != nil {
		return 0, invalidInput("encode preferences: %v", err)
	}
	if err := atomicWrite(s.path, data); err != nil {
		return 0, err
	}
	s.file = next
	return s.file.Revision, nil
}
