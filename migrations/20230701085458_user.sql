-- Add migration script here
CREATE TABLE users
(
    id            TEXT PRIMARY KEY,
    name          TEXT NOT NULL,
    password_hash TEXT NOT NULL
);