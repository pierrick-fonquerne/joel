import { TestBed, fakeAsync, flushMicrotasks } from '@angular/core/testing';
import { provideRouter } from '@angular/router';
import { signal } from '@angular/core';
import { AccountsComponent } from './accounts.component';
import { WealthService } from './wealth.service';
import { Account } from './wealth.models';

const accounts: Account[] = [
  { id: 'a1', name: 'PEA Bourso', kind: 'brokerage_pea', owner: 'personal', currency: 'EUR', is_archived: false, notes: null, latest_valuation: null, is_stale: true },
  { id: 'a2', name: 'Trésorerie', kind: 'bank_account', owner: 'company', currency: 'EUR', is_archived: false, notes: null,
    latest_valuation: { id: 'v', as_of: '2026-10-01', amount: '40000', currency: 'EUR', source: 'manual', recorded_at: '' }, is_stale: false },
];

describe('AccountsComponent', () => {
  let wealth: jasmine.SpyObj<WealthService>;

  beforeEach(() => {
    wealth = Object.assign(jasmine.createSpyObj<WealthService>('WealthService', ['listAccounts', 'createAccount', 'archiveAccount']), {
      vaultSealed: signal(false),
    });
    wealth.listAccounts.and.resolveTo(accounts);
    wealth.createAccount.and.resolveTo(accounts[0]);
    wealth.archiveAccount.and.resolveTo({});
    TestBed.configureTestingModule({ providers: [provideRouter([]), { provide: WealthService, useValue: wealth }] });
    localStorage.removeItem('joel.wealth.discreet');
  });

  afterEach(() => {
    localStorage.removeItem('joel.wealth.discreet');
  });

  it('groups accounts by owner and flags stale ones', fakeAsync(() => {
    const fixture = TestBed.createComponent(AccountsComponent);
    fixture.detectChanges();
    flushMicrotasks();
    fixture.detectChanges();
    const text = (fixture.nativeElement as HTMLElement).textContent!;
    expect(text).toContain('Perso');
    expect(text).toContain('SASU');
    expect(text).toContain('À mettre à jour');
  }));

  it('masks the latest valuation in discreet mode', fakeAsync(() => {
    localStorage.setItem('joel.wealth.discreet', 'true');
    const fixture = TestBed.createComponent(AccountsComponent);
    fixture.detectChanges();
    flushMicrotasks();
    fixture.detectChanges();
    const text = (fixture.nativeElement as HTMLElement).textContent!;
    expect(text).toContain('•••••');
    expect(text).not.toContain('40');
  }));

  it('creates an account then reloads the list', fakeAsync(() => {
    const fixture = TestBed.createComponent(AccountsComponent);
    fixture.detectChanges();
    flushMicrotasks();
    const component = fixture.componentInstance;
    component.draftName.set('Livret A');
    component.draftKind.set('savings');
    void component.create();
    flushMicrotasks();
    expect(wealth.createAccount).toHaveBeenCalledWith({ name: 'Livret A', kind: 'savings', owner: 'personal', currency: 'EUR', notes: null });
    expect(wealth.listAccounts).toHaveBeenCalledTimes(2);
  }));

  it('asks for a second tap before archiving', fakeAsync(() => {
    const fixture = TestBed.createComponent(AccountsComponent);
    fixture.detectChanges();
    flushMicrotasks();
    const component = fixture.componentInstance;
    void component.archive('a1');
    flushMicrotasks();
    expect(wealth.archiveAccount).not.toHaveBeenCalled();
    void component.archive('a1');
    flushMicrotasks();
    expect(wealth.archiveAccount).toHaveBeenCalledWith('a1');
  }));
});
