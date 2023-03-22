-- This file should undo anything in `up.sql`

ALTER TABLE users
DROP CONSTRAINT unique_user_name;

ALTER TABLE users
ALTER id DROP DEFAULT;

DROP SEQUENCE users_id_seq;
