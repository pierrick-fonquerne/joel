import { Component, inject, signal } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { ActivatedRoute, Router, RouterLink } from '@angular/router';
import { LoggerService } from '../../core/logging/logger.service';
import { normalizeAmount } from './amount-input';
import { DiscreetModeService } from './discreet-mode.service';
import { VaultSealedComponent } from './vault-sealed.component';
import { WealthAmountPipe } from './wealth-amount.pipe';
import { errorCodeOf, wealthErrorMessage } from './wealth-error-messages';
import { WealthService } from './wealth.service';
import { Valuation } from './wealth.models';

function localToday(): string {
  const now = new Date();
  return new Date(now.getTime() - now.getTimezoneOffset() * 60_000).toISOString().slice(0, 10);
}

@Component({
  selector: 'app-valuation-form',
  imports: [FormsModule, RouterLink, VaultSealedComponent, WealthAmountPipe],
  template: `
    <h2>Nouveau relevé</h2>
    @if (wealth.vaultSealed()) {
      <app-vault-sealed />
    } @else {
      <form (ngSubmit)="submit()" class="form">
        <label>
          Montant
          <span class="form__amount">
            <input name="amount" inputmode="decimal" autocomplete="off" [ngModel]="amount()" (ngModelChange)="amount.set($event)" required />
            <button type="button" class="form__sign" aria-label="Inverser le signe" (click)="toggleSign()">±</button>
          </span>
        </label>
        <label>
          Date
          <input name="asOf" type="date" [ngModel]="asOf()" (ngModelChange)="asOf.set($event)" required />
        </label>
        @if (errorMessage(); as message) {
          <p class="form__error" role="alert">{{ message }}</p>
        }
        <button type="submit" [disabled]="isSaving()">Enregistrer</button>
        <a routerLink="/patrimoine/comptes">Annuler</a>
      </form>

      @if (history().length > 0) {
        <h3>Derniers relevés</h3>
        <ul class="history">
          @for (valuation of history(); track valuation.id) {
            <li><span>{{ valuation.as_of }}</span><span>{{ valuation.amount | wealthAmount: valuation.currency : discreet.isDiscreet() }}</span></li>
          }
        </ul>
      }
    }
  `,
  styles: `
    .form { display: flex; flex-direction: column; gap: 0.9rem; max-width: 24rem; }
    .form input { font-size: 1.3rem; min-height: 2.9rem; width: 100%; }
    .form button { min-height: 3rem; font-size: 1.1rem; }
    .form__amount { display: flex; gap: 0.5rem; }
    .form__sign { min-width: 3rem; min-height: 2.9rem; font-size: 1.3rem; }
    .form__error { color: #f08a7e; }
    .history { list-style: none; padding: 0; }
    .history li { display: flex; justify-content: space-between; padding: 0.3rem 0; }
  `,
})
export class ValuationFormComponent {
  protected readonly wealth = inject(WealthService);
  protected readonly discreet = inject(DiscreetModeService);
  private readonly router = inject(Router);
  private readonly logger = inject(LoggerService);
  private readonly accountId = inject(ActivatedRoute).snapshot.paramMap.get('id') ?? '';

  readonly amount = signal('');
  readonly asOf = signal(localToday());
  protected readonly errorMessage = signal<string | null>(null);
  protected readonly isSaving = signal(false);
  protected readonly history = signal<Valuation[]>([]);

  constructor() {
    this.wealth
      .accountHistory(this.accountId)
      .then((valuations) => this.history.set(valuations.slice(-5).reverse()))
      .catch(() => this.logger.error('wealth.valuation.history_failed', {}));
  }

  toggleSign(): void {
    const current = this.amount().trim();
    this.amount.set(current.startsWith('-') ? current.slice(1).trimStart() : `-${current}`);
  }

  async submit(): Promise<void> {
    const amount = normalizeAmount(this.amount());
    if (amount === null) {
      this.errorMessage.set(wealthErrorMessage('invalid_amount', 'Montant invalide'));
      return;
    }
    this.isSaving.set(true);
    this.errorMessage.set(null);
    try {
      await this.wealth.recordValuation(this.accountId, amount, this.asOf());
      this.logger.info('wealth.valuation.recorded', {});
      await this.router.navigateByUrl('/patrimoine/comptes');
    } catch (error) {
      this.errorMessage.set(wealthErrorMessage(errorCodeOf(error), 'Enregistrement impossible, réessaie'));
    } finally {
      this.isSaving.set(false);
    }
  }
}
