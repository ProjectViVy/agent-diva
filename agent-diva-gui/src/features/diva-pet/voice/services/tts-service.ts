import { invoke } from '@tauri-apps/api/core'
import { addVoiceLogEvent } from './voice-log'
import { asSiliconFlowProviderHandler, createTTSProviderHandler } from './tts/provider-factory'
import {
  DEFAULT_SPEED,
  PROVIDER_DEFAULTS,
  PROVIDER_VOICE_DEFAULTS,
  type RemoteVoiceListResponse,
  type RemoteVoiceUploadResponse,
  type SiliconFlowTTSProviderHandler,
  type TTSProvider,
  type TTSProviderFactoryContext,
  type TTSProviderHandler,
  type TTSRequest,
  type TTSResponse,
  type TTSVoiceConfig,
  type VoiceFileReader,
} from './tts/types'

export type {
  TTSProvider,
  TTSRequest,
  TTSResponse,
  TTSVoiceConfig,
  VoiceFileReader,
} from './tts/types'

type TTSRequestErrorKind = "clone_voice_invalid" | "http" | "network" | "timeout";

interface TTSRequestPolicy {
  maxRetries: number;
  purpose: string;
  timeoutMs: number;
}

interface TTSRequestErrorOptions {
  cause?: unknown;
  endpoint: string;
  kind: TTSRequestErrorKind;
  responseBody?: string;
  retryable: boolean;
  status?: number;
}

class TTSRequestError extends Error {
  readonly endpoint: string;
  readonly kind: TTSRequestErrorKind;
  readonly responseBody?: string;
  readonly retryable: boolean;
  readonly status?: number;

  constructor(message: string, options: TTSRequestErrorOptions) {
    super(message);
    this.name = "TTSRequestError";
    this.endpoint = options.endpoint;
    this.kind = options.kind;
    this.responseBody = options.responseBody;
    this.retryable = options.retryable;
    this.status = options.status;

    if (options.cause !== undefined) {
      (this as Error & { cause?: unknown }).cause = options.cause;
    }
  }
}

const DEFAULT_RETRYABLE_STATUSES = new Set([408, 425, 429, 500, 502, 503, 504]);
const VOICE_LIST_REQUEST_POLICY: TTSRequestPolicy = {
  maxRetries: 1,
  purpose: "voice list request",
  timeoutMs: 8000,
};
const VOICE_UPLOAD_REQUEST_POLICY: TTSRequestPolicy = {
  maxRetries: 1,
  purpose: "voice upload request",
  timeoutMs: 20000,
};
const VOICE_SYNTHESIS_REQUEST_POLICY: TTSRequestPolicy = {
  maxRetries: 1,
  purpose: "voice synthesis request",
  timeoutMs: 30000,
};


class TTSService {
  private currentAudio: HTMLAudioElement | null = null;
  private isPlaying = false;
  private cachedCloneVoice:
    | {
        key: string;
        uri: string;
      }
    | null = null;
  private pendingCloneVoiceKey: string | null = null;
  private pendingCloneVoicePromise: Promise<string | null> | null = null;
  private voiceFileReader: VoiceFileReader | null = null;

  /**
   * Inject a {@link VoiceFileReader} implementation used for reading
   * voice reference files (inline reference cloning and voice upload).
   *
   * When no reader is set (default), inline reference cloning and
   * voice upload are gracefully skipped.
   */
  setVoiceFileReader(reader: VoiceFileReader): void {
    this.voiceFileReader = reader;
  }

  /**
   * Pre-warm the reusable voice clone URI so the first synthesis call
   * does not need to wait for the upload / lookup round-trip.
   *
   * Safe to call even when voice cloning is not configured — the call
   * is silently skipped.
   */
  async prepareVoiceClone(voiceConfig: TTSVoiceConfig | null | undefined): Promise<void> {
    if (!this.supportsReusableVoiceCloning(voiceConfig)) {
      this.clearCloneVoiceCache();
      return;
    }

    await this.ensureCloneVoiceUri(voiceConfig);
  }

