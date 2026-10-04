package speech

// DN-6B closed provider adapters: SiliconFlow STT (multipart WAV) and
// SiliconFlow/MiniMax TTS (JSON). Request shapes are the DN-0S
// probe-pinned mappings — raw model IDs, no translation parameter, no
// retries/failover, no cross-host redirects (the shared client disables
// redirect following entirely).
//
// Outbound URLs: HTTPS only. A plain-http endpoint is accepted ONLY for
// loopback hosts so the offline fixture server can exercise the lane —
// controlled by allowInsecureLoopback, which the shell never sets.

import (
	"bytes"
	"context"
	"encoding/base64"
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"mime/multipart"
	"net"
	"net/http"
	"net/url"
	"strings"
	"time"
	"unicode/utf8"
)

const (
	STTJSONMaxBytes     = 64 * 1024
	STTTextMaxChars     = 16 * 1024
	TTSTextMaxChars     = 4000
	TTSTextMaxBytes     = 16 * 1024
	MP3MaxBytes         = 16 * 1024 * 1024
	MiniMaxJSONMaxBytes = 34 * 1024 * 1024

	connectTimeout = 10 * time.Second
	totalTimeout   = 120 * time.Second
)

func providerErr(msg string) *SpeechError { return speechErr(CodeProviderError, msg) }

// checkRequestURL enforces: no credentials/query/fragment (already
// enforced on base_url), and HTTPS unless the host is loopback and the
// caller explicitly allowed insecure loopback (fixture servers only).
func checkRequestURL(rawURL string, allowInsecureLoopback bool) error {
	u, err := url.Parse(rawURL)
	if err != nil || u.Scheme == "" {
		return providerErr("endpoint has no scheme")
	}
	switch u.Scheme {
	case "https":
	case "http":
		host := u.Hostname()
		loopback := host == "127.0.0.1" || host == "localhost" || host == "::1"
		if !(allowInsecureLoopback && loopback) {
			return providerErr("provider endpoint must be HTTPS")
		}
	default:
		return providerErr("provider endpoint must be HTTPS")
	}
	return nil
}

func mapTransport(err error) *SpeechError {
	var ne net.Error
	if errors.Is(err, context.DeadlineExceeded) || (errors.As(err, &ne) && ne.Timeout()) {
		return speechErr(CodeTimeout, "provider request timed out").retryable(true)
	}
	var ue *url.Error
	if errors.As(err, &ue) && ue.Err != nil {
		var opErr *net.OpError
		if errors.As(ue.Err, &opErr) {
			return providerErr(fmt.Sprintf("provider transport failed: %v", opErr.Op)).retryable(true)
		}
	}
	return providerErr(fmt.Sprintf("provider transport failed: %v", err))
}

// readCapped reads a response body with a hard byte cap; oversize is an
// error, not a truncation.
func readCapped(resp *http.Response, cap int, provider Provider) ([]byte, int, error) {
	status := resp.StatusCode
	buf, err := io.ReadAll(io.LimitReader(resp.Body, int64(cap)+1))
	if err != nil {
		return nil, status, mapTransport(err).provider(provider)
	}
	if len(buf) > cap {
		return nil, status, providerErr("provider response exceeds byte cap").
			provider(provider).httpStatus(status)
	}
	return buf, status, nil
}

func mp3Magic(b []byte) bool {
	return bytes.HasPrefix(b, []byte("ID3")) ||
		(len(b) >= 2 && b[0] == 0xFF && b[1]&0xE0 == 0xE0)
}

func newHTTPClient() *http.Client {
	return &http.Client{
		Timeout: totalTimeout,
		Transport: &http.Transport{
			DialContext: (&net.Dialer{Timeout: connectTimeout}).DialContext,
		},
		CheckRedirect: func(req *http.Request, via []*http.Request) error {
			return http.ErrUseLastResponse // never follow redirects
		},
	}
}

func doProvider(ctx context.Context, client *http.Client, req *http.Request) (*http.Response, error) {
	resp, err := client.Do(req.WithContext(ctx))
	if err != nil {
		return nil, err
	}
	return resp, nil
}

