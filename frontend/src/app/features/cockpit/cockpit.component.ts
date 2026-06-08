import { Component, DestroyRef, inject, signal } from '@angular/core';
import { takeUntilDestroyed } from '@angular/core/rxjs-interop';
import { HttpClient } from '@angular/common/http';
import { LoggerService } from '../../core/logging/logger.service';
import type { Health } from '../../core/api';

@Component({
  selector: 'app-cockpit',
  template: `
    <h2>Cockpit</h2>
    @if (health(); as h) {
      <p>Joel est opérationnel - api v{{ h.version }}, base {{ h.db }}.</p>
    } @else {
      <p>Connexion à l'api...</p>
    }
  `,
})
export class CockpitComponent {
  private readonly http = inject(HttpClient);
  private readonly logger = inject(LoggerService);
  private readonly destroyRef = inject(DestroyRef);
  readonly health = signal<Health | null>(null);

  constructor() {
    this.http
      .get<Health>('/api/healthz')
      .pipe(takeUntilDestroyed(this.destroyRef))
      .subscribe({
        next: (h) => this.health.set(h),
        error: () => this.logger.error('cockpit.health.unreachable', {}),
      });
  }
}
