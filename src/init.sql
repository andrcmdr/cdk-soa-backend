CREATE TABLE events (
  id SERIAL PRIMARY KEY,
  contract_address TEXT NOT NULL,
  event_name TEXT NOT NULL,
  parameters JSONB NOT NULL,
  inserted_at TIMESTAMP DEFAULT now()
);
