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
    expect((fixture.nativeElement as HTMLElement).textContent).not.toContain('150');
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
