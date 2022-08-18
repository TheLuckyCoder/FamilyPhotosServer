-- Your SQL goes here
CREATE TABLE photos (
    id INT8 NOT NULL PRIMARY KEY,
    owner INT8 NOT NULL,
    name TEXT NOT NULL,
    time_created TIMESTAMP NOT NULL,
    file_size INT8 NOT NULL,
    folder TEXT
);