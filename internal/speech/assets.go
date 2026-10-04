package speech

// Bounded owned reference-asset storage. Import takes raw bytes + a
// display name/MIME (from the `x-diva-asset-meta` IPC header) — never a
// path or URL. Native IDs are digest-derived; filenames are DERIVED from
// the ID, so neither caller names nor manifest content can reach outside
// the asset dir. Deletes never race readers: an in-use asset is marked
// delete_pending and the last released lease completes the removal.

import (
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"sort"
	"strings"
	"sync"
)

const (
	AssetMaxFileBytes   = 10 * 1024 * 1024
	AssetMaxFiles       = 20
	AssetMaxTotalBytes  = 100 * 1024 * 1024
	AssetMaxDisplayName = 128
	assetManifest       = "manifest.json"
)

// AssetImportMeta is caller-supplied metadata (`x-diva-asset-meta`,
// bounded by the shell).
type AssetImportMeta struct {
	DisplayName string `json:"display_name"`
	MimeType    string `json:"mime_type"`
}

type assetEntry struct {
	AssetID       string `json:"asset_id"`
	DisplayName   string `json:"display_name"`
	MimeType      string `json:"mime_type"`
	SizeBytes     uint64 `json:"size_bytes"`
	DigestSHA256  string `json:"digest_sha256"`
	DeletePending bool   `json:"delete_pending,omitempty"`
}

// AssetDescriptor is the public descriptor — no filesystem path, no
// secret fields.
type AssetDescriptor struct {
	AssetID      string `json:"asset_id"`
	DisplayName  string `json:"display_name"`
	MimeType     string `json:"mime_type"`
	SizeBytes    uint64 `json:"size_bytes"`
	DigestSHA256 string `json:"digest_sha256"`
	Status       string `json:"status"`
}

type assetManifestDoc struct {
	Assets []assetEntry `json:"assets"`
}

// ValidateAssetID rejects anything that is not a native-generated
// `va-<16 lowercase hex>` — including separators, `..`, or foreign
// namespaces — before it can ever reach a path join.
func ValidateAssetID(id string) error {
	if len(id) != 19 || !strings.HasPrefix(id, "va-") {
		return invalidInput("malformed asset_id %q", id)
	}
	for _, c := range id[3:] {
		if !(c >= '0' && c <= '9' || c >= 'a' && c <= 'f') {
			return invalidInput("malformed asset_id %q", id)
		}
	}
	return nil
}

func assetFileName(id, mime string) string {
	ext := "mp3"
	if strings.Contains(mime, "wav") {
		ext = "wav"
	}
	return id + "." + ext
}

func sniffMime(b []byte) string {
	if len(b) >= 12 && string(b[0:4]) == "RIFF" && string(b[8:12]) == "WAVE" {
		return "audio/wav"
	}
	if len(b) >= 3 && string(b[0:3]) == "ID3" {
		return "audio/mpeg"
	}
	if len(b) >= 2 && b[0] == 0xFF && b[1]&0xE0 == 0xE0 {
		return "audio/mpeg"
	}
	return ""
}

func normalizeMime(mime string) string {
	switch strings.ToLower(strings.TrimSpace(mime)) {
	case "audio/wav", "audio/x-wav", "audio/wave", "audio/vnd.wave":
		return "audio/wav"
	case "audio/mpeg", "audio/mp3", "audio/mpeg3":
		return "audio/mpeg"
	default:
		return ""
	}
}

// AssetLease holds read bytes plus a release hook that completes pending
// deletes once the last reader releases.
type AssetLease struct {
	store    *AssetStore
	id       string
	Bytes    []byte
	MimeType string
	released bool
	mu       sync.Mutex
}

// Release returns the lease; the last release completes a pending delete.
func (l *AssetLease) Release() {
	l.mu.Lock()
	defer l.mu.Unlock()
	if l.released {
		return
	}
	l.released = true
	l.store.releaseLease(l.id)
}

type assetInner struct {
	manifest assetManifestDoc
	leases   map[string]int
}

// AssetStore is the bounded owned store.
type AssetStore struct {
	dir string
	mu  sync.Mutex
	in  assetInner
}

