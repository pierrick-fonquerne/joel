import { Component, computed, inject, signal } from '@angular/core';
import { RouterLink } from '@angular/router';
import { LoggerService } from '../../core/logging/logger.service';
import { errorCodeOf, wealthErrorMessage } from './wealth-error-messages';
import { DiscreetModeService } from './discreet-mode.service';
import { NetWorthChartComponent } from './net-worth-chart.component';
import { VaultSealedComponent } from './vault-sealed.component';
import { WealthAmountPipe } from './wealth-amount.pipe';
import { WealthService } from './wealth.service';
import { ACCOUNT_KIND_LABELS, AccountKind, NetWorth, Owner, OWNER_LABELS } from './wealth.models';

const UNAVAILABLE_MESSAGE = 'Patrimoine indisponible, réessaie plus tard';

type OwnerSelection = Owner | 'total';

function isoDate(date: Date): string {
  const month = String(date.getMonth() + 1).padStart(2, '0');
  const day = String(date.getDate()).padStart(2, '0');
  return `${date.getFullYear()}-${month}-${day}`;
}

@Component({
  selector: 'app-wealth-dashboard',
  imports: [RouterLink, NetWorthChartComponent, VaultSealedComponent, WealthAmountPipe],
  template: `
    <header class="wealth__header">
      <h2>Patrimoine</h2>
      <button type="button" class="wealth__discreet" (click)="discreet.toggle()">
        {{ discreet.isDiscreet() ? 'Afficher' : 'Masquer' }}
      </button>
    </header>

    @if (wealth.vaultSealed()) {
      <app-vault-sealed />
    } @else {
      @if (current(); as netWorth) {
        <nav class="wealth__owners">
          @for (option of ownerOptions; track option.value) {
            <button type="button" [class.active]="selectedOwner() === option.value" (click)="selectedOwner.set(option.value)">
              {{ option.label }}
            </button>
          }
        </nav>

        <p class="wealth__total">{{ selectedTotal() | wealthAmount: 'EUR' : discreet.isDiscreet() }}</p>
        @if (delta(); as change) {
          <p class="wealth__delta">
            {{ change.sign }}{{ change.signedAmount | wealthAmount: 'EUR' : discreet.isDiscreet() }} {{ discreet.isDiscreet() ? '(•••)' : '(' + change.percent + ')' }}
            depuis la fin du mois dernier
          </p>
        }

        @if (historyError(); as message) {
          <p class="wealth__error" role="alert">{{ message }}</p>
        } @else {
          <app-net-worth-chart [points]="chartPoints()" />
        }

        @if (selectedOwner() === 'total') {
          <ul class="wealth__kinds">
            @for (entry of kindBreakdown(); track entry.kind) {
              <li><span>{{ entry.label }}</span><span>{{ entry.amount | wealthAmount: 'EUR' : discreet.isDiscreet() }}</span></li>
            }
          </ul>
        }

        <a routerLink="/patrimoine/comptes" class="wealth__accounts">
          @if (netWorth.stale_account_ids.length > 0) {
            {{ netWorth.stale_account_ids.length }} compte{{ netWorth.stale_account_ids.length > 1 ? 's' : '' }} à mettre à jour
          } @else {
            Voir les comptes
          }
        </a>
      } @else if (loadError()) {
        <p class="wealth__error" role="alert">{{ loadError() }}</p>
      } @else {
        <p>Chargement du patrimoine...</p>
      }
    }
  `,
  styles: `
    .wealth__header { display: flex; justify-content: space-between; align-items: center; }
    .wealth__owners { display: flex; gap: 0.5rem; margin: 0.5rem 0; }
    .wealth__owners .active { font-weight: 700; text-decoration: underline; }
    .wealth__total { font-size: 2.2rem; font-weight: 700; margin: 0.5rem 0 0; }
    .wealth__error { color: #e0a458; }
    .wealth__delta { color: #8fa3b1; margin: 0 0 1rem; }
    .wealth__kinds { list-style: none; padding: 0; }
    .wealth__kinds li { display: flex; justify-content: space-between; padding: 0.35rem 0; border-bottom: 1px solid #222a31; }
    .wealth__accounts { display: block; margin-top: 1rem; }
    button { min-height: 2.75rem; padding: 0 0.9rem; }
  `,
})
export class WealthDashboardComponent {
  protected readonly wealth = inject(WealthService);
  protected readonly discreet = inject(DiscreetModeService);
  private readonly logger = inject(LoggerService);

  readonly selectedOwner = signal<OwnerSelection>('total');
  protected readonly current = signal<NetWorth | null>(null);
  protected readonly history = signal<NetWorth[]>([]);
  protected readonly loadError = signal<string | null>(null);
  protected readonly historyError = signal<string | null>(null);
  protected readonly ownerOptions: { value: OwnerSelection; label: string }[] = [
    { value: 'total', label: 'Total' },
    { value: 'personal', label: OWNER_LABELS.personal },
    { value: 'company', label: OWNER_LABELS.company },
  ];

  protected readonly selectedTotal = computed(() => {
    const netWorth = this.current();
    return netWorth ? this.totalFor(netWorth) : null;
  });

  protected readonly delta = computed(() => {
    const points = this.history();
    if (points.length < 2) {
      return null;
    }
    const now = Number(this.totalFor(points[points.length - 1]));
    const before = Number(this.totalFor(points[points.length - 2]));
    const difference = now - before;
    const sign = difference >= 0 ? '+' : '';
    const percent = before === 0
      ? '-'
      : `${sign}${new Intl.NumberFormat('fr-FR', { maximumFractionDigits: 1 }).format((difference / Math.abs(before)) * 100)} %`;
    return { signedAmount: `${difference.toFixed(2)}`, percent, sign };
  });

  protected readonly chartPoints = computed(() =>
    this.history().map((point) => ({ date: point.as_of, value: Number(this.totalFor(point)) })),
  );

  protected readonly kindBreakdown = computed(() => {
    const netWorth = this.current();
    if (!netWorth) {
      return [];
    }
    return (Object.entries(netWorth.by_kind) as [AccountKind, string][])
      .map(([kind, amount]) => ({ kind, label: ACCOUNT_KIND_LABELS[kind], amount }))
      .sort((left, right) => Number(right.amount) - Number(left.amount));
  });

  constructor() {
    void this.load();
  }

  private totalFor(netWorth: NetWorth): string {
    const owner = this.selectedOwner();
    return owner === 'total' ? netWorth.total : (netWorth.by_owner[owner] ?? '0');
  }

  private async load(): Promise<void> {
    const today = new Date();
    const oneYearAgo = new Date(today.getFullYear() - 1, today.getMonth(), today.getDate());
    await Promise.all([this.loadCurrent(), this.loadHistory(isoDate(oneYearAgo), isoDate(today))]);
  }

  private async loadCurrent(): Promise<void> {
    try {
      this.current.set(await this.wealth.netWorth());
    } catch (error) {
      this.loadError.set(wealthErrorMessage(errorCodeOf(error), UNAVAILABLE_MESSAGE));
      this.logger.error('wealth.dashboard.load_failed', { isVaultSealed: this.wealth.vaultSealed() });
    }
  }

  private async loadHistory(from: string, to: string): Promise<void> {
    try {
      this.history.set(await this.wealth.netWorthHistory(from, to));
    } catch (error) {
      this.historyError.set(wealthErrorMessage(errorCodeOf(error), UNAVAILABLE_MESSAGE));
      this.logger.error('wealth.dashboard.history_failed', { isVaultSealed: this.wealth.vaultSealed() });
    }
  }
}
