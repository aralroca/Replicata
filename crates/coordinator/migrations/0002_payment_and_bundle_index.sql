-- A settlement transaction can only pay for one delivery.
CREATE UNIQUE INDEX IF NOT EXISTS payments_tx_signature ON payments (tx_signature) WHERE tx_signature IS NOT NULL;

-- File names inside each bundle, so paid-route guards never load or unpack blobs.
ALTER TABLE bundles ADD COLUMN IF NOT EXISTS files TEXT[] NOT NULL DEFAULT '{}';
