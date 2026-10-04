import { WealthAmountPipe } from './wealth-amount.pipe';

describe('WealthAmountPipe', () => {
  const pipe = new WealthAmountPipe();

  it('formats euros the French way', () => {
    expect(pipe.transform('1234.5', 'EUR', false).replace(/\s/g, ' ')).toBe('1 234,50 €');
  });

  it('masks amounts in discreet mode', () => {
    expect(pipe.transform('1234.5', 'EUR', true)).toBe('••••• €');
  });

  it('shows a dash when there is no amount', () => {
    expect(pipe.transform(null, 'EUR', false)).toBe('-');
  });
});
