import { inject, Injectable, signal } from '@angular/core';
import { HttpClient, HttpErrorResponse } from '@angular/common/http';
import { firstValueFrom, Observable } from 'rxjs';
import { Account, NetWorth, NewAccount, Valuation } from './wealth.models';

const VAULT_SEALED_CODE = 'wealth_vault_sealed';

@Injectable({ providedIn: 'root' })
export class WealthService {
  private readonly http = inject(HttpClient);

  /** True after the api reported the wealth vault as sealed, until the next success. */
  readonly vaultSealed = signal(false);

  listAccounts(): Promise<Account[]> {
    return this.call(this.http.get<Account[]>('/api/wealth/accounts'));
  }

  createAccount(account: NewAccount): Promise<Account> {
    return this.call(this.http.post<Account>('/api/wealth/accounts', account));
  }

  archiveAccount(id: string): Promise<unknown> {
    return this.call(this.http.post(`/api/wealth/accounts/${id}/archive`, {}));
  }

  recordValuation(accountId: string, amount: string, asOf: string): Promise<Valuation> {
    return this.call(
      this.http.post<Valuation>(`/api/wealth/accounts/${accountId}/valuations`, { amount, as_of: asOf }),
    );
  }

  accountHistory(accountId: string): Promise<Valuation[]> {
    return this.call(this.http.get<Valuation[]>(`/api/wealth/accounts/${accountId}/valuations`));
  }

  netWorth(at?: string): Promise<NetWorth> {
    const query = at ? `?at=${at}` : '';
    return this.call(this.http.get<NetWorth>(`/api/wealth/net-worth${query}`));
  }

  netWorthHistory(from: string, to?: string): Promise<NetWorth[]> {
    const query = to ? `?from=${from}&to=${to}` : `?from=${from}`;
    return this.call(this.http.get<NetWorth[]>(`/api/wealth/net-worth/history${query}`));
  }

  private async call<T>(request: Observable<T>): Promise<T> {
    try {
      const result = await firstValueFrom(request);
      this.vaultSealed.set(false);
      return result;
    } catch (error) {
      if (error instanceof HttpErrorResponse && error.status === 503 && error.error?.code === VAULT_SEALED_CODE) {
        this.vaultSealed.set(true);
      }
      throw error;
    }
  }
}
