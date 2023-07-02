use serde::{Deserialize, Serialize};

use crate::utils::primitive_date_time_serde;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Photo {
    pub id: i64,
    pub user_name: String,
    pub name: String,
    #[serde(with = "primitive_date_time_serde")]
    pub created_at: time::PrimitiveDateTime,
    pub file_size: i64,
    pub folder: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PhotoBody {
    pub user_name: String,
    pub name: String,
    pub created_at: time::PrimitiveDateTime,
    pub file_size: i64,
    pub folder: Option<String>,
}

impl Photo {
    pub fn full_name(&self) -> String {
        let folder_path = match self.folder.as_ref() {
            None => String::new(),
            Some(folder) => format!("{folder}/"),
        };

        folder_path + self.name.as_str()
    }

    pub fn partial_path(&self) -> Result<String, String> {
        Ok(format!("{}/{}", self.user_name, self.full_name()))
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

        format!("{}.{}", self.id, special_extension.unwrap_or("webp"))
    }
}

// TODO: Cleanup
impl PhotoBody {
    pub fn full_name(&self) -> String {
        let folder_path = match self.folder.as_ref() {
            None => String::new(),
            Some(folder) => format!("{folder}/"),
        };

        folder_path + self.name.as_str()
    }
}
