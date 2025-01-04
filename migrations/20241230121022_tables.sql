CREATE TABLE users
(
    id            TEXT NOT NULL PRIMARY KEY,
    name          TEXT NOT NULL,
    password_hash TEXT NOT NULL
);

CREATE TABLE photos
(
    id         INTEGER  NOT NULL PRIMARY KEY,
    user_id    TEXT     NOT NULL,
    name       TEXT     NOT NULL,
    created_at DATETIME NOT NULL,
    file_size  INTEGER  NOT NULL,
    folder     TEXT,

    FOREIGN KEY (user_id) REFERENCES users (id)
);

CREATE INDEX photos_created_at_desc_index ON photos (created_at DESC);

CREATE TABLE favorite_photos
(
    user_id  TEXT    NOT NULL,
    photo_id INTEGER NOT NULL,
    PRIMARY KEY (user_id, photo_id),
    FOREIGN KEY (user_id) REFERENCES users (id),
    FOREIGN KEY (photo_id) REFERENCES photos (id)
);