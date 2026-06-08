import { Component, inject, signal } from '@angular/core';
import { HttpClient } from '@angular/common/http';
import { LoggerService } from '../../core/logging/logger.service';
import type { Health } from '../../core/api';

@Component({
  selector: 'app-cockpit',
  template: `
    <h2>Cockpit</h2>
    @if (health(); as h) {
      <p>Joel est operationnel - api v{{ h.version }}, base {{ h.db }}.</p>
    } @else {
      <p>Connexion a l'api...</p>
    }
  `,
})
export class CockpitComponent {
  private readonly http = inject(HttpClient);
  private readonly logger = inject(LoggerService);
  readonly health = signal<Health | null>(null);

  constructor() {
    this.http.get<Health>('/api/healthz').subscribe({
      next: (h) => this.health.set(h),
      error: () => this.logger.error('cockpit.health.unreachable', {}),
    });
  }
}
