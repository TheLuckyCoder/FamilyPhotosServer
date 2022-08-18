-- Your SQL goes here
CREATE TABLE users (
    id INT8 NOT NULL PRIMARY KEY,
    display_name TEXT NOT NULL,
    user_name TEXT NOT NULL,
    password TEXT NOT NULL
);