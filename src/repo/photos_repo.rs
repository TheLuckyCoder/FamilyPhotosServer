use crate::model::photo::{Photo, PhotoBase, PhotoBody, PhotoDto};
use crate::utils::internal_error;
use axum::response::ErrorResponse;
use sqlx::{query, query_as, PgPool, Postgres, QueryBuilder};

#[derive(Clone)]
pub struct PhotosRepository {
    pool: PgPool,
}

impl PhotosRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn get_photo(&self, id: i64) -> Result<Photo, ErrorResponse> {
        query_as!(Photo, "select * from photos where id = $1", id)
            .fetch_one(&self.pool) // fetch_optional
            .await
            .map_err(internal_error)
    }

    pub async fn get_photos(&self) -> Result<Vec<Photo>, ErrorResponse> {
        query_as!(Photo, "select * from photos order by created_at desc")
            .fetch_all(&self.pool)
            .await
            .map_err(internal_error)
    }

    pub async fn get_photos_by_user<T: AsRef<str>>(
        &self,
        user_id: T,
    ) -> Result<Vec<Photo>, ErrorResponse> {
        query_as!(
            Photo,
            "select * from photos where photos.user_id = $1 order by photos.created_at desc",
            user_id.as_ref()
        )
        .fetch_all(&self.pool)
        .await
        .map_err(internal_error)
    }

    pub async fn get_photos_dto_by_user<T: AsRef<str>>(
        &self,
        user_id: T,
    ) -> Result<Vec<PhotoDto>, ErrorResponse> {
        query_as!(
            PhotoDto,
            "select photos.*,\
                exists(select 1 from favorite_photos as fp where fp.user_id = $1 and fp.photo_id = photos.id) \
                as \"is_favorite!\" from photos where photos.user_id = $1 order by photos.created_at desc",
            user_id.as_ref()
        )
        .fetch_all(&self.pool)
        .await
        .map_err(internal_error)
    }

    pub async fn insert_photo(&self, photo: &PhotoBody) -> Result<Photo, ErrorResponse> {
        query_as!(
            Photo,
            "insert into photos (user_id, name, created_at, file_size, folder) values ($1, $2, $3, $4, $5) returning *",
            photo.user_id(),
            photo.name(),
            photo.created_at(),
            photo.file_size(),
            photo.folder_name()
        )
        .fetch_one(&self.pool)
        .await
        .map_err(internal_error)
    }

    pub async fn insert_photos(&self, photos: &[PhotoBody]) -> Result<(), sqlx::Error> {
        let mut query_builder: QueryBuilder<Postgres> =
            QueryBuilder::new("insert into photos (user_id, name, created_at, file_size, folder) ");

        query_builder.push_values(photos, |mut b, photo| {
            b.push_bind(photo.user_id())
                .push_bind(photo.name())
                .push_bind(photo.created_at())
                .push_bind(photo.file_size())
                .push_bind(photo.folder_name());
        });

        query_builder.build().execute(&self.pool).await.map(|_| ())
    }

    pub async fn insert_favorite<T: AsRef<str>>(
        &self,
        photo_id: i64,
        user_id: T,
    ) -> Result<(), ErrorResponse> {
        query!(
            "insert into favorite_photos (photo_id, user_id) values ($1, $2)",
            photo_id,
            user_id.as_ref()
        )
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(internal_error)
    }

    pub async fn delete_favorite<T: AsRef<str>>(
        &self,
        photo_id: i64,
        user_id: T,
    ) -> Result<(), ErrorResponse> {
        query!(
            "delete from favorite_photos where photo_id = $1 and user_id = $2",
            photo_id,
            user_id.as_ref()
        )
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(internal_error)
    }

    pub async fn update_photo(&self, photo: &Photo) -> Result<(), ErrorResponse> {
        query!(
            "update photos set user_id = $2, name = $3, created_at = $4, file_size = $5, folder = $6 where id = $1",
            photo.id(),
            photo.user_id(),
            photo.name(),
            photo.created_at(),
            photo.file_size(),
            photo.folder_name()
        )
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(internal_error)
    }

    pub async fn delete_photo(&self, id: i64) -> Result<(), ErrorResponse> {
        query!("delete from photos where id = $1", id)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(internal_error)
    }

    pub async fn delete_photos(&self, photo_ids: &[i64]) -> Result<(), sqlx::Error> {
        query!(
            "delete from photos where id in (select * from UNNEST($1::int8[]))",
            photo_ids
        )
        .execute(&self.pool)
        .await
        .map(|_| ())
    }
}
