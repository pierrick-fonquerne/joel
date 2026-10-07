import { Component, DestroyRef, inject, signal } from '@angular/core';
import { takeUntilDestroyed } from '@angular/core/rxjs-interop';
import { HttpClient } from '@angular/common/http';
import { RouterLink } from '@angular/router';
import { LoggerService } from '../../core/logging/logger.service';
import type { Health } from '../../core/api';
import { DiscreetModeService } from '../wealth/discreet-mode.service';
import { WealthAmountPipe } from '../wealth/wealth-amount.pipe';
import { WealthService } from '../wealth/wealth.service';
import { NetWorth } from '../wealth/wealth.models';

@Component({
  selector: 'app-cockpit',
  imports: [RouterLink, WealthAmountPipe],
  template: `
    <h2>Cockpit</h2>
    @if (health(); as h) {
      <p>Joel est opérationnel - api v{{ h.version }}, base {{ h.db }}.</p>
    } @else {
      <p>Connexion à l'api...</p>
    }
    <a routerLink="/patrimoine" class="cockpit__tile">
      <h3>Patrimoine</h3>
      @if (wealth.vaultSealed()) {
        <p>Coffre scellé</p>
      } @else {
        @if (netWorth(); as value) {
          <p>{{ value.total | wealthAmount: 'EUR' : discreet.isDiscreet() }}</p>
          @if (value.stale_account_ids.length > 0) {
            <p>{{ value.stale_account_ids.length }} à mettre à jour</p>
          }
        }
      }
    </a>
  `,
})
export class CockpitComponent {
  private readonly http = inject(HttpClient);
  private readonly logger = inject(LoggerService);
  private readonly destroyRef = inject(DestroyRef);
  protected readonly wealth = inject(WealthService);
  protected readonly discreet = inject(DiscreetModeService);
  readonly health = signal<Health | null>(null);
  readonly netWorth = signal<NetWorth | null>(null);

  constructor() {
    this.http
      .get<Health>('/api/healthz')
      .pipe(takeUntilDestroyed(this.destroyRef))
      .subscribe({
        next: (h) => this.health.set(h),
        error: () => this.logger.error('cockpit.health.unreachable', {}),
      });
    this.wealth
      .netWorth()
      .then((value) => this.netWorth.set(value))
      .catch(() => this.logger.error('cockpit.wealth.unavailable', {}));
  }
}
