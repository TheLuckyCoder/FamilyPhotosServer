use std::str::from_utf8;
use sha2::{Digest, Sha512};

pub fn get_hash_from_password(password: &String) -> String {
    const SALT: &str = "cFp&kB^tRdH4";

    let mut hasher = Sha512::new();

    let input = SALT.to_string() + password;
    hasher.update(input.as_bytes());

    let array = hasher.finalize().to_vec();
    from_utf8(&*array).unwrap().to_string()
}