// siliconflowSTT: POST {base}/v1/audio/transcriptions, multipart
// file=<wav> + model. Success JSON {text}; the response body is capped
// at 64 KiB and the text at 16 KiB.
func siliconflowSTT(ctx context.Context, client *http.Client, cfg *SttPreferences, key string, wav []byte, allowInsecureLoopback bool) (string, uint64, error) {
	url := strings.TrimSuffix(cfg.BaseURL, "/") + "/v1/audio/transcriptions"
	if err := checkRequestURL(url, allowInsecureLoopback); err != nil {
		return "", 0, err
	}
	var body bytes.Buffer
	mw := multipart.NewWriter(&body)
	part, err := mw.CreatePart(map[string][]string{
		"Content-Disposition": {`form-data; name="file"; filename="audio.wav"`},
		"Content-Type":        {"audio/wav"},
	})
	if err != nil {
		return "", 0, providerErr(fmt.Sprintf("multipart: %v", err))
	}
	if _, err := part.Write(wav); err != nil {
		return "", 0, providerErr(fmt.Sprintf("multipart: %v", err))
	}
	if err := mw.WriteField("model", cfg.Model); err != nil {
		return "", 0, providerErr(fmt.Sprintf("multipart: %v", err))
	}
	if err := mw.Close(); err != nil {
		return "", 0, providerErr(fmt.Sprintf("multipart: %v", err))
	}
	req, err := http.NewRequest(http.MethodPost, url, &body)
	if err != nil {
		return "", 0, providerErr(fmt.Sprintf("request build: %v", err))
	}
	req.Header.Set("Authorization", "Bearer "+key)
	req.Header.Set("Content-Type", mw.FormDataContentType())
	resp, err := doProvider(ctx, client, req)
	if err != nil {
		return "", 0, mapTransport(err).provider(ProviderSiliconFlow)
	}
	defer resp.Body.Close()
	raw, status, err := readCapped(resp, STTJSONMaxBytes, ProviderSiliconFlow)
	if err != nil {
		return "", 0, err
	}
	if status < 200 || status >= 300 {
		return "", 0, providerErr("STT request rejected").
			provider(ProviderSiliconFlow).httpStatus(status).
			retryable(status == 429 || status >= 500)
	}
	var doc struct {
		Text string `json:"text"`
	}
	if err := json.Unmarshal(raw, &doc); err != nil {
		return "", 0, providerErr(fmt.Sprintf("malformed STT JSON: %v", err)).
			provider(ProviderSiliconFlow).httpStatus(status)
	}
	if utf8.RuneCountInString(doc.Text) > STTTextMaxChars {
		return "", 0, providerErr("STT text exceeds 16 KiB cap").
			provider(ProviderSiliconFlow).httpStatus(status)
	}
	return doc.Text, uint64(len(raw)), nil
}

// siliconflowTTS: POST {base}/v1/audio/speech. The voice union is
// system (`voice` string) XOR reusable `speech:<name>:<id>` uri XOR
// inline `references` built from an owned asset lease — the lease must
// outlive the request.
func siliconflowTTS(ctx context.Context, client *http.Client, cfg *SiliconFlowTts, key, text string, inline *AssetLease, allowInsecureLoopback bool) ([]byte, uint64, error) {
	url := strings.TrimSuffix(cfg.BaseURL, "/") + "/v1/audio/speech"
	if err := checkRequestURL(url, allowInsecureLoopback); err != nil {
		return nil, 0, err
	}
	payload := map[string]any{
		"model":           cfg.Model,
		"input":           text,
		"response_format": "mp3",
		"stream":          false,
	}
	switch {
	case cfg.Reference != nil && !cfg.Reference.isSystem && cfg.Reference.VoiceID == "":
		if inline == nil {
			return nil, 0, speechErr(CodeAssetNotFound,
				fmt.Sprintf("reference asset %s not leased", cfg.Reference.AssetID))
		}
		b64 := base64.StdEncoding.EncodeToString(inline.Bytes)
		payload["references"] = []map[string]any{{
			"audio": fmt.Sprintf("data:%s;base64,%s", inline.MimeType, b64),
			"text":  cfg.Reference.Transcript,
		}}
	case cfg.Reference != nil && cfg.Reference.VoiceID != "":
		payload["voice"] = cfg.Reference.VoiceID
	default:
		payload["voice"] = cfg.Voice
	}
	raw, err := json.Marshal(payload)
	if err != nil {
		return nil, 0, providerErr(fmt.Sprintf("encode payload: %v", err))
	}
	req, err := http.NewRequest(http.MethodPost, url, bytes.NewReader(raw))
	if err != nil {
		return nil, 0, providerErr(fmt.Sprintf("request build: %v", err))
	}
	req.Header.Set("Authorization", "Bearer "+key)
	req.Header.Set("Content-Type", "application/json")
	resp, err := doProvider(ctx, client, req)
	if err != nil {
		return nil, 0, mapTransport(err).provider(ProviderSiliconFlow)
	}
	defer resp.Body.Close()
	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		status := resp.StatusCode
		_, _ = io.Copy(io.Discard, io.LimitReader(resp.Body, int64(STTJSONMaxBytes)))
		return nil, 0, providerErr("TTS request rejected").
			provider(ProviderSiliconFlow).httpStatus(status).
			retryable(status == 429 || status >= 500)
	}
	body, status, err := readCapped(resp, MP3MaxBytes, ProviderSiliconFlow)
	if err != nil {
		return nil, 0, err
	}
	if !mp3Magic(body) {
		return nil, 0, providerErr("provider body is not MP3").
			provider(ProviderSiliconFlow).httpStatus(status)
	}
	return body, uint64(len(body)), nil
}