  /**
   * Synthesize speech from text using the configured TTS provider.
   *
   * Fallback chain (when `referenceVoice` is set):
   *   1. Reusable voice cloning (SiliconFlow)
   *   2. Inline reference cloning (SiliconFlow)
   *   3. Standard API TTS (any provider)
   *   4. Browser SpeechSynthesis
   *
   * @returns a {@link TTSResponse} with an object URL, or `null` if
   *          synthesis was skipped (e.g. disabled or no API key).
   */
  async synthesize(
    request: TTSRequest,
    voiceConfig: TTSVoiceConfig,
  ): Promise<TTSResponse | null> {
    const startTime = performance.now();
    const provider = voiceConfig.provider as TTSProvider;

    console.log("[TTSService] TTS start:", {
      enabled: voiceConfig.enabled,
      hasReferenceVoice: !!voiceConfig.referenceVoice,
      provider,
      text: `${request.text.slice(0, 30)}...`,
    });
    addVoiceLogEvent({
      level: "info",
      source: "tts",
      message: "开始 TTS 合成",
      detail: {
        provider,
        model: voiceConfig.model,
        baseUrlHost: this.safeHost(voiceConfig.baseUrl),
        hasApiKey: !!voiceConfig.apiKey,
        hasReferenceVoice: !!voiceConfig.referenceVoice,
      },
    });

    if (!voiceConfig.enabled) {
      return null;
    }

    if (provider === "browser") {
      const result = await this.synthesizeWithBrowser(request);
      console.log(
        `[TTSService] Browser TTS finished in ${(performance.now() - startTime).toFixed(0)}ms`,
      );
      return result;
    }

    if (provider === "minimax") {
      const minimaxHandler = this.getProviderHandler("minimax");
      try {
        if (!minimaxHandler) {
          throw new Error("MiniMax TTS provider factory is unavailable.");
        }
        const result = await minimaxHandler.synthesize(request, voiceConfig);
        console.log(
          `[TTSService] MiniMax TTS finished in ${(performance.now() - startTime).toFixed(0)}ms`,
        );
        return result;
      } catch (error) {
        console.error("[TTSService] MiniMax TTS failed, falling back to browser TTS.", error);
        addVoiceLogEvent({
          level: "warn",
          source: "tts",
          message: "MiniMax TTS 失败，回退到浏览器播报",
          detail: { provider, error: this.formatUnknownError(error) },
        });
        return this.synthesizeWithBrowser(request);
      }
    }

    if (voiceConfig.referenceVoice) {
      if (provider === "siliconflow") {
        try {
          const result = await this.synthesizeWithVoiceCloning(request, voiceConfig);
          console.log(
            `[TTSService] Voice cloning finished in ${(performance.now() - startTime).toFixed(0)}ms`,
          );
          return result;
        } catch (error) {
          console.error(
            "[TTSService] Voice cloning failed, falling back to standard TTS.",
            error,
          );
        }
      } else {
        console.warn(
          `[TTSService] Provider "${provider}" does not support voice cloning. Falling back to standard TTS.`,
        );
      }
    }

    try {
      const result = await this.synthesizeWithAPI(request, voiceConfig);
      console.log(
        `[TTSService] Standard TTS finished in ${(performance.now() - startTime).toFixed(0)}ms`,
      );
      return result;
    } catch (error) {
      console.error("[TTSService] API TTS failed, falling back to browser TTS.", error);
      addVoiceLogEvent({
        level: "warn",
        source: "tts",
        message: "API TTS 失败，回退到浏览器播报",
        detail: { provider, error: this.formatUnknownError(error) },
      });
      return this.synthesizeWithBrowser(request);
    }
  }

  /**
   * Play an audio blob URL and wait until playback ends or fails.
   *
   * @throws Error if audio playback fails.
   */
  async playAudio(audioUrl: string): Promise<void> {
    if (this.currentAudio) {
      this.currentAudio.pause();
      this.currentAudio = null;
    }

    return new Promise((resolve, reject) => {
      const audio = new Audio(audioUrl);
      this.currentAudio = audio;
      this.isPlaying = true;

      audio.onended = () => {
        this.isPlaying = false;
        this.currentAudio = null;
        resolve();
      };

      audio.onerror = () => {
        this.isPlaying = false;
        this.currentAudio = null;
        reject(new Error("Audio playback failed"));
      };

      audio.play().catch(reject);
    });
  }

