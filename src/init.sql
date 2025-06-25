CREATE TABLE events (
  id SERIAL PRIMARY KEY,
  setter TEXT,
  value TEXT,
  inserted_at TIMESTAMP DEFAULT now()
);
