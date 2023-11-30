use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::utils::primitive_date_time_serde;

pub trait PhotoBase {
    fn user_id(&self) -> &String;

    fn name(&self) -> &String;

    fn created_at(&self) -> PrimitiveDateTime;

    fn file_size(&self) -> i64;

    fn folder_name(&self) -> Option<&String>;

    fn full_name(&self) -> String {
        let folder_path = match self.folder_name().as_ref() {
            None => String::new(),
            Some(folder) => format!("{folder}/"),
        };

        folder_path + self.name().as_str()
    }

    fn partial_path(&self) -> String {
        format!("{}/{}", self.user_id(), self.full_name())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Photo {
    pub id: i64,
    pub user_id: String,
    pub name: String,
    #[serde(with = "primitive_date_time_serde")]
    pub created_at: PrimitiveDateTime,
    pub file_size: i64,
    pub folder: Option<String>,
}

impl PhotoBase for Photo {
    fn user_id(&self) -> &String {
        &self.user_id
    }

    fn name(&self) -> &String {
        &self.name
    }

    fn created_at(&self) -> PrimitiveDateTime {
        self.created_at
    }

    fn file_size(&self) -> i64 {
        self.file_size
    }

    fn folder_name(&self) -> Option<&String> {
        self.folder.as_ref()
    }
}

impl Photo {
    pub fn id(&self) -> i64 {
        self.id
    }

    pub fn partial_thumbnail_path(&self) -> String {
        let special_extension = match self.name.rsplit_once('.') {
            None => None,
            Some((_before, after)) => {
                if after == "heic" || after == "heif" {
                    Some("png")
                } else {
                    None
                }
            }
        };

        format!("{}.{}", self.id, special_extension.unwrap_or("jpg"))
    }
}

#[derive(Debug, Clone)]
pub struct PhotoBody {
    user_name: String,
    name: String,
    created_at: PrimitiveDateTime,
    file_size: i64,
    folder: Option<String>,
}

impl PhotoBase for PhotoBody {
    fn user_id(&self) -> &String {
        &self.user_name
    }

    fn name(&self) -> &String {
        &self.name
    }

    fn created_at(&self) -> PrimitiveDateTime {
        self.created_at
    }

    fn file_size(&self) -> i64 {
        self.file_size
    }

    fn folder_name(&self) -> Option<&String> {
        self.folder.as_ref()
    }
}

impl PhotoBody {
    pub fn new(
        user_name: String,
        name: String,
        created_at: PrimitiveDateTime,
        file_size: i64,
        folder: Option<String>,
    ) -> Self {
        Self {
            user_name,
            name,
            created_at,
            file_size,
            folder,
        }
    }
    
    pub fn set_file_size(&mut self, value: i64) {
        self.file_size = value;
    }
}
