import { Component } from '@angular/core';

@Component({
  selector: 'app-vault-sealed',
  template: `
    <section class="sealed">
      <h3>Le coffre patrimoine est scellé</h3>
      <p>Descelle Egide avec tes parts, puis recharge la page d'ici 30 secondes.</p>
    </section>
  `,
  styles: `.sealed { padding: 1rem; border-radius: 0.5rem; background: #2a2116; color: #f3d9a4; }`,
})
export class VaultSealedComponent {}