// OpenAssetStore opens the store: reconcile the manifest with the
// directory (orphan manifest rows whose files are gone are dropped on
// open — a row without bytes is a lie), and delete orphan files not
// referenced by any entry.
func OpenAssetStore(dir string) (*AssetStore, error) {
	if err := os.MkdirAll(dir, 0o700); err != nil {
		return nil, invalidInput("create %s: %v", dir, err)
	}
	var manifest assetManifestDoc
	manifestPath := filepath.Join(dir, assetManifest)
	if data, err := os.ReadFile(manifestPath); err == nil {
		if err := strictDecode(data, &manifest); err != nil {
			return nil, invalidInput("manifest corrupt: %v", err)
		}
	}
	kept := manifest.Assets[:0]
	keepNames := map[string]bool{assetManifest: true}
	for _, a := range manifest.Assets {
		name := assetFileName(a.AssetID, a.MimeType)
		if info, err := os.Stat(filepath.Join(dir, name)); err == nil && info.Mode().IsRegular() {
			kept = append(kept, a)
			keepNames[name] = true
		}
	}
	manifest.Assets = kept
	if entries, err := os.ReadDir(dir); err == nil {
		for _, e := range entries {
			if keepNames[e.Name()] {
				continue
			}
			if e.Type().IsRegular() {
				_ = os.Remove(filepath.Join(dir, e.Name()))
			}
		}
	}
	return &AssetStore{dir: dir, in: assetInner{manifest: manifest, leases: map[string]int{}}}, nil
}

func (s *AssetStore) persistLocked() error {
	data, err := json.MarshalIndent(s.in.manifest, "", "  ")
	if err != nil {
		return invalidInput("encode manifest: %v", err)
	}
	return atomicWrite(filepath.Join(s.dir, assetManifest), data)
}

func descriptorOf(e assetEntry) AssetDescriptor {
	status := "active"
	if e.DeletePending {
		status = "pending"
	}
	return AssetDescriptor{
		AssetID: e.AssetID, DisplayName: e.DisplayName, MimeType: e.MimeType,
		SizeBytes: e.SizeBytes, DigestSHA256: e.DigestSHA256, Status: status,
	}
}

// Import stores signature-verified WAV/MP3 bytes under size, count, and
// total caps. Re-import of identical bytes updates the display name —
// one asset per digest, never a duplicate.
func (s *AssetStore) Import(bytes []byte, meta AssetImportMeta) (AssetDescriptor, error) {
	s.mu.Lock()
	defer s.mu.Unlock()
	sniffed := sniffMime(bytes)
	if sniffed == "" {
		return AssetDescriptor{}, speechErr(CodeInvalidAudio, "bytes are not WAV or MP3")
	}
	declared := normalizeMime(meta.MimeType)
	if declared == "" {
		return AssetDescriptor{}, invalidInput("unsupported mime_type %q", meta.MimeType)
	}
	if declared != sniffed {
		return AssetDescriptor{}, speechErr(CodeInvalidAudio,
			fmt.Sprintf("mime_type %s does not match bytes (%s)", declared, sniffed))
	}
	if len(bytes) == 0 || len(bytes) > AssetMaxFileBytes {
		return AssetDescriptor{}, speechErr(CodeInvalidAudio,
			fmt.Sprintf("asset size %d outside 1..=%d", len(bytes), AssetMaxFileBytes))
	}
	if strings.TrimSpace(meta.DisplayName) == "" || len(meta.DisplayName) > AssetMaxDisplayName {
		return AssetDescriptor{}, invalidInput("display_name empty or overlong")
	}
	sum := sha256.Sum256(bytes)
	digest := hex.EncodeToString(sum[:])
	assetID := "va-" + digest[:16]
	for i := range s.in.manifest.Assets {
		if s.in.manifest.Assets[i].AssetID == assetID {
			s.in.manifest.Assets[i].DisplayName = meta.DisplayName
			s.in.manifest.Assets[i].DeletePending = false
			if err := s.persistLocked(); err != nil {
				return AssetDescriptor{}, err
			}
			return descriptorOf(s.in.manifest.Assets[i]), nil
		}
	}
	if len(s.in.manifest.Assets) >= AssetMaxFiles {
		return AssetDescriptor{}, speechErr(CodeInvalidInput,
			fmt.Sprintf("asset count %d at cap %d", len(s.in.manifest.Assets), AssetMaxFiles))
	}
	var total uint64
	for _, a := range s.in.manifest.Assets {
		total += a.SizeBytes
	}
	if total+uint64(len(bytes)) > AssetMaxTotalBytes {
		return AssetDescriptor{}, speechErr(CodeInvalidInput, "asset store total cap exceeded")
	}
	entry := assetEntry{
		AssetID: assetID, DisplayName: meta.DisplayName, MimeType: sniffed,
		SizeBytes: uint64(len(bytes)), DigestSHA256: digest,
	}
	if err := atomicWrite(filepath.Join(s.dir, assetFileName(assetID, sniffed)), bytes); err != nil {
		return AssetDescriptor{}, err
	}
	s.in.manifest.Assets = append(s.in.manifest.Assets, entry)
	if err := s.persistLocked(); err != nil {
		return AssetDescriptor{}, err
	}
	return descriptorOf(entry), nil
}

