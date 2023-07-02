-- Add migration script here
CREATE TABLE photos
(
    id         INT8 GENERATED ALWAYS AS IDENTITY,
    user_name  TEXT      NOT NULL,
    name       TEXT      NOT NULL,
    created_at TIMESTAMP NOT NULL,
    file_size  INT8      NOT NULL,
    folder     TEXT,
    PRIMARY KEY (id),
    CONSTRAINT fk_user
        FOREIGN KEY (user_name)
            REFERENCES users (user_name)
);