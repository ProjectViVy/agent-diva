import { invoke } from "@tauri-apps/api/core";

export interface ProviderModelCatalog {
  provider: string;
  source: "runtime" | "static_fallback" | "unsupported" | "error" | string;
  runtime_supported: boolean;
  api_base?: string | null;
  models: string[];
  custom_models: string[];
  warnings: string[];
  error?: string | null;
}

export const getProviders = () =>
  invoke<ProviderSpecDto[]>("get_providers");

export interface ProviderModelTestResult {
  ok: boolean;
  message: string;
  latency_ms: number;
}

export interface ProviderSpecDto {
  name: string;
  display_name: string;
  api_type: string;
  source: string;
  auth_mode: string;
  login_supported: boolean;
  credential_store: string;
  runtime_backend: string;
  configured: boolean;
  ready: boolean;
  authenticated: boolean;
  active_profile?: string | null;
  expires_at?: string | null;
  profiles: ProviderAuthProfileDto[];
  default_api_base: string;
  default_model?: string | null;
  models: string[];
  custom_models: string[];
}

export interface ProviderAuthProfileDto {
  id: string;
  profile_name: string;
  account_id?: string | null;
  is_active: boolean;
}

export interface ProviderAuthStatusDto {
  provider: string;
  active_profile?: string | null;
  authenticated: boolean;
  expires_at?: string | null;
  profiles: ProviderAuthProfileDto[];
}

export type ProviderLoginMode = "browser" | "paste_redirect" | "device_code";

export interface ProviderLoginResponseDto {
  status: string;
  authorize_url?: string | null;
  result?: {
    provider: string;
    profile_name: string;
    account_id?: string | null;
    status: string;
  } | null;
  message?: string | null;
}

export interface PendingProviderLoginStatusDto {
  provider: string;
  profile: string;
  status: string;
  authorize_url?: string | null;
  message?: string | null;
  result?: ProviderLoginResponseDto["result"];
}

export interface CustomProviderPayload {
  id: string;
  displayName: string;
  apiKey: string;
  apiBase?: string | null;
  defaultModel?: string | null;
  models: string[];
}

export const getProviderModels = (
  provider: string,
  apiBase?: string | null,
  apiKey?: string | null
) =>
  invoke<ProviderModelCatalog>("get_provider_models", {
    provider,
    apiBase: apiBase ?? null,
    apiKey: apiKey ?? null,
  });

export const testProviderModel = (
  provider: string,
  model: string,
  apiBase?: string | null,
  apiKey?: string | null
) =>
  invoke<ProviderModelTestResult>("test_provider_model", {
    provider,
    model,
    apiBase: apiBase ?? null,
    apiKey: apiKey ?? null,
  });

export const addProviderModel = (provider: string, model: string) =>
  invoke<ProviderModelCatalog>("add_provider_model", {
    provider,
    model,
  });

export const removeProviderModel = (provider: string, model: string) =>
  invoke<ProviderModelCatalog>("remove_provider_model", {
    provider,
    model,
  });

export const createCustomProvider = (payload: CustomProviderPayload) =>
  invoke<ProviderSpecDto>("create_custom_provider", {
    payload: {
      id: payload.id,
      displayName: payload.displayName,
      apiType: "openai",
      apiKey: payload.apiKey,
      apiBase: payload.apiBase ?? null,
      defaultModel: payload.defaultModel ?? null,
      models: payload.models,
      extraHeaders: null,
    },
  });

export const deleteCustomProvider = (provider: string) =>
  invoke<void>("delete_custom_provider", { provider });

export const getProviderAuthStatus = (provider: string) =>
  invoke<ProviderAuthStatusDto>("get_provider_auth_status", { provider });

export const listProviderProfiles = (provider: string) =>
  invoke<ProviderAuthProfileDto[]>("list_provider_profiles", { provider });

export const loginProvider = (
  provider: string,
  profile: string,
  mode: ProviderLoginMode,
  redirectUrl?: string | null,
) =>
  invoke<ProviderLoginResponseDto>("login_provider", {
    provider,
    profile,
    mode,
    redirectUrl: redirectUrl ?? null,
  });

export const getProviderLoginStatus = (provider: string, profile: string) =>
  invoke<PendingProviderLoginStatusDto>("get_provider_login_status", {
    provider,
    profile,
  });

export const useProviderProfile = (provider: string, profile: string) =>
  invoke<ProviderAuthStatusDto>("use_provider_profile", { provider, profile });

export const refreshProviderAuth = (provider: string, profile?: string | null) =>
  invoke<ProviderAuthStatusDto>("refresh_provider_auth", {
    provider,
    profile: profile ?? null,
  });

export const logoutProvider = (provider: string, profile?: string | null) =>
  invoke<ProviderAuthStatusDto>("logout_provider", {
    provider,
    profile: profile ?? null,
  });
