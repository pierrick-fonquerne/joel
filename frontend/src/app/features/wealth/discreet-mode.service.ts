import { Injectable, signal } from '@angular/core';

export const DISCREET_MODE_STORAGE_KEY = 'joel.wealth.discreet';

function readStoredMode(): boolean {
  try {
    return localStorage.getItem(DISCREET_MODE_STORAGE_KEY) === 'true';
  } catch {
    return false;
  }
}

@Injectable({ providedIn: 'root' })
export class DiscreetModeService {
  /** When true, every wealth amount is masked on screen. */
  readonly isDiscreet = signal(readStoredMode());

  toggle(): void {
    const next = !this.isDiscreet();
    this.isDiscreet.set(next);
    try {
      localStorage.setItem(DISCREET_MODE_STORAGE_KEY, String(next));
    } catch {
      return;
    }
  }
}