  /**
   * High-level convenience: synthesize text and play the result
   * automatically. Falls back to browser SpeechSynthesis when API
   * TTS is unavailable or fails.
   *
   * @param text        Text to speak.
   * @param voiceConfig Optional voice configuration. When omitted or
   *                    disabled, browser TTS is used directly.
   * @param onComplete  Optional callback invoked after audio finishes.
   */
  async speakText(
    text: string,
    voiceConfig?: TTSVoiceConfig,
    onComplete?: () => void,
  ): Promise<void> {
    const resolvedSpeed = voiceConfig?.speed ?? DEFAULT_SPEED;
    const resolvedVolume = voiceConfig?.volume ?? 1.0;

    if (!voiceConfig?.enabled) {
      await this.speakWithBrowserAsync(text, resolvedSpeed, resolvedVolume);
      onComplete?.();
      return;
    }

    if (voiceConfig.provider === "browser") {
      await this.speakWithBrowserAsync(text, resolvedSpeed, resolvedVolume);
      onComplete?.();
      return;
    }

    try {
      const response = await this.synthesize({ speed: resolvedSpeed, text }, voiceConfig);
      if (response?.audioUrl) {
        await this.playAudio(response.audioUrl);
      } else {
        addVoiceLogEvent({
          level: "warn",
          source: "tts",
          message: "远程 TTS 未返回音频，回退到浏览器播报",
          detail: { provider: voiceConfig.provider },
        });
        await this.speakWithBrowserAsync(text, resolvedSpeed, resolvedVolume);
      }
    } catch (error) {
      console.error("[TTSService] Failed to speak with API TTS, falling back.", error);
      addVoiceLogEvent({
        level: "error",
        source: "tts",
        message: "TTS 播放失败，已回退到浏览器播报",
        detail: { provider: voiceConfig.provider, error: this.formatUnknownError(error) },
      });
      await this.speakWithBrowserAsync(text, resolvedSpeed, resolvedVolume);
    }

    onComplete?.();
  }

  /**
   * Stop any currently playing audio and cancel pending browser
   * SpeechSynthesis utterances.
   */
  stopPlayback(): void {
    if (this.currentAudio) {
      this.currentAudio.pause();
      this.currentAudio = null;
    }
    this.isPlaying = false;

    if (window.speechSynthesis) {
      window.speechSynthesis.cancel();
    }
  }

  /**
   * Returns `true` when an audio clip is currently playing.
   */
  isCurrentlyPlaying(): boolean {
    return this.isPlaying;
  }

  /**
   * Returns the currently-playing HTMLAudioElement, or null.
   *
   * Used by the lip sync system to connect a MediaElementAudioSourceNode
   * before playback starts. The element reference is valid from the moment
   * `playAudio()` creates it until playback ends.
   */
  getCurrentAudioElement(): HTMLAudioElement | null {
    return this.currentAudio;
  }

  // ---------------------------------------------------------------------------
  // Private synthesis helpers
  // ---------------------------------------------------------------------------

