CREATE TABLE wealth_accounts (
    id UUID PRIMARY KEY,
    kind TEXT NOT NULL CHECK (kind IN (
        'brokerage_pea', 'brokerage_cto', 'life_insurance', 'retirement_plan', 'bank_account',
        'savings', 'crypto_wallet', 'real_estate', 'company_shares', 'loan')),
    owner TEXT NOT NULL CHECK (owner IN ('personal', 'company')),
    currency TEXT NOT NULL CHECK (currency ~ '^[A-Z]{3}$'),
    name BYTEA NOT NULL,
    notes BYTEA,
    is_archived BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE TABLE wealth_valuations (
    id UUID PRIMARY KEY,
    account_id UUID NOT NULL REFERENCES wealth_accounts(id),
    as_of DATE NOT NULL,
    currency TEXT NOT NULL CHECK (currency ~ '^[A-Z]{3}$'),
    amount BYTEA NOT NULL,
    source TEXT NOT NULL CHECK (source IN ('manual', 'csv_import', 'bank_aggregation', 'price_feed')),
    recorded_at TIMESTAMPTZ NOT NULL
);
CREATE INDEX wealth_valuations_latest_idx
    ON wealth_valuations (account_id, as_of DESC, recorded_at DESC);

CREATE FUNCTION wealth_valuations_reject_change() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'wealth_valuations is append-only';
END;
$$;
CREATE TRIGGER wealth_valuations_append_only
    BEFORE UPDATE OR DELETE ON wealth_valuations
    FOR EACH ROW EXECUTE FUNCTION wealth_valuations_reject_change();

CREATE TABLE wealth_exchange_rates (
    currency TEXT NOT NULL CHECK (currency ~ '^[A-Z]{3}$'),
    on_date DATE NOT NULL,
    units_per_eur NUMERIC(20, 10) NOT NULL CHECK (units_per_eur > 0),
    PRIMARY KEY (currency, on_date)
);

CREATE TABLE wealth_keys (
    version INTEGER GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    egide_key_name TEXT NOT NULL,
    wrapped_key TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
