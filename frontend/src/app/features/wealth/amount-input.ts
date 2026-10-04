const AMOUNT_PATTERN = /^-?\d+(\.\d{1,2})?$/;

/**
 * Turns what a French keyboard produces ("1 234,56") into the API format ("1234.56").
 * Returns null when the input is not an amount with at most two decimals.
 */
export function normalizeAmount(input: string): string | null {
  const compact = input.replace(/\s/g, '');
  if ((compact.match(/,/g) ?? []).length > 1) {
    return null;
  }
  const normalized = compact.replace(',', '.');
  return AMOUNT_PATTERN.test(normalized) ? normalized : null;
}
