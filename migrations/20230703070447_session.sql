-- Add migration script here
CREATE TABLE session
(
    key     TEXT NOT NULL,
    session TEXT NOT NULL,
    cookie  TEXT,
    PRIMARY KEY (key)
);