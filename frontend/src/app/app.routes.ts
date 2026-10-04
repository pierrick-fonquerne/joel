import { Routes } from '@angular/router';
import { ShellComponent } from './core/layout/shell.component';
import { authGuard } from './core/auth/auth.guard';

export const routes: Routes = [
  {
    path: 'login',
    loadComponent: () => import('./features/login/login.component').then((m) => m.LoginComponent),
  },
  {
    path: '',
    component: ShellComponent,
    canActivate: [authGuard],
    children: [
      { path: '', pathMatch: 'full', redirectTo: 'cockpit' },
      {
        path: 'cockpit',
        loadComponent: () =>
          import('./features/cockpit/cockpit.component').then((m) => m.CockpitComponent),
      },
      {
        path: 'patrimoine',
        loadComponent: () =>
          import('./features/wealth/wealth-dashboard.component').then((m) => m.WealthDashboardComponent),
      },
      {
        path: 'securite',
        loadComponent: () =>
          import('./features/settings/security.component').then((m) => m.SecurityComponent),
      },
    ],
  },
];
