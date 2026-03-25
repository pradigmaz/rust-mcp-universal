CREATE TABLE origins (
  id INTEGER PRIMARY KEY,
  origin_key TEXT NOT NULL
);

CREATE UNIQUE INDEX uq_origins_origin_key
ON origins(origin_key);
