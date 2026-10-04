import { HttpErrorResponse } from '@angular/common/http';

const WEALTH_ERROR_MESSAGES: Record<string, string> = {
  invalid_currency: 'Devise invalide : 3 lettres, par exemple EUR',
  unsupported_currency: 'Devise non suivie par la BCE : crée le compte en EUR',
  invalid_amount: 'Montant invalide',
  invalid_account_kind: 'Type de compte invalide',
  invalid_owner: 'Propriétaire invalide',
  empty_account_name: 'Donne un nom au compte',
  positive_loan_valuation: 'Un prêt se saisit en négatif',
  negative_asset_valuation: 'Un actif ne peut pas être négatif',
  currency_mismatch: 'La devise ne correspond pas au compte',
  future_valuation: 'La date est dans le futur',
  archived_account: 'Ce compte est archivé',
  invalid_date: 'Date invalide',
  account_not_found: 'Compte introuvable',
  exchange_rate_missing: 'Taux BCE introuvable pour un relevé en devise',
  invalid_range: 'Période invalide',
  wealth_vault_sealed: 'Coffre scellé : descelle Egide puis réessaie',
};

/** Extracts the backend error code from an HTTP error body, if any. */
export function errorCodeOf(error: unknown): string | undefined {
  if (!(error instanceof HttpErrorResponse)) {
    return undefined;
  }
  const code = (error.error as { code?: unknown } | null)?.code;
  return typeof code === 'string' ? code : undefined;
}

/** Maps a backend error code to its French message, or the fallback when unknown. */
export function wealthErrorMessage(code: string | undefined, fallback: string): string {
  return (code !== undefined && WEALTH_ERROR_MESSAGES[code]) || fallback;
}