  private async synthesizeWithVoiceCloning(
    request: TTSRequest,
    voiceConfig: TTSVoiceConfig,
  ): Promise<TTSResponse | null> {
    const apiKey = voiceConfig.apiKey;

    if (!apiKey) {
      console.warn("[TTSService] Voice cloning requires an API key.");
      addVoiceLogEvent({
        level: "warn",
        source: "tts",
        message: "克隆音色需要 API Key，跳过克隆链路",
        detail: { provider: "siliconflow" },
      });
      return null;
    }

    const cloneModel = "FunAudioLLM/CosyVoice2-0.5B";
    const siliconflowHandler = this.getSiliconFlowProviderHandler();
    const cloneVoiceUri = await this.ensureCloneVoiceUri(voiceConfig);

    if (!cloneVoiceUri) {
      console.warn(
        "[TTSService] Reusable cloned voice is unavailable, falling back to inline reference upload.",
      );
      return this.synthesizeWithInlineReferenceCloning(request, voiceConfig);
    }

    try {
      return await siliconflowHandler.synthesizeReusableClone(request, voiceConfig, {
        cloneVoiceUri,
        model: cloneModel,
      });
    } catch (error) {
      if (!this.isCloneVoiceInvalidationError(error)) {
        throw error;
      }

      console.warn(
        "[TTSService] Cached cloned voice URI was rejected. Rebuilding the reusable clone.",
        this.formatRequestError(error),
      );

      try {
        const rebuiltCloneVoiceUri = await this.rebuildCloneVoiceUri(
          voiceConfig,
          cloneVoiceUri,
        );

        if (rebuiltCloneVoiceUri) {
          return await siliconflowHandler.synthesizeReusableClone(request, voiceConfig, {
            cloneVoiceUri: rebuiltCloneVoiceUri,
            model: cloneModel,
          });
        }
      } catch (rebuildError) {
        console.warn(
          "[TTSService] Failed to rebuild the cloned voice URI. Falling back to inline reference upload.",
          this.formatRequestError(rebuildError),
        );
      }

      console.warn(
        "[TTSService] Rebuilt cloned voice URI is unavailable, falling back to inline reference upload.",
      );
      return this.synthesizeWithInlineReferenceCloning(request, voiceConfig);
    }
  }

  private async synthesizeWithInlineReferenceCloning(
    request: TTSRequest,
    voiceConfig: TTSVoiceConfig,
  ): Promise<TTSResponse | null> {
    const apiKey = voiceConfig.apiKey;
    const referenceVoice = voiceConfig.referenceVoice;

    if (!apiKey || !referenceVoice) {
      return null;
    }

    // Graceful degradation when no VoiceFileReader is configured
    if (!this.voiceFileReader) {
      console.warn(
        "[TTSService] No VoiceFileReader configured. Skipping inline reference cloning.",
      );
      addVoiceLogEvent({
        level: "warn",
        source: "tts",
        message: "未配置音色文件读取器，跳过内联参考音色",
      });
      return null;
    }

    const voiceFileData = await this.voiceFileReader.readVoiceFile(referenceVoice);
    if (!voiceFileData) {
      console.warn(
        `[TTSService] Voice file "${referenceVoice}" could not be read.`,
      );
      addVoiceLogEvent({
        level: "error",
        source: "tts",
        message: "参考音色文件读取失败",
        detail: { path: referenceVoice },
      });
      return null;
    }

    const cloneModel = "FunAudioLLM/CosyVoice2-0.5B";
    const siliconflowHandler = this.getSiliconFlowProviderHandler();

    return siliconflowHandler.synthesizeInlineClone(request, voiceConfig, {
      model: cloneModel,
      referenceAudio: `data:${voiceFileData.contentType};base64,${voiceFileData.base64Data}`,
      referenceText: voiceConfig.referenceText || "",
    });
  }

  private async synthesizeWithAPI(
    request: TTSRequest,
    voiceConfig: TTSVoiceConfig,
  ): Promise<TTSResponse | null> {
    const provider = voiceConfig.provider as TTSProvider;
    const apiKey = voiceConfig.apiKey;

    if (!apiKey) {
      console.warn(`[TTSService] ${provider} voice API key is not configured.`);
      addVoiceLogEvent({
        level: "warn",
        source: "tts",
        message: "当前 TTS Provider 未填写 API Key，请先完成配置",
        detail: { provider },
      });
      return null;
    }

    // 硅基流动通过 Tauri Rust 后端代理，绕过浏览器 CORS 限制
    if (provider === "siliconflow") {
      return this.getSiliconFlowProviderHandler().synthesize(request, voiceConfig);
    }

    const { baseUrl, model } = this.resolveProviderConfig(
      provider,
      voiceConfig.baseUrl,
      voiceConfig.model,
    );
    const audioBlob = await this.requestBlob(
      `${baseUrl}/audio/speech`,
      {
        method: "POST",
        headers: {
          Authorization: `Bearer ${apiKey}`,
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          input: request.text,
          model,
          response_format: "mp3",
          speed: request.speed || voiceConfig.speed || DEFAULT_SPEED,
          voice: PROVIDER_VOICE_DEFAULTS[provider] || PROVIDER_VOICE_DEFAULTS.openai,
        }),
      },
      {
        ...VOICE_SYNTHESIS_REQUEST_POLICY,
        purpose: `${provider} TTS request`,
      },
    );

