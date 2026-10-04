package speech

import (
	"errors"
	"strings"

	"github.com/zalando/go-keyring"
)

// OS credential slots for speech providers. Keys live only in the
// platform store, keyed by a FIXED namespace derived at startup — never
// by arbitrary service/profile strings from the wire. Readback is
// presence-only; an unavailable or locked store surfaces
// credential_unavailable with no plaintext/env/sample fallback.

// Fixed keyring service namespace — one app, one profile.
const credentialService = "dev.projectivy.diva.speech"

func credentialSlot(p Provider) string { return "v1." + p.String() }

// SecretStore abstracts what a store may be; injected for tests.
type SecretStore interface {
	Set(service, user, secret string) error
	Get(service, user string) (string, error)
	Delete(service, user string) error
}

func mapStoreError(err error) *SpeechError {
	// ErrUnsupportedPlatform / platform failures are environment facts —
	// not retryable inside this process; transient store errors are.
	retryable := !errors.Is(err, keyring.ErrUnsupportedPlatform)
	return speechErr(CodeCredentialUnavailable, "OS credential store unavailable").retryable(retryable)
}

// OsKeyring is the production store backed by zalando/go-keyring.
type OsKeyring struct{}

func (OsKeyring) Set(service, user, secret string) error {
	if err := keyring.Set(service, user, secret); err != nil {
		return mapStoreError(err)
	}
	return nil
}

func (OsKeyring) Get(service, user string) (string, error) {
	v, err := keyring.Get(service, user)
	if errors.Is(err, keyring.ErrNotFound) {
		return "", nil
	}
	if err != nil {
		return "", mapStoreError(err)
	}
	return v, nil
}

func (OsKeyring) Delete(service, user string) error {
	if err := keyring.Delete(service, user); err != nil && !errors.Is(err, keyring.ErrNotFound) {
		return mapStoreError(err)
	}
	return nil
}

// CredentialPresence is the presence readback state — never the secret.
type CredentialPresence string

const (
	PresencePresent     CredentialPresence = "present"
	PresenceAbsent      CredentialPresence = "absent"
	PresenceUnavailable CredentialPresence = "unavailable"
)

// CredentialStore wraps a SecretStore with the fixed namespace and
// provider slots.
type CredentialStore struct {
	store SecretStore
}

func NewCredentialStore(store SecretStore) *CredentialStore {
	return &CredentialStore{store: store}
}

// Set stores a provider key; the value is never echoed anywhere. `key`
// is bounded and must be non-empty — it is still never logged or
// returned.
func (s *CredentialStore) Set(provider Provider, key string) error {
	if strings.TrimSpace(key) == "" || len(key) > 512 {
		return speechErr(CodeInvalidInput, "credential empty or overlong")
	}
	if err := s.store.Set(credentialService, credentialSlot(provider), key); err != nil {
		if se, ok := err.(*SpeechError); ok {
			return se.provider(provider)
		}
		return err
	}
	return nil
}

// getSecret is the secret readback for the request lane only — key
// material is handed to the provider adapter at admission and never
// logged, echoed, or persisted outside the OS store.
func (s *CredentialStore) getSecret(provider Provider) (string, error) {
	v, err := s.store.Get(credentialService, credentialSlot(provider))
	if err != nil {
		if se, ok := err.(*SpeechError); ok {
			return "", se.provider(provider)
		}
		return "", err
	}
	if v == "" {
		return "", speechErr(CodeNotConfigured, "no provider credential configured").provider(provider)
	}
	return v, nil
}

// Presence is the presence-only readback for speech_config_get — no key
// material.
func (s *CredentialStore) Presence(provider Provider) CredentialPresence {
	v, err := s.store.Get(credentialService, credentialSlot(provider))
	if err != nil {
		return PresenceUnavailable
	}
	if v == "" {
		return PresenceAbsent
	}
	return PresencePresent
}

// Delete is idempotent — absent is a success.
func (s *CredentialStore) Delete(provider Provider) error {
	if err := s.store.Delete(credentialService, credentialSlot(provider)); err != nil {
		if se, ok := err.(*SpeechError); ok {
			return se.provider(provider)
		}
		return err
	}
	return nil
}
