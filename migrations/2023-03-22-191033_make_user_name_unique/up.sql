-- Your SQL goes here

ALTER TABLE users
ADD CONSTRAINT unique_user_name UNIQUE (user_name);

CREATE SEQUENCE users_id_seq MINVALUE 10;

ALTER TABLE users
ALTER id SET DEFAULT nextval('users_id_seq');

ALTER SEQUENCE users_id_seq OWNED BY users.id;