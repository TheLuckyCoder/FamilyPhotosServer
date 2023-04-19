use crate::db::{Handler, Pool};
use async_trait::async_trait;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use rand::prelude::*;

use crate::model::photo::{Photo, PhotoBody};
use crate::schema::photos::dsl::{id, owner, photos, time_created};

pub enum GetPhotos {
    All,
    Owner(i64),
}

pub struct GetPhoto {
    pub id: i64,
}

pub struct InsertPhoto(pub PhotoBody);

pub struct InsertPhotos(pub Vec<Photo>);

pub struct UpdatePhoto(pub Photo);

pub struct DeletePhoto {
    pub id: i64,
}

pub struct DeletePhotos {
    pub ids: Vec<i64>,
}

#[async_trait]
impl Handler<GetPhotos> for Pool {
    type Result = QueryResult<Vec<Photo>>;

    async fn send(&self, msg: GetPhotos) -> Self::Result {
        let mut conn = self.0.get().await.expect("Unable to get a connection");

        match msg {
            GetPhotos::All => photos
                .order(time_created.desc())
                .get_results::<Photo>(&mut conn),
            GetPhotos::Owner(owner_id) => photos
                .filter(owner.eq(owner_id))
                .order(time_created.desc())
                .get_results::<Photo>(&mut conn),
        }
        .await
    }
}

#[async_trait]
impl Handler<GetPhoto> for Pool {
    type Result = QueryResult<Photo>;

    async fn send(&self, msg: GetPhoto) -> Self::Result {
        let mut conn = self.0.get().await.expect("Unable to get a connection");
        photos
            .filter(id.eq(msg.id))
            .get_result::<Photo>(&mut conn)
            .await
    }
}

#[async_trait]
impl Handler<InsertPhoto> for Pool {
    type Result = QueryResult<Photo>;

    async fn send(&self, msg: InsertPhoto) -> Self::Result {
        let body = msg.0;
        let generated_id = {
            let rng = &mut self.1.lock().await;
            rng.gen::<i64>()
        };

        let photo = Photo {
            id: generated_id,
            owner: body.owner,
            name: body.name,
            time_created: body.time_created,
            file_size: body.file_size,
            folder: body.folder,
            caption: None,
        };

        let mut conn = self.0.get().await.expect("Unable to get a connection");

        diesel::insert_into(photos)
            .values(photo)
            .get_result::<Photo>(&mut conn)
            .await
    }
}

#[async_trait]
impl Handler<InsertPhotos> for Pool {
    type Result = QueryResult<usize>;

    async fn send(&self, msg: InsertPhotos) -> Self::Result {
        let rng = &mut self.1.lock().await;

        let new_photos = msg
            .0
            .into_iter()
            .map(|body| Photo {
                id: rng.gen::<i64>(),
                owner: body.owner,
                name: body.name,
                time_created: body.time_created,
                file_size: body.file_size,
                folder: body.folder,
                caption: None,
            })
            .collect::<Vec<Photo>>();

        let mut conn = self.0.get().await.expect("Unable to get a connection");

        diesel::insert_into(photos)
            .values(new_photos)
            .execute(&mut conn)
            .await
    }
}

#[async_trait]
impl Handler<UpdatePhoto> for Pool {
    type Result = QueryResult<Photo>;

    async fn send(&self, msg: UpdatePhoto) -> Self::Result {
        let mut conn = self.0.get().await.expect("Unable to get a connection");
        let photo = msg.0;

        diesel::update(photos.filter(id.eq(photo.id)))
            .set(&photo)
            .get_result(&mut conn)
            .await
    }
}

#[async_trait]
impl Handler<DeletePhoto> for Pool {
    type Result = QueryResult<usize>;

    async fn send(&self, msg: DeletePhoto) -> Self::Result {
        let mut conn = self.0.get().await.expect("Unable to get a connection");

        diesel::delete(photos.filter(id.eq(msg.id)))
            .execute(&mut conn)
            .await
    }
}

#[async_trait]
impl Handler<DeletePhotos> for Pool {
    type Result = QueryResult<usize>;

    async fn send(&self, msg: DeletePhotos) -> Self::Result {
        let mut conn = self.0.get().await.expect("Unable to get a connection");

        diesel::delete(photos.filter(id.eq_any(msg.ids)))
            .execute(&mut conn)
            .await
    }
}
