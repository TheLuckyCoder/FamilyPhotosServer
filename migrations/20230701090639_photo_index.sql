-- Add migration script here
CREATE INDEX photos_created_at_desc_index ON photos (created_at DESC);