import { signal } from '@angular/core';
import { TestBed, fakeAsync, flushMicrotasks } from '@angular/core/testing';
import { provideHttpClient } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { provideRouter } from '@angular/router';
import { CockpitComponent } from './cockpit.component';
import { WealthService } from '../wealth/wealth.service';
import { DiscreetModeService } from '../wealth/discreet-mode.service';
import { NetWorth } from '../wealth/wealth.models';

const netWorth: NetWorth = {
  as_of: '2026-10-04',
  total: '150000',
  by_owner: { personal: '110000', company: '40000' },
  by_kind: { brokerage_pea: '110000', bank_account: '40000' },
  stale_account_ids: ['a1'],
};

describe('CockpitComponent wealth tile', () => {
  let wealth: jasmine.SpyObj<WealthService> & { vaultSealed: ReturnType<typeof signal<boolean>> };

  beforeEach(() => {
    wealth = Object.assign(jasmine.createSpyObj<WealthService>('WealthService', ['netWorth']), {
      vaultSealed: signal(false),
    });
    wealth.netWorth.and.resolveTo(netWorth);
    localStorage.removeItem('joel.wealth.discreet');
    TestBed.configureTestingModule({
      providers: [
        provideRouter([]),
        provideHttpClient(),
        provideHttpClientTesting(),
        { provide: WealthService, useValue: wealth },
      ],
    });
  });

  function render(): HTMLElement {
    const fixture = TestBed.createComponent(CockpitComponent);
    fixture.detectChanges();
    flushMicrotasks();
    fixture.detectChanges();
    return fixture.nativeElement as HTMLElement;
  }

  const tileText = (root: HTMLElement): string =>
    root.querySelector('.cockpit__tile')!.textContent!.replace(/\s/g, ' ');

  it('shows the total and the stale count', fakeAsync(() => {
    const text = tileText(render());
    expect(text).toContain('150 000,00 €');
    expect(text).toContain('1 à mettre à jour');
  }));

  it('shows the sealed state', fakeAsync(() => {
    wealth.vaultSealed.set(true);
    const text = tileText(render());
    expect(text).toContain('Coffre scellé');
    expect(text).not.toContain('150');
  }));

  it('masks the total in discreet mode', fakeAsync(() => {
    TestBed.inject(DiscreetModeService).toggle();
    const text = tileText(render());
    expect(text).toContain('•••••');
    expect(text).not.toContain('150');
  }));

  it('survives a failing wealth call and keeps the health line', fakeAsync(() => {
    wealth.netWorth.and.rejectWith(new Error('boom'));
    const fixture = TestBed.createComponent(CockpitComponent);
    fixture.detectChanges();
    TestBed.inject(HttpTestingController)
      .expectOne('/api/healthz')
      .flush({ version: '1.0', db: 'ok' });
    flushMicrotasks();
    fixture.detectChanges();
    const root = fixture.nativeElement as HTMLElement;
    expect(root.textContent).toContain('Joel est opérationnel');
    expect(tileText(root)).toContain('Patrimoine');
  }));
});
