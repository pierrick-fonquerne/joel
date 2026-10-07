import { normalizeAmount } from './amount-input';

describe('normalizeAmount', () => {
  it('accepts French keyboard input', () => {
    expect(normalizeAmount('1 234,56')).toBe('1234.56');
    expect(normalizeAmount('1 234,5')).toBe('1234.5');
    expect(normalizeAmount(' 250000 ')).toBe('250000');
    expect(normalizeAmount('-180 000')).toBe('-180000');
    expect(normalizeAmount('12.30')).toBe('12.30');
  });

  it('rejects ambiguous or invalid input', () => {
    for (const input of ['', 'abc', '12,345', '1e3', '1,2,3', '1.', ',5', '+3']) {
      expect(normalizeAmount(input)).withContext(input).toBeNull();
    }
  });
});
