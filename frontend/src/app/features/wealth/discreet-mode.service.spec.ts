import { TestBed } from '@angular/core/testing';
import { DiscreetModeService, DISCREET_MODE_STORAGE_KEY } from './discreet-mode.service';

describe('DiscreetModeService', () => {
  beforeEach(() => localStorage.removeItem(DISCREET_MODE_STORAGE_KEY));

  it('toggles and persists the mode', () => {
    const service = TestBed.inject(DiscreetModeService);
    expect(service.isDiscreet()).toBeFalse();
    service.toggle();
    expect(service.isDiscreet()).toBeTrue();
    expect(localStorage.getItem(DISCREET_MODE_STORAGE_KEY)).toBe('true');
  });

  it('restores the persisted mode', () => {
    localStorage.setItem(DISCREET_MODE_STORAGE_KEY, 'true');
    expect(TestBed.inject(DiscreetModeService).isDiscreet()).toBeTrue();
  });
});
