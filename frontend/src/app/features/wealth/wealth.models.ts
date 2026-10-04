export type Owner = 'personal' | 'company';

export type AccountKind =
  | 'brokerage_pea'
  | 'brokerage_cto'
  | 'life_insurance'
  | 'retirement_plan'
  | 'bank_account'
  | 'savings'
  | 'crypto_wallet'
  | 'real_estate'
  | 'company_shares'
  | 'loan';

export interface Valuation {
  id: string;
  as_of: string;
  amount: string;
  currency: string;
  source: string;
  recorded_at: string;
}

export interface Account {
  id: string;
  name: string;
  kind: AccountKind;
  owner: Owner;
  currency: string;
  is_archived: boolean;
  notes: string | null;
  latest_valuation: Valuation | null;
  is_stale: boolean;
}

export interface NetWorth {
  as_of: string;
  total: string;
  by_owner: Partial<Record<Owner, string>>;
  by_kind: Partial<Record<AccountKind, string>>;
  stale_account_ids: string[];
}

export interface NewAccount {
  name: string;
  kind: AccountKind;
  owner: Owner;
  currency: string;
  notes: string | null;
}

export const ACCOUNT_KINDS: readonly AccountKind[] = [
  'brokerage_pea',
  'brokerage_cto',
  'life_insurance',
  'retirement_plan',
  'bank_account',
  'savings',
  'crypto_wallet',
  'real_estate',
  'company_shares',
  'loan',
];

export const ACCOUNT_KIND_LABELS: Record<AccountKind, string> = {
  brokerage_pea: 'PEA',
  brokerage_cto: 'Compte-titres',
  life_insurance: 'Assurance-vie',
  retirement_plan: 'PER',
  bank_account: 'Compte bancaire',
  savings: 'Épargne',
  crypto_wallet: 'Crypto',
  real_estate: 'Immobilier',
  company_shares: 'Parts de société',
  loan: 'Prêt',
};

export const OWNER_LABELS: Record<Owner, string> = {
  personal: 'Perso',
  company: 'SASU',
};
