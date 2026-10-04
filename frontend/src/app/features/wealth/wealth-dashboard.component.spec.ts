import { HttpErrorResponse } from '@angular/common/http';
import { TestBed, fakeAsync, flushMicrotasks } from '@angular/core/testing';
import { provideRouter } from '@angular/router';
import { WealthDashboardComponent } from './wealth-dashboard.component';
import { WealthService } from './wealth.service';
import { NetWorth } from './wealth.models';
import { signal } from '@angular/core';

function netWorth(total: string, personal: string, company: string, asOf: string): NetWorth {
  return {
    as_of: asOf,
    total,
    by_owner: { personal, company },
    by_kind: { brokerage_pea: personal, bank_account: company },
    stale_account_ids: ['a1'],
  };
}

describe('WealthDashboardComponent', () => {
  const current = netWorth('150000', '110000', '40000', '2026-10-04');
  const previous = netWorth('140000', '100000', '40000', '2026-09-30');
  let wealth: jasmine.SpyObj<WealthService> & { vaultSealed: ReturnType<typeof signal<boolean>> };

  beforeEach(() => {
    wealth = Object.assign(jasmine.createSpyObj<WealthService>('WealthService', ['netWorth', 'netWorthHistory']), {
      vaultSealed: signal(false),
    });
    wealth.netWorth.and.resolveTo(current);
    wealth.netWorthHistory.and.resolveTo([previous, current]);
    TestBed.configureTestingModule({
      providers: [provideRouter([]), { provide: WealthService, useValue: wealth }],
    });
    localStorage.removeItem('joel.wealth.discreet');
  });

  it('shows the total, the delta since last month end and the stale count', fakeAsync(() => {
    const fixture = TestBed.createComponent(WealthDashboardComponent);
    fixture.detectChanges();
    flushMicrotasks();
    fixture.detectChanges();
    const text = (fixture.nativeElement as HTMLElement).textContent!.replace(/\s/g, ' ');
    expect(text).toContain('150 000,00 €');
    expect(text).toContain('+10 000,00 €');
    expect(text).toContain('+7,1 %');
    expect(text).toContain('1 compte à mettre à jour');
  }));

  it('switches to the company total', fakeAsync(() => {
    const fixture = TestBed.createComponent(WealthDashboardComponent);
    fixture.detectChanges();
    flushMicrotasks();
    fixture.componentInstance.selectedOwner.set('company');
    fixture.detectChanges();
    expect((fixture.nativeElement as HTMLElement).querySelector('.wealth__total')!.textContent!.replace(/\s/g, ' ')).toContain('40 000,00 €');
  }));

  it('masks every amount in discreet mode', fakeAsync(() => {
    const fixture = TestBed.createComponent(WealthDashboardComponent);
    fixture.detectChanges();
    flushMicrotasks();
    (fixture.nativeElement as HTMLElement).querySelector<HTMLButtonElement>('.wealth__discreet')!.click();
    fixture.detectChanges();
    const text = (fixture.nativeElement as HTMLElement).textContent!.replace(/\s/g, ' ');
    expect(text).toContain('•••••');
    expect(text).toContain('(•••)');
    expect(text).not.toContain('+');
    expect(text).not.toContain('-');
    expect((fixture.nativeElement as HTMLElement).querySelector('app-net-worth-chart')).toBeNull();
    for (const leaked of ['150', '10 000', '110 000', '40 000', '140', '7,1']) {
      expect(text).not.toContain(leaked);
    }
  }));

  it('shows the mapped message when the net worth call fails with a known code', fakeAsync(() => {
    wealth.netWorth.and.rejectWith(new HttpErrorResponse({ status: 422, error: { code: 'exchange_rate_missing' } }));
    const fixture = TestBed.createComponent(WealthDashboardComponent);
    fixture.detectChanges();
    flushMicrotasks();
    fixture.detectChanges();
    const text = (fixture.nativeElement as HTMLElement).textContent!;
    expect(text).toContain('Taux BCE introuvable pour un relevé en devise');
    expect(text).not.toContain('Chargement');
  }));

  it('shows the fallback message when the failure has no known code', fakeAsync(() => {
    wealth.netWorth.and.rejectWith(new HttpErrorResponse({ status: 500, error: null }));
    const fixture = TestBed.createComponent(WealthDashboardComponent);
    fixture.detectChanges();
    flushMicrotasks();
    fixture.detectChanges();
    expect((fixture.nativeElement as HTMLElement).textContent).toContain('Patrimoine indisponible, réessaie plus tard');
  }));

  it('keeps today total when only the history fails', fakeAsync(() => {
    wealth.netWorthHistory.and.rejectWith(new HttpErrorResponse({ status: 400, error: { code: 'invalid_range' } }));
    const fixture = TestBed.createComponent(WealthDashboardComponent);
    fixture.detectChanges();
    flushMicrotasks();
    fixture.detectChanges();
    const element = fixture.nativeElement as HTMLElement;
    expect(element.textContent!.replace(/\s/g, ' ')).toContain('150 000,00 €');
    expect(element.textContent).toContain('Période invalide');
    expect(element.querySelector('app-net-worth-chart')).toBeNull();
  }));

  it('explains the sealed vault instead of an error', fakeAsync(() => {
    wealth.netWorth.and.rejectWith(new Error('sealed'));
    wealth.netWorthHistory.and.rejectWith(new Error('sealed'));
    wealth.vaultSealed.set(true);
    const fixture = TestBed.createComponent(WealthDashboardComponent);
    fixture.detectChanges();
    flushMicrotasks();
    fixture.detectChanges();
    expect((fixture.nativeElement as HTMLElement).textContent).toContain('Le coffre patrimoine est scellé');
  }));
});
