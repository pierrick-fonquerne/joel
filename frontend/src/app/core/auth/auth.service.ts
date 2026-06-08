import { inject, Injectable, signal } from '@angular/core';
import { HttpClient } from '@angular/common/http';
import { firstValueFrom } from 'rxjs';
import { base64UrlToBuffer, bufferToBase64Url } from './webauthn.helpers';
import { LoggerService } from '../logging/logger.service';

export interface Identity {
  email: string;
  display_name: string;
}

@Injectable({ providedIn: 'root' })
export class AuthService {
  private readonly http = inject(HttpClient);
  private readonly logger = inject(LoggerService);

  /** Identity of the connected user; null when logged out or unknown. */
  readonly identity = signal<Identity | null>(null);

  async loginWithPassword(email: string, password: string, totp: string): Promise<void> {
    await firstValueFrom(this.http.post('/api/auth/login', { email, password, totp }));
    await this.refreshIdentity();
    this.logger.info('auth.login', { method: 'password' });
  }

  async loginWithPasskey(): Promise<void> {
    const start = await firstValueFrom(
      this.http.post<{ challenge_id: string; options: { publicKey: Record<string, unknown> } }>(
        '/api/auth/webauthn/login/start',
        {},
      ),
    );
    const publicKey = start.options.publicKey;
    const assertion = (await navigator.credentials.get({
      publicKey: {
        ...publicKey,
        challenge: base64UrlToBuffer(publicKey['challenge'] as string),
      } as PublicKeyCredentialRequestOptions,
    })) as PublicKeyCredential;
    const response = assertion.response as AuthenticatorAssertionResponse;
    await firstValueFrom(
      this.http.post('/api/auth/webauthn/login/finish', {
        challenge_id: start.challenge_id,
        credential: {
          id: assertion.id,
          rawId: bufferToBase64Url(assertion.rawId),
          type: assertion.type,
          response: {
            authenticatorData: bufferToBase64Url(response.authenticatorData),
            clientDataJSON: bufferToBase64Url(response.clientDataJSON),
            signature: bufferToBase64Url(response.signature),
            userHandle: response.userHandle ? bufferToBase64Url(response.userHandle) : null,
          },
        },
      }),
    );
    await this.refreshIdentity();
    this.logger.info('auth.login', { method: 'passkey' });
  }

  async registerPasskey(label: string): Promise<void> {
    const options = await firstValueFrom(
      this.http.post<{ publicKey: Record<string, unknown> }>('/api/auth/webauthn/register/start', {}),
    );
    const publicKey = options.publicKey;
    const user = publicKey['user'] as Record<string, unknown>;
    const credential = (await navigator.credentials.create({
      publicKey: {
        ...publicKey,
        challenge: base64UrlToBuffer(publicKey['challenge'] as string),
        user: { ...user, id: base64UrlToBuffer(user['id'] as string) },
        excludeCredentials: ((publicKey['excludeCredentials'] as { id: string; type: string }[] | undefined) ?? []).map(
          (c) => ({ ...c, id: base64UrlToBuffer(c.id) }),
        ),
      } as PublicKeyCredentialCreationOptions,
    })) as PublicKeyCredential;
    const response = credential.response as AuthenticatorAttestationResponse;
    await firstValueFrom(
      this.http.post('/api/auth/webauthn/register/finish', {
        label,
        credential: {
          id: credential.id,
          rawId: bufferToBase64Url(credential.rawId),
          type: credential.type,
          response: {
            attestationObject: bufferToBase64Url(response.attestationObject),
            clientDataJSON: bufferToBase64Url(response.clientDataJSON),
          },
        },
      }),
    );
    this.logger.info('auth.passkey.registered', { label });
  }

  async refreshIdentity(): Promise<void> {
    try {
      this.identity.set(await firstValueFrom(this.http.get<Identity>('/api/auth/me')));
    } catch {
      this.identity.set(null);
    }
  }

  async logout(): Promise<void> {
    await firstValueFrom(this.http.post('/api/auth/logout', {}));
    this.identity.set(null);
    this.logger.info('auth.logout', {});
  }
}
