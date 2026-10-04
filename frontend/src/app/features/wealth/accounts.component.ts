import { Component, computed, inject, signal } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { RouterLink } from '@angular/router';
import { LoggerService } from '../../core/logging/logger.service';
import { DiscreetModeService } from './discreet-mode.service';
import { VaultSealedComponent } from './vault-sealed.component';
import { WealthAmountPipe } from './wealth-amount.pipe';
import { errorCodeOf, wealthErrorMessage } from './wealth-error-messages';
import { WealthService } from './wealth.service';
import { Account, ACCOUNT_KIND_LABELS, ACCOUNT_KINDS, AccountKind, Owner, OWNER_LABELS } from './wealth.models';

const OWNERS: readonly Owner[] = ['personal', 'company'];

@Component({
  selector: 'app-accounts',
  imports: [FormsModule, RouterLink, VaultSealedComponent, WealthAmountPipe],
  template: `
    <h2>Comptes</h2>
    @if (wealth.vaultSealed()) {
      <app-vault-sealed />
    } @else {
      @if (errorMessage(); as message) {
        <p role="alert" class="form__error">{{ message }}</p>
      }
      @for (group of groups(); track group.owner) {
        <h3>{{ group.label }}</h3>
        <ul class="accounts">
          @for (account of group.accounts; track account.id) {
            <li class="accounts__item">
              <a [routerLink]="['/patrimoine/comptes', account.id, 'releve']">
                <strong>{{ account.name }}</strong>
                <span>{{ kindLabels[account.kind] }}</span>
                <span>{{ account.latest_valuation?.amount ?? null | wealthAmount: account.currency : discreet.isDiscreet() }}</span>
                @if (account.is_stale) {
                  <span class="accounts__stale">À mettre à jour</span>
                }
              </a>
              <button type="button" (click)="archive(account.id)">
                {{ pendingArchiveId() === account.id ? 'Confirmer' : 'Archiver' }}
              </button>
            </li>
          }
        </ul>
      }

      <h3>Nouveau compte</h3>
      <form (ngSubmit)="create()" class="form">
        <input name="name" placeholder="Nom" [ngModel]="draftName()" (ngModelChange)="draftName.set($event)" required />
        <select name="kind" [ngModel]="draftKind()" (ngModelChange)="draftKind.set($event)">
          @for (kind of kinds; track kind) {
            <option [value]="kind">{{ kindLabels[kind] }}</option>
          }
        </select>
        <select name="owner" [ngModel]="draftOwner()" (ngModelChange)="draftOwner.set($event)">
          @for (owner of owners; track owner) {
            <option [value]="owner">{{ ownerLabels[owner] }}</option>
          }
        </select>
        <input name="currency" maxlength="3" [ngModel]="draftCurrency()" (ngModelChange)="draftCurrency.set($event.toUpperCase())" />
        <input name="notes" placeholder="Notes" [ngModel]="draftNotes()" (ngModelChange)="draftNotes.set($event)" />
        <button type="submit" [disabled]="isCreating()">Créer</button>
      </form>
    }
  `,
  styles: `
    .accounts { list-style: none; padding: 0; }
    .accounts__item { display: flex; justify-content: space-between; align-items: center; gap: 0.5rem; padding: 0.5rem 0; border-bottom: 1px solid #222a31; }
    .accounts__item a { display: grid; gap: 0.15rem; color: inherit; text-decoration: none; }
    .accounts__stale { color: #f3c26b; font-size: 0.85rem; }
    .form { display: flex; flex-direction: column; gap: 0.6rem; max-width: 24rem; }
    .form input, .form select, button { min-height: 2.75rem; }
    .form__error { color: #f08a7e; }
  `,
})
export class AccountsComponent {
  protected readonly wealth = inject(WealthService);
  protected readonly discreet = inject(DiscreetModeService);
  private readonly logger = inject(LoggerService);

  protected readonly kinds = ACCOUNT_KINDS;
  protected readonly kindLabels = ACCOUNT_KIND_LABELS;
  protected readonly owners = OWNERS;
  protected readonly ownerLabels = OWNER_LABELS;

  protected readonly accounts = signal<Account[]>([]);
  protected readonly pendingArchiveId = signal<string | null>(null);
  protected readonly errorMessage = signal<string | null>(null);
  protected readonly isCreating = signal(false);
  readonly draftName = signal('');
  readonly draftKind = signal<AccountKind>('bank_account');
  readonly draftOwner = signal<Owner>('personal');
  readonly draftCurrency = signal('EUR');
  readonly draftNotes = signal('');

  protected readonly groups = computed(() =>
    OWNERS.map((owner) => ({
      owner,
      label: OWNER_LABELS[owner],
      accounts: this.accounts().filter((account) => account.owner === owner && !account.is_archived),
    })).filter((group) => group.accounts.length > 0),
  );

  constructor() {
    void this.reload();
  }

  async create(): Promise<void> {
    if (this.isCreating()) {
      return;
    }
    this.isCreating.set(true);
    this.errorMessage.set(null);
    try {
      await this.wealth.createAccount({
        name: this.draftName().trim(),
        kind: this.draftKind(),
        owner: this.draftOwner(),
        currency: this.draftCurrency(),
        notes: this.draftNotes().trim() === '' ? null : this.draftNotes(),
      });
      this.draftName.set('');
      this.draftNotes.set('');
      await this.reload();
    } catch (error) {
      this.errorMessage.set(wealthErrorMessage(errorCodeOf(error), 'Création impossible, réessaie'));
    } finally {
      this.isCreating.set(false);
    }
  }

  async archive(id: string): Promise<void> {
    if (this.pendingArchiveId() !== id) {
      this.pendingArchiveId.set(id);
      return;
    }
    this.pendingArchiveId.set(null);
    this.errorMessage.set(null);
    try {
      await this.wealth.archiveAccount(id);
      await this.reload();
    } catch (error) {
      this.errorMessage.set(wealthErrorMessage(errorCodeOf(error), 'Archivage impossible, réessaie'));
      this.logger.error('wealth.account.archive_failed', {});
    }
  }

  private async reload(): Promise<void> {
    try {
      this.accounts.set(await this.wealth.listAccounts());
    } catch {
      this.logger.error('wealth.accounts.load_failed', { isVaultSealed: this.wealth.vaultSealed() });
    }
  }
}