// List returns every known asset descriptor.
func (s *AssetStore) List() ([]AssetDescriptor, error) {
	s.mu.Lock()
	defer s.mu.Unlock()
	out := make([]AssetDescriptor, 0, len(s.in.manifest.Assets))
	for _, e := range s.in.manifest.Assets {
		out = append(out, descriptorOf(e))
	}
	sort.Slice(out, func(i, j int) bool { return out[i].AssetID < out[j].AssetID })
	return out, nil
}

// Exists is the known-asset check for config validation — pending
// deletes do not count (a new config must not pin a dying asset).
func (s *AssetStore) Exists(assetID string) bool {
	if ValidateAssetID(assetID) != nil {
		return false
	}
	s.mu.Lock()
	defer s.mu.Unlock()
	for _, a := range s.in.manifest.Assets {
		if a.AssetID == assetID && !a.DeletePending {
			return true
		}
	}
	return false
}

// Read returns bytes for explicit local preview — takes a lease so a
// delete issued mid-read completes only on release.
func (s *AssetStore) Read(assetID string) (*AssetLease, error) {
	if err := ValidateAssetID(assetID); err != nil {
		return nil, err
	}
	s.mu.Lock()
	defer s.mu.Unlock()
	var entry *assetEntry
	for i := range s.in.manifest.Assets {
		if s.in.manifest.Assets[i].AssetID == assetID && !s.in.manifest.Assets[i].DeletePending {
			entry = &s.in.manifest.Assets[i]
			break
		}
	}
	if entry == nil {
		return nil, speechErr(CodeAssetNotFound, "unknown asset_id")
	}
	path := filepath.Join(s.dir, assetFileName(entry.AssetID, entry.MimeType))
	info, err := os.Lstat(path)
	if err != nil {
		return nil, speechErr(CodeAssetNotFound, "asset file missing")
	}
	if !info.Mode().IsRegular() {
		return nil, speechErr(CodeAssetNotFound, "asset file is not a regular file")
	}
	bytes, err := os.ReadFile(path)
	if err != nil {
		return nil, invalidInput("read asset: %v", err)
	}
	s.in.leases[assetID]++
	return &AssetLease{store: s, id: assetID, Bytes: bytes, MimeType: entry.MimeType}, nil
}

// Delete is immediate when unleased; "pending" while a reader holds a
// lease — the last release completes the removal.
func (s *AssetStore) Delete(assetID string) (string, error) {
	if err := ValidateAssetID(assetID); err != nil {
		return "", err
	}
	s.mu.Lock()
	defer s.mu.Unlock()
	pos := -1
	for i := range s.in.manifest.Assets {
		if s.in.manifest.Assets[i].AssetID == assetID {
			pos = i
			break
		}
	}
	if pos < 0 {
		return "", speechErr(CodeAssetNotFound, "unknown asset_id")
	}
	if s.in.leases[assetID] > 0 {
		s.in.manifest.Assets[pos].DeletePending = true
		if err := s.persistLocked(); err != nil {
			return "", err
		}
		return "pending", nil
	}
	entry := s.in.manifest.Assets[pos]
	s.in.manifest.Assets = append(s.in.manifest.Assets[:pos], s.in.manifest.Assets[pos+1:]...)
	if err := os.Remove(filepath.Join(s.dir, assetFileName(entry.AssetID, entry.MimeType))); err != nil {
		return "", invalidInput("remove asset: %v", err)
	}
	if err := s.persistLocked(); err != nil {
		return "", err
	}
	return "deleted", nil
}

func (s *AssetStore) releaseLease(assetID string) {
	s.mu.Lock()
	defer s.mu.Unlock()
	count := s.in.leases[assetID]
	if count <= 1 {
		delete(s.in.leases, assetID)
		for i := range s.in.manifest.Assets {
			if s.in.manifest.Assets[i].AssetID == assetID && s.in.manifest.Assets[i].DeletePending {
				entry := s.in.manifest.Assets[i]
				s.in.manifest.Assets = append(s.in.manifest.Assets[:i], s.in.manifest.Assets[i+1:]...)
				_ = os.Remove(filepath.Join(s.dir, assetFileName(entry.AssetID, entry.MimeType)))
				_ = s.persistLocked()
				break
			}
		}
		return
	}
	s.in.leases[assetID] = count - 1
}
