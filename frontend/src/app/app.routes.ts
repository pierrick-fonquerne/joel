import { Routes } from '@angular/router';
import { ShellComponent } from './core/layout/shell.component';

export const routes: Routes = [
  {
    path: '',
    component: ShellComponent,
    children: [
      { path: '', pathMatch: 'full', redirectTo: 'cockpit' },
      {
        path: 'cockpit',
        loadComponent: () =>
          import('./features/cockpit/cockpit.component').then((m) => m.CockpitComponent),
      },
    ],
  },
];
