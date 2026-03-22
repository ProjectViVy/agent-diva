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
  configured: boolean;
  ready: boolean;
  default_api_base: string;
  default_model?: string | null;
  models: string[];
  custom_models: string[];
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
