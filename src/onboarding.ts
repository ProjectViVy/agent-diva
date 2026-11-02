import { invoke, isTauri } from '@tauri-apps/api/core';

export type OnboardingState = { completed: boolean; forced: boolean };

export async function readOnboarding(): Promise<OnboardingState> {
  if (!isTauri()) return { completed: true, forced: false };
  const state = await invoke<OnboardingState>('onboarding_read');
  return { completed: Boolean(state.completed), forced: Boolean(state.forced) };
}

export async function completeOnboarding(): Promise<void> {
  if (!isTauri()) return;
  await invoke('onboarding_complete');
}

export async function resetOnboarding(): Promise<void> {
  if (!isTauri()) return;
  await invoke('onboarding_reset');
}

export async function openPersonaDirectory(): Promise<string> {
  if (!isTauri()) throw new Error('请在桌面应用中打开角色工作区');
  return invoke<string>('open_persona_directory');
}
