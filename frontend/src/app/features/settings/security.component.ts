import { Component, inject, signal } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { AuthService } from '../../core/auth/auth.service';

@Component({
  selector: 'app-security',
  imports: [FormsModule],
  template: `
    <h2>Sécurité</h2>
    <p>Enregistrez une passkey sur cet appareil pour vous connecter avec Face ID ou Windows Hello.</p>
    <form (ngSubmit)="add()">
      <input name="label" [(ngModel)]="label" placeholder="Nom de l'appareil (iPhone, PC…)" required />
      <button type="submit">Ajouter une passkey</button>
    </form>
    @if (done()) {
      <p>Passkey enregistrée ✔</p>
    }
    @if (error()) {
      <p>Échec de l'enregistrement.</p>
    }
  `,
})
export class SecurityComponent {
  private readonly auth = inject(AuthService);
  label = '';
  readonly done = signal(false);
  readonly error = signal(false);

  async add(): Promise<void> {
    this.done.set(false);
    this.error.set(false);
    try {
      await this.auth.registerPasskey(this.label);
      this.done.set(true);
    } catch {
      this.error.set(true);
    }
  }
}
