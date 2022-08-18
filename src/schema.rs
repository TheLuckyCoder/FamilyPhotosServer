table! {
    photos (id) {
        id -> Int8,
        owner -> Int8,
        name -> Text,
        time_created -> Timestamp,
        file_size -> Int8,
        folder -> Nullable<Text>,
    }
}

table! {
    users (id) {
        id -> Int8,
        display_name -> Text,
        user_name -> Text,
        password -> Text,
    }
}

allow_tables_to_appear_in_same_query!(
    photos,
    users,
);
