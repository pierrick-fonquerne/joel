import { inject } from '@angular/core';
import { CanActivateFn, Router } from '@angular/router';
import { AuthService } from './auth.service';

export const authGuard: CanActivateFn = async () => {
  const auth = inject(AuthService);
  const router = inject(Router);
  if (auth.identity()) {
    return true;
  }
  await auth.refreshIdentity();
  return auth.identity() ? true : router.parseUrl('/login');
};
