CREATE TABLE favorite_photos
(
    user_id  TEXT NOT NULL,
    photo_id INT8 NOT NULL,
    PRIMARY KEY (user_id, photo_id),
    CONSTRAINT fk_user FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE,
    CONSTRAINT fb_photo FOREIGN KEY (photo_id) REFERENCES photos (id) ON DELETE CASCADE
);