-- Your SQL goes here
-- The type is VARCHAR(32) as this will store the merchant_connector_account id
-- which will be generated by the application using default length
ALTER TABLE payment_attempt
ADD COLUMN IF NOT EXISTS merchant_connector_id VARCHAR(32);
