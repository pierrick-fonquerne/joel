import { Component } from '@angular/core';
import { RouterLink, RouterLinkActive, RouterOutlet } from '@angular/router';

@Component({
  selector: 'app-shell',
  imports: [RouterOutlet, RouterLink, RouterLinkActive],
  template: `
    <div class="shell">
      <aside class="shell__nav">
        <h1 class="shell__brand">Joel</h1>
        <nav>
          <a routerLink="/cockpit" routerLinkActive="active">Cockpit</a>
          <a routerLink="/securite" routerLinkActive="active">Sécurité</a>
          <span class="shell__soon">Agents</span>
          <span class="shell__soon">Routines</span>
          <span class="shell__soon">Presse</span>
          <span class="shell__soon">Planning</span>
        </nav>
      </aside>
      <main class="shell__content"><router-outlet /></main>
    </div>
  `,
  styles: `
    .shell { display: flex; min-height: 100vh; }
    .shell__nav { width: 13rem; padding: 1rem; background: #101418; color: #e8eaed; }
    .shell__brand { font-size: 1.4rem; margin: 0 0 1.5rem; }
    .shell__nav nav { display: flex; flex-direction: column; gap: 0.75rem; }
    .shell__nav a { color: #e8eaed; text-decoration: none; }
    .shell__nav a.active { font-weight: 700; }
    .shell__soon { color: #5f6a72; cursor: default; }
    .shell__content { flex: 1; padding: 1.5rem; }
    @media (max-width: 700px) {
      .shell { flex-direction: column; }
      .shell__nav { width: auto; }
      .shell__nav nav { flex-direction: row; flex-wrap: wrap; }
    }
  `,
})
export class ShellComponent {}