// minimaxTTS: POST {base}/v1/t2a_v2. HTTP 200 still requires
// base_resp.status_code == 0 and data.status == 2; data.audio is hex
// MP3 — malformed hex or oversize decode is a provider_error, never
// successful audio.
func minimaxTTS(ctx context.Context, client *http.Client, cfg *MiniMaxTts, key, text string, allowInsecureLoopback bool) ([]byte, uint64, error) {
	url := strings.TrimSuffix(cfg.BaseURL, "/") + "/v1/t2a_v2"
	if err := checkRequestURL(url, allowInsecureLoopback); err != nil {
		return nil, 0, err
	}
	payload := map[string]any{
		"model":  cfg.Model,
		"text":   text,
		"stream": false,
		"voice_setting": map[string]any{
			"voice_id": cfg.VoiceID,
			"speed":    cfg.Speed,
			"vol":      cfg.Volume,
		},
		"audio_setting": map[string]any{
			"sample_rate": 32000,
			"bitrate":     128000,
			"format":      "mp3",
			"channel":     1,
		},
	}
	raw, err := json.Marshal(payload)
	if err != nil {
		return nil, 0, providerErr(fmt.Sprintf("encode payload: %v", err))
	}
	req, err := http.NewRequest(http.MethodPost, url, bytes.NewReader(raw))
	if err != nil {
		return nil, 0, providerErr(fmt.Sprintf("request build: %v", err))
	}
	req.Header.Set("Authorization", "Bearer "+key)
	req.Header.Set("Content-Type", "application/json")
	resp, err := doProvider(ctx, client, req)
	if err != nil {
		return nil, 0, mapTransport(err).provider(ProviderMiniMax)
	}
	defer resp.Body.Close()
	body, status, err := readCapped(resp, MiniMaxJSONMaxBytes, ProviderMiniMax)
	if err != nil {
		return nil, 0, err
	}
	if status < 200 || status >= 300 {
		return nil, 0, providerErr("TTS request rejected").
			provider(ProviderMiniMax).httpStatus(status).
			retryable(status == 429 || status >= 500)
	}
	var doc struct {
		BaseResp struct {
			StatusCode int `json:"status_code"`
		} `json:"base_resp"`
		Data struct {
			Status int    `json:"status"`
			Audio  string `json:"audio"`
		} `json:"data"`
	}
	if err := json.Unmarshal(body, &doc); err != nil {
		return nil, 0, providerErr(fmt.Sprintf("malformed MiniMax JSON: %v", err)).
			provider(ProviderMiniMax).httpStatus(status)
	}
	if doc.BaseResp.StatusCode != 0 {
		return nil, 0, providerErr("MiniMax business error").
			provider(ProviderMiniMax).httpStatus(status)
	}
	if doc.Data.Status != 2 {
		return nil, 0, providerErr("MiniMax audio not ready").
			provider(ProviderMiniMax).httpStatus(status)
	}
	mp3, err := hex.DecodeString(doc.Data.Audio)
	if err != nil {
		return nil, 0, providerErr("malformed MiniMax audio hex").
			provider(ProviderMiniMax).httpStatus(status)
	}
	if len(mp3) > MP3MaxBytes {
		return nil, 0, providerErr("MP3 exceeds 16 MiB cap").
			provider(ProviderMiniMax).httpStatus(status)
	}
	if !mp3Magic(mp3) {
		return nil, 0, providerErr("provider body is not MP3").
			provider(ProviderMiniMax).httpStatus(status)
	}
	return mp3, uint64(len(body)), nil
}
