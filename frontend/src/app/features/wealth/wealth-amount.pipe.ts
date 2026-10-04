import { Pipe, PipeTransform } from '@angular/core';

const MASK = '•••••';

@Pipe({ name: 'wealthAmount' })
export class WealthAmountPipe implements PipeTransform {
  /** Formats a decimal string for display only; arithmetic stays on the server. */
  transform(amount: string | null | undefined, currency = 'EUR', isDiscreet = false): string {
    if (amount === null || amount === undefined) {
      return '-';
    }
    if (isDiscreet) {
      const symbol = currency === 'EUR' ? '€' : currency;
      return `${MASK} ${symbol}`;
    }
    return new Intl.NumberFormat('fr-FR', { style: 'currency', currency }).format(Number(amount));
  }
}
