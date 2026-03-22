import { openUrl } from '@tauri-apps/plugin-opener';
import { isTauriRuntime } from '../api/desktop';

/** Opens a URL in the system default browser (Tauri) or a new tab (browser dev). */
export async function openExternalUrl(url: string): Promise<void> {
  if (isTauriRuntime()) {
    await openUrl(url);
    return;
  }
  window.open(url, '_blank', 'noopener,noreferrer');
}
