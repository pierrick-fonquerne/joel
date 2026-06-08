import { Component, inject, signal } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { Router } from '@angular/router';
import { AuthService } from '../../core/auth/auth.service';

@Component({
  selector: 'app-login',
  imports: [FormsModule],
  template: `
    <div class="login">
      <h1>Joel</h1>
      <p>Votre cabinet vous attend.</p>

      <button type="button" class="login__passkey" (click)="passkey()">Se connecter avec une passkey</button>

      @if (fallback()) {
        <form (ngSubmit)="password()">
          <input name="email" type="email" [(ngModel)]="email" placeholder="Email" required />
          <input name="password" type="password" [(ngModel)]="pwd" placeholder="Mot de passe" required />
          <input name="totp" inputmode="numeric" [(ngModel)]="totp" placeholder="Code TOTP" required />
          <button type="submit">Connexion</button>
        </form>
      } @else {
        <button type="button" class="login__alt" (click)="fallback.set(true)">Utiliser le mot de passe</button>
      }

      @if (error()) {
        <p class="login__error">Connexion impossible. Vérifiez vos informations.</p>
      }
    </div>
  `,
  styles: `
    .login { max-width: 22rem; margin: 15vh auto; display: flex; flex-direction: column; gap: 0.8rem; text-align: center; }
    .login form { display: flex; flex-direction: column; gap: 0.6rem; }
    .login input, .login button { padding: 0.7rem; font-size: 1rem; }
    .login__passkey { background: #101418; color: #fff; border: none; border-radius: 6px; }
    .login__alt { background: none; border: none; color: #444; text-decoration: underline; }
    .login__error { color: #b00020; }
  `,
})
export class LoginComponent {
  private readonly auth = inject(AuthService);
  private readonly router = inject(Router);

  email = '';
  pwd = '';
  totp = '';
  readonly fallback = signal(false);
  readonly error = signal(false);

  async passkey(): Promise<void> {
    await this.attempt(() => this.auth.loginWithPasskey());
  }

  async password(): Promise<void> {
    await this.attempt(() => this.auth.loginWithPassword(this.email, this.pwd, this.totp));
  }

  private async attempt(action: () => Promise<void>): Promise<void> {
    this.error.set(false);
    try {
      await action();
      await this.router.navigateByUrl('/cockpit');
    } catch {
      this.error.set(true);
    }
  }
}
