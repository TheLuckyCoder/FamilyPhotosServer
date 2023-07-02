-- Add migration script here
CREATE TABLE users
(
    user_name     TEXT PRIMARY KEY,
    name          TEXT NOT NULL,
    password_hash TEXT NOT NULL
);