    return {
      audioUrl: URL.createObjectURL(audioBlob),
      durationMs: 0,
      isCloned: false,
    };
  }

  private async synthesizeWithBrowser(request: TTSRequest): Promise<TTSResponse | null> {
    if (!window.speechSynthesis) {
      return null;
    }

    return new Promise((resolve) => {
      const utterance = new SpeechSynthesisUtterance(request.text);
      utterance.lang = "zh-CN";
      utterance.rate = request.speed || DEFAULT_SPEED;
      utterance.volume = 1.0;

      const voices = window.speechSynthesis.getVoices();
      const chineseVoice = voices.find((voice) => voice.lang.includes("zh"));
      if (chineseVoice) {
        utterance.voice = chineseVoice;
      }

      utterance.onend = () => {
        resolve({
          audioUrl: "",
          durationMs: 0,
          isCloned: false,
        });
      };

      utterance.onerror = () => {
        resolve(null);
      };

      window.speechSynthesis.speak(utterance);
    });
  }

  // ---------------------------------------------------------------------------
  // Browser TTS convenience helpers
  // ---------------------------------------------------------------------------

  private speakWithBrowserAsync(text: string, speed = DEFAULT_SPEED, volume = 1.0): Promise<void> {
    return new Promise((resolve) => {
      if (!window.speechSynthesis) {
        resolve();
        return;
      }

      const utterance = new SpeechSynthesisUtterance(text);
      utterance.lang = "zh-CN";
      utterance.rate = speed;
      utterance.volume = Math.max(0, Math.min(1, volume));
      addVoiceLogEvent({
        level: "info",
        source: "tts",
        message: "使用浏览器语音播报",
        detail: { provider: "browser", speed, volume: utterance.volume },
      });

      const voices = window.speechSynthesis.getVoices();
      const chineseVoice = voices.find((voice) => voice.lang.includes("zh"));
      if (chineseVoice) {
        utterance.voice = chineseVoice;
      }

      utterance.onend = () => {
        resolve();
      };

      utterance.onerror = () => {
        resolve();
      };

      window.speechSynthesis.speak(utterance);
    });
  }

  // ---------------------------------------------------------------------------
  // Reusable voice cloning (SiliconFlow)
  // ---------------------------------------------------------------------------

  private async rebuildCloneVoiceUri(
    voiceConfig: TTSVoiceConfig,
    invalidUri: string,
  ): Promise<string | null> {
    const cloneVoiceKey = this.createCloneVoiceKey(voiceConfig);
    this.clearCloneVoiceCache();

    const listedUri = await this.findExistingCloneVoiceUri(voiceConfig, invalidUri);

    if (listedUri) {
      this.cachedCloneVoice = {
        key: cloneVoiceKey,
        uri: listedUri,
      };
      return listedUri;
    }

    const uploadedUri = await this.uploadCloneVoice(voiceConfig);

    if (uploadedUri) {
      this.cachedCloneVoice = {
        key: cloneVoiceKey,
        uri: uploadedUri,
      };
    }

    return uploadedUri;
  }

  private isCloneVoiceInvalidationError(error: unknown): error is TTSRequestError {
    return error instanceof TTSRequestError && error.kind === "clone_voice_invalid";
  }

  // ---------------------------------------------------------------------------
  // HTTP helpers
  // ---------------------------------------------------------------------------

  private async requestJson<T>(
    endpoint: string,
    init: RequestInit,
    policy: TTSRequestPolicy,
    classifyResponseError?: (status: number, responseBody: string) => TTSRequestError | null,
  ): Promise<T> {
    const response = await this.performRequest(
      endpoint,
      init,
      policy,
      classifyResponseError,
    );

    return (await response.json()) as T;
  }

  private async requestBlob(
    endpoint: string,
    init: RequestInit,
    policy: TTSRequestPolicy,
    classifyResponseError?: (status: number, responseBody: string) => TTSRequestError | null,
  ) {
    const response = await this.performRequest(
      endpoint,
      init,
      policy,
      classifyResponseError,
    );

    return response.blob();
  }

  private async performRequest(
    endpoint: string,
    init: RequestInit,
    policy: TTSRequestPolicy,
    classifyResponseError?: (status: number, responseBody: string) => TTSRequestError | null,
  ): Promise<Response> {
    for (let attempt = 0; attempt <= policy.maxRetries; attempt += 1) {
      const controller = new AbortController();
      const timeoutId = window.setTimeout(() => {
        controller.abort();
      }, policy.timeoutMs);

      try {
        const response = await fetch(endpoint, {
          ...init,
          signal: controller.signal,
        });

        window.clearTimeout(timeoutId);

        if (response.ok) {
          return response;
        }

        const responseBody = await response.text();
        const requestError =
          classifyResponseError?.(response.status, responseBody) ??
          this.createHttpRequestError(policy, endpoint, response.status, responseBody);

        if (attempt < policy.maxRetries && requestError.retryable) {
          this.logRequestRetry(policy, attempt + 1, requestError);
          continue;
        }

        throw requestError;
      } catch (error) {
        window.clearTimeout(timeoutId);

        const requestError = this.normalizeRequestError(error, policy, endpoint);

        if (attempt < policy.maxRetries && requestError.retryable) {
          this.logRequestRetry(policy, attempt + 1, requestError);
          continue;
        }

        throw requestError;
      }
    }

    throw new TTSRequestError(`Exhausted retries for ${policy.purpose}.`, {
      endpoint,
      kind: "network",
      retryable: false,
    });
  }

  private createHttpRequestError(
    policy: TTSRequestPolicy,
    endpoint: string,
    status: number,
    responseBody: string,
  ) {
    return new TTSRequestError(
      `${policy.purpose} failed with HTTP ${status}.`,
      {
        endpoint,
        kind: "http",
        responseBody,
        retryable: DEFAULT_RETRYABLE_STATUSES.has(status),
        status,
      },
    );
  }

  private normalizeRequestError(
    error: unknown,
    policy: TTSRequestPolicy,
    endpoint: string,
  ) {
    if (error instanceof TTSRequestError) {
      return error;
    }

    if (error instanceof DOMException && error.name === "AbortError") {
      return new TTSRequestError(
        `${policy.purpose} timed out after ${policy.timeoutMs}ms.`,
        {
          cause: error,
          endpoint,
          kind: "timeout",
          retryable: true,
        },
      );
    }

    if (error instanceof TypeError) {
      return new TTSRequestError(
        `${policy.purpose} failed because the network request could not be completed.`,
        {
          cause: error,
          endpoint,
          kind: "network",
          retryable: true,
        },
      );
    }

    return new TTSRequestError(`${policy.purpose} failed unexpectedly.`, {
      cause: error,
      endpoint,
      kind: "http",
      retryable: false,
    });
  }

  private logRequestRetry(
    policy: TTSRequestPolicy,
    nextAttempt: number,
    error: TTSRequestError,
  ) {
    console.warn(
      `[TTSService] ${policy.purpose} failed (${error.kind}). Retrying ${nextAttempt}/${policy.maxRetries}.`,
      this.formatRequestError(error),
    );
  }

  private formatRequestError(error: unknown) {
    if (!(error instanceof TTSRequestError)) {
      return error;
    }

    return {
      endpoint: error.endpoint,
      kind: error.kind,
      message: error.message,
      responseBody: error.responseBody,
      retryable: error.retryable,
      status: error.status,
    };
  }

  private formatUnknownError(error: unknown): string {
    if (error instanceof Error) return error.message;
    return String(error);
  }

  private safeHost(baseUrl: string): string {
    try {
      return new URL(baseUrl).host;
    } catch {
      return baseUrl.trim() ? "custom" : "";
    }
  }

  // ---------------------------------------------------------------------------
  // Voice clone lifecycle helpers
  // ---------------------------------------------------------------------------

  private supportsReusableVoiceCloning(
    voiceConfig: TTSVoiceConfig | null | undefined,
  ): voiceConfig is TTSVoiceConfig {
    return Boolean(
      voiceConfig?.enabled &&
        voiceConfig.provider === "siliconflow" &&
        voiceConfig.apiKey &&
        voiceConfig.referenceVoice,
    );
  }

  private clearCloneVoiceCache() {
    this.cachedCloneVoice = null;
    this.pendingCloneVoiceKey = null;
    this.pendingCloneVoicePromise = null;
  }

  private resolveProviderConfig(
    provider: string,
    baseUrl: string,
    model: string | null,
  ) {
    const fallbackDefaults = PROVIDER_DEFAULTS.siliconflow!;
    const defaults =
      PROVIDER_DEFAULTS[provider as keyof typeof PROVIDER_DEFAULTS] ??
      fallbackDefaults;

    return {
      baseUrl: baseUrl || defaults.baseUrl,
      model: model || defaults.model,
    };
  }

  private createProviderFactoryContext(): TTSProviderFactoryContext {
    return {
      createAudioResponse: (base64Data, contentType, isCloned = false) =>
        this.createAudioResponse(base64Data, contentType, isCloned),
      invokeCommand: <TResponse>(command: string, payload: unknown) =>
        invoke<TResponse>(command, { payload }),
      logEvent: addVoiceLogEvent,
      resolveProviderConfig: (provider, baseUrl, model) =>
        this.resolveProviderConfig(provider, baseUrl, model),
    };
  }

  private getProviderHandler(provider: TTSProvider): TTSProviderHandler | null {
    return createTTSProviderHandler(provider, this.createProviderFactoryContext());
  }

  private getSiliconFlowProviderHandler(): SiliconFlowTTSProviderHandler {
    return asSiliconFlowProviderHandler(this.getProviderHandler("siliconflow"));
  }

  private createAudioResponse(
    base64Data: string,
    contentType: string | null | undefined,
    isCloned = false,
  ): TTSResponse {
    const audioBlob = this.decodeBase64ToBlob(
      base64Data,
      contentType || "audio/mpeg",
    );

    return {
      audioUrl: URL.createObjectURL(audioBlob),
      durationMs: 0,
      isCloned,
    };
  }

  private createCloneVoiceKey(voiceConfig: TTSVoiceConfig) {
    return [
      voiceConfig.provider,
      voiceConfig.baseUrl,
      voiceConfig.model ?? "",
      voiceConfig.referenceVoice ?? "",
      voiceConfig.referenceText ?? "",
      voiceConfig.apiKey ?? "",
    ].join("|");
  }

  private createCloneVoiceName(voiceConfig: TTSVoiceConfig) {
    const rawName =
      voiceConfig.referenceVoice?.split("/").pop()?.replace(/\.[^.]+$/, "") || "voice";
    const safeName =
      rawName
        .toLowerCase()
        .replace(/[^a-z0-9_-]+/g, "-")
        .replace(/^-+|-+$/g, "")
        .slice(0, 24) || "voice";

    return `anipet-${safeName}-${this.hashText(this.createCloneVoiceKey(voiceConfig)).slice(0, 10)}`;
  }

  private hashText(value: string) {
    let hash = 5381;

    for (let index = 0; index < value.length; index += 1) {
      hash = (hash * 33) ^ value.charCodeAt(index);
    }

    return (hash >>> 0).toString(16);
  }

  private async ensureCloneVoiceUri(voiceConfig: TTSVoiceConfig): Promise<string | null> {
    const cloneVoiceKey = this.createCloneVoiceKey(voiceConfig);

    if (this.cachedCloneVoice?.key === cloneVoiceKey) {
      return this.cachedCloneVoice.uri;
    }

    if (
      this.pendingCloneVoiceKey === cloneVoiceKey &&
      this.pendingCloneVoicePromise
    ) {
      return this.pendingCloneVoicePromise;
    }

    this.pendingCloneVoiceKey = cloneVoiceKey;
    this.pendingCloneVoicePromise = this.resolveCloneVoiceUri(
      voiceConfig,
      cloneVoiceKey,
    )
      .catch((error) => {
        console.warn("[TTSService] Failed to prepare cloned voice URI.", error);
        return null;
      })
      .finally(() => {
        if (this.pendingCloneVoiceKey === cloneVoiceKey) {
          this.pendingCloneVoiceKey = null;
          this.pendingCloneVoicePromise = null;
        }
      });

    return this.pendingCloneVoicePromise;
  }

  private async resolveCloneVoiceUri(
    voiceConfig: TTSVoiceConfig,
    cloneVoiceKey: string,
  ): Promise<string | null> {
    const existingUri = await this.findExistingCloneVoiceUri(voiceConfig);

    if (existingUri) {
      this.cachedCloneVoice = {
        key: cloneVoiceKey,
        uri: existingUri,
      };
      return existingUri;
    }

    const uploadedUri = await this.uploadCloneVoice(voiceConfig);

    if (uploadedUri) {
      this.cachedCloneVoice = {
        key: cloneVoiceKey,
        uri: uploadedUri,
      };
    }

    return uploadedUri;
  }

  private async findExistingCloneVoiceUri(
    voiceConfig: TTSVoiceConfig,
    excludedUri?: string | null,
  ): Promise<string | null> {
    const apiKey = voiceConfig.apiKey;

    if (!apiKey) {
      return null;
    }

    const { baseUrl, model } = this.resolveProviderConfig(
      "siliconflow",
      voiceConfig.baseUrl,
      voiceConfig.model,
    );
    const cloneVoiceName = this.createCloneVoiceName(voiceConfig);

    try {
      const payload = await this.requestJson<RemoteVoiceListResponse>(
        `${baseUrl}/audio/voice/list`,
        {
          method: "GET",
          headers: {
            Authorization: `Bearer ${apiKey}`,
          },
        },
        VOICE_LIST_REQUEST_POLICY,
      );
      const matchedVoice = payload.results?.find((candidate) => {
        const candidateUri = candidate.uri?.trim() || null;

        return (
          candidate.customName === cloneVoiceName &&
          candidate.model === model &&
          candidateUri !== excludedUri
        );
      });

      return matchedVoice?.uri?.trim() || null;
    } catch (error) {
      console.warn(
        "[TTSService] Failed to query uploaded voices.",
        this.formatRequestError(error),
      );
      return null;
    }
  }

  private async uploadCloneVoice(
    voiceConfig: TTSVoiceConfig,
  ): Promise<string | null> {
    const apiKey = voiceConfig.apiKey;
    const referenceVoice = voiceConfig.referenceVoice;

    if (!apiKey || !referenceVoice) {
      return null;
    }

    // Graceful degradation when no VoiceFileReader is configured
    if (!this.voiceFileReader) {
      console.warn(
        "[TTSService] No VoiceFileReader configured. Skipping voice clone upload.",
      );
      return null;
    }

    const voiceFileData = await this.voiceFileReader.readVoiceFile(referenceVoice);
    if (!voiceFileData) {
      console.warn(
        `[TTSService] Voice file "${referenceVoice}" could not be read for upload.`,
      );
      return null;
    }

    const { baseUrl, model } = this.resolveProviderConfig(
      "siliconflow",
      voiceConfig.baseUrl,
      voiceConfig.model,
    );
    const formData = new FormData();

    formData.append("customName", this.createCloneVoiceName(voiceConfig));
    formData.append("model", model);
    formData.append("text", voiceConfig.referenceText || "");
    formData.append(
      "file",
      this.decodeBase64ToBlob(voiceFileData.base64Data, voiceFileData.contentType),
      voiceFileData.fileName,
    );

    const payload = await this.requestJson<RemoteVoiceUploadResponse>(
      `${baseUrl}/uploads/audio/voice`,
      {
        method: "POST",
        headers: {
          Authorization: `Bearer ${apiKey}`,
        },
        body: formData,
      },
      VOICE_UPLOAD_REQUEST_POLICY,
    );
    return payload.uri?.trim() || null;
  }

  private decodeBase64ToBlob(base64Data: string, contentType: string) {
    const binary = atob(base64Data);
    const bytes = new Uint8Array(binary.length);

    for (let index = 0; index < binary.length; index += 1) {
      bytes[index] = binary.charCodeAt(index);
    }

    return new Blob([bytes], {
      type: contentType,
    });
  }
}

export const ttsService = new TTSService();
