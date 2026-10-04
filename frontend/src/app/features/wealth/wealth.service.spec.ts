import { TestBed, fakeAsync, flushMicrotasks } from '@angular/core/testing';
import { provideHttpClient } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { WealthService } from './wealth.service';

describe('WealthService', () => {
  let service: WealthService;
  let http: HttpTestingController;

  beforeEach(() => {
    TestBed.configureTestingModule({ providers: [provideHttpClient(), provideHttpClientTesting()] });
    service = TestBed.inject(WealthService);
    http = TestBed.inject(HttpTestingController);
  });

  afterEach(() => http.verify());

  it('posts a valuation with a decimal string amount', fakeAsync(() => {
    service.recordValuation('a1', '1234.56', '2026-10-04');
    const request = http.expectOne('/api/wealth/accounts/a1/valuations');
    expect(request.request.method).toBe('POST');
    expect(request.request.body).toEqual({ amount: '1234.56', as_of: '2026-10-04' });
    request.flush({});
    flushMicrotasks();
  }));

  it('requests net worth history with its range', fakeAsync(() => {
    service.netWorthHistory('2025-10-04', '2026-10-04');
    http.expectOne('/api/wealth/net-worth/history?from=2025-10-04&to=2026-10-04').flush([]);
    flushMicrotasks();
  }));

  it('flags the sealed vault on a 503 wealth_vault_sealed and clears it on success', fakeAsync(() => {
    let rejected = false;
    service.netWorth().catch(() => { rejected = true; });
    http.expectOne('/api/wealth/net-worth').flush({ code: 'wealth_vault_sealed' }, { status: 503, statusText: 'Service Unavailable' });
    flushMicrotasks();
    expect(rejected).toBeTrue();
    expect(service.vaultSealed()).toBeTrue();

    service.listAccounts();
    http.expectOne('/api/wealth/accounts').flush([]);
    flushMicrotasks();
    expect(service.vaultSealed()).toBeFalse();
  }));
});
