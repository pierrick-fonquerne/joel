import { TestBed, fakeAsync, flushMicrotasks } from '@angular/core/testing';
import { provideHttpClient } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { AuthService } from './auth.service';

describe('AuthService', () => {
  let service: AuthService;
  let http: HttpTestingController;

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [provideHttpClient(), provideHttpClientTesting()],
    });
    service = TestBed.inject(AuthService);
    http = TestBed.inject(HttpTestingController);
  });

  afterEach(() => http.verify());

  it('posts credentials on password login and loads identity', fakeAsync(() => {
    let resolved = false;
    service.loginWithPassword('a@b.c', 'pwd', '123456').then(() => { resolved = true; });
    const loginReq = http.expectOne('/api/auth/login');
    expect(loginReq.request.method).toBe('POST');
    expect(loginReq.request.body).toEqual({ email: 'a@b.c', password: 'pwd', totp: '123456' });
    loginReq.flush({});
    flushMicrotasks();
    const meReq = http.expectOne('/api/auth/me');
    meReq.flush({ email: 'a@b.c', display_name: 'A' });
    flushMicrotasks();
    expect(resolved).toBeTrue();
    expect(service.identity()?.email).toBe('a@b.c');
  }));

  it('clears identity on logout', fakeAsync(() => {
    let resolved = false;
    service.logout().then(() => { resolved = true; });
    http.expectOne('/api/auth/logout').flush(null, { status: 204, statusText: 'No Content' });
    flushMicrotasks();
    expect(resolved).toBeTrue();
    expect(service.identity()).toBeNull();
  }));
});
