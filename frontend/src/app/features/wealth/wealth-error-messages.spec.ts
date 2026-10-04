import { HttpErrorResponse } from '@angular/common/http';
import { errorCodeOf, wealthErrorMessage } from './wealth-error-messages';

describe('wealth error messages', () => {
  it('extracts the code from an http error body', () => {
    expect(errorCodeOf(new HttpErrorResponse({ status: 400, error: { code: 'invalid_amount' } }))).toBe('invalid_amount');
  });

  it('returns undefined for other errors', () => {
    expect(errorCodeOf(new Error('boom'))).toBeUndefined();
    expect(errorCodeOf(new HttpErrorResponse({ status: 500, error: null }))).toBeUndefined();
  });

  it('maps known codes and falls back otherwise', () => {
    expect(wealthErrorMessage('wealth_vault_sealed', 'x')).toBe('Coffre scellé : descelle Egide puis réessaie');
    expect(wealthErrorMessage('unknown_code', 'repli')).toBe('repli');
    expect(wealthErrorMessage(undefined, 'repli')).toBe('repli');
  });
});
