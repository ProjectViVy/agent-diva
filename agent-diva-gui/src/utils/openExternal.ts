import { Browser } from '@wailsio/runtime';
import { isTauriRuntime } from '../api/desktop';

/** Opens a URL in the system default browser (Wails) or a new tab (browser dev). */
export async function openExternalUrl(url: string): Promise<void> {
  if (isTauriRuntime()) {
    await Browser.OpenURL(url);
    return;
  }
  window.open(url, '_blank', 'noopener,noreferrer');
}
