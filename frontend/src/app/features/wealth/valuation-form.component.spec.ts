import { TestBed, fakeAsync, flushMicrotasks } from '@angular/core/testing';
import { ActivatedRoute, Router, convertToParamMap, provideRouter } from '@angular/router';
import { signal } from '@angular/core';
import { HttpErrorResponse } from '@angular/common/http';
import { ValuationFormComponent } from './valuation-form.component';
import { WealthService } from './wealth.service';

describe('ValuationFormComponent', () => {
  let wealth: jasmine.SpyObj<WealthService>;

  beforeEach(() => {
    localStorage.removeItem('joel.wealth.discreet');
    wealth = Object.assign(jasmine.createSpyObj<WealthService>('WealthService', ['recordValuation', 'accountHistory']), {
      vaultSealed: signal(false),
    });
    wealth.accountHistory.and.resolveTo([]);
    wealth.recordValuation.and.resolveTo({ id: 'v1', as_of: '2026-10-04', amount: '1234.56', currency: 'EUR', source: 'manual', recorded_at: '' });
    TestBed.configureTestingModule({
      providers: [
        provideRouter([]),
        { provide: WealthService, useValue: wealth },
        { provide: ActivatedRoute, useValue: { snapshot: { paramMap: convertToParamMap({ id: 'a1' }) } } },
      ],
    });
  });

  afterEach(() => {
    localStorage.removeItem('joel.wealth.discreet');
  });

  it('normalizes a French amount, posts it and goes back to the accounts', fakeAsync(() => {
    const router = TestBed.inject(Router);
    spyOn(router, 'navigateByUrl').and.resolveTo(true);
    const fixture = TestBed.createComponent(ValuationFormComponent);
    fixture.detectChanges();
    fixture.componentInstance.amount.set('1 234,56');
    fixture.componentInstance.asOf.set('2026-10-04');
    void fixture.componentInstance.submit();
    flushMicrotasks();
    expect(wealth.recordValuation).toHaveBeenCalledWith('a1', '1234.56', '2026-10-04');
    expect(router.navigateByUrl).toHaveBeenCalledWith('/patrimoine/comptes');
  }));

  it('refuses an invalid amount without calling the api', fakeAsync(() => {
    const fixture = TestBed.createComponent(ValuationFormComponent);
    fixture.detectChanges();
    fixture.componentInstance.amount.set('12,345');
    void fixture.componentInstance.submit();
    flushMicrotasks();
    fixture.detectChanges();
    expect(wealth.recordValuation).not.toHaveBeenCalled();
    expect((fixture.nativeElement as HTMLElement).textContent).toContain('Montant invalide');
  }));

  it('prefills the date with today', () => {
    const fixture = TestBed.createComponent(ValuationFormComponent);
    const now = new Date();
    const local = new Date(now.getTime() - now.getTimezoneOffset() * 60_000).toISOString().slice(0, 10);
    expect(fixture.componentInstance.asOf()).toBe(local);
  });

  it('masks history amounts in discreet mode', fakeAsync(() => {
    localStorage.setItem('joel.wealth.discreet', 'true');
    wealth.accountHistory.and.resolveTo([
      { id: 'v1', as_of: '2026-10-01', amount: '40000', currency: 'EUR', source: 'manual', recorded_at: '' },
    ]);
    const fixture = TestBed.createComponent(ValuationFormComponent);
    fixture.detectChanges();
    flushMicrotasks();
    fixture.detectChanges();
    const text = (fixture.nativeElement as HTMLElement).textContent!;
    expect(text).toContain('•••••');
    expect(text).not.toContain('40');
  }));

  it('toggles the sign then posts a negative amount', fakeAsync(() => {
    spyOn(TestBed.inject(Router), 'navigateByUrl').and.resolveTo(true);
    const fixture = TestBed.createComponent(ValuationFormComponent);
    fixture.detectChanges();
    fixture.componentInstance.amount.set('180 000');
    fixture.componentInstance.toggleSign();
    expect(fixture.componentInstance.amount()).toBe('-180 000');
    void fixture.componentInstance.submit();
    flushMicrotasks();
    expect(wealth.recordValuation).toHaveBeenCalledWith('a1', '-180000', jasmine.any(String));
    fixture.componentInstance.toggleSign();
    expect(fixture.componentInstance.amount()).toBe('180 000');
  }));

  it('shows the mapped message for an api error code', fakeAsync(() => {
    wealth.recordValuation.and.rejectWith(new HttpErrorResponse({ status: 400, error: { code: 'positive_loan_valuation' } }));
    const fixture = TestBed.createComponent(ValuationFormComponent);
    fixture.detectChanges();
    fixture.componentInstance.amount.set('100');
    void fixture.componentInstance.submit();
    flushMicrotasks();
    fixture.detectChanges();
    expect((fixture.nativeElement as HTMLElement).textContent).toContain('Un prêt se saisit en négatif');
  }));
});
