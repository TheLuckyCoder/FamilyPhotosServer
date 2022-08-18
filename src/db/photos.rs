use actix::{Handler, Message};
use diesel::prelude::*;
use rand::prelude::*;

use crate::DbActor;
use crate::model::photo::{Photo, PhotoBody};
use crate::schema::photos::dsl::{id, owner, photos};

#[derive(Message)]
#[rtype(result = "QueryResult<Vec<Photo>>")]
pub struct GetPhotos {
    pub owner: i64,
}

#[derive(Message)]
#[rtype(result = "QueryResult<Photo>")]
pub struct GetPhoto {
    pub id: i64,
}

#[derive(Message)]
#[rtype(result = "QueryResult<Photo>")]
pub struct CreatePhoto(pub PhotoBody);

#[derive(Message)]
#[rtype(result = "QueryResult<Photo>")]
pub struct UpdatePhoto(pub Photo);

#[derive(Message)]
#[rtype(result = "QueryResult<usize>")]
pub struct DeletePhoto {
    pub id: i64,
}

impl Handler<GetPhotos> for DbActor {
    type Result = QueryResult<Vec<Photo>>;

    fn handle(&mut self, msg: GetPhotos, _: &mut Self::Context) -> Self::Result {
        let mut conn = self.0.get().expect("Unable to get a connection");
        photos.filter(owner.eq(msg.owner))
            .get_results::<Photo>(&mut conn)
    }
}

impl Handler<GetPhoto> for DbActor {
    type Result = QueryResult<Photo>;

    fn handle(&mut self, msg: GetPhoto, _: &mut Self::Context) -> Self::Result {
        let mut conn = self.0.get().expect("Unable to get a connection");
        photos.filter(id.eq(msg.id)).get_result::<Photo>(&mut conn)
    }
}

impl Handler<CreatePhoto> for DbActor {
    type Result = QueryResult<Photo>;

    fn handle(&mut self, msg: CreatePhoto, _: &mut Self::Context) -> Self::Result {
        let body = msg.0;
        let generated_id = {
            let rng = &mut self.1.lock().unwrap();
            rng.gen::<i64>()
        };
        let photo = Photo {
            id: generated_id,
            owner: body.owner,
            name: body.name,
            time_created: body.time_created,
            file_size: body.file_size,
            folder: body.folder,
        };

        let mut conn = self.0.get().expect("Unable to get a connection");

        diesel::insert_into(photos)
            .values(photo)
            .get_result::<Photo>(&mut conn)
    }
}

impl Handler<UpdatePhoto> for DbActor {
    type Result = QueryResult<Photo>;

    fn handle(&mut self, msg: UpdatePhoto, _: &mut Self::Context) -> Self::Result {
        let mut conn = self.0.get().expect("Unable to get a connection");
        let photo = msg.0;

        diesel::update(photos).set(&photo).get_result(&mut conn)
    }
}

impl Handler<DeletePhoto> for DbActor {
    type Result = QueryResult<usize>;

    fn handle(&mut self, msg: DeletePhoto, _: &mut Self::Context) -> Self::Result {
        let mut conn = self.0.get().expect("Unable to get a connection");

        diesel::delete(photos.filter(id.eq(msg.id))).execute(&mut conn)
    }
}