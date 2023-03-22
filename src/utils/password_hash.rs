use base64::prelude::*;
use rand::Rng;
use rand_hc::Hc128Rng;
use sha2::{Digest, Sha512};

pub fn generate_password() -> String {
    let mut rng = Hc128Rng::from_entropy();
    let mut password = String::new();

    for _ in 0..10 {
        let random_char = rng.gen_range(33, 126);
        password.push(random_char as char);
    }

    password
}

pub fn get_hash_from_password(password: &String) -> String {
    const SALT: &str = "cFp&kB^tRdH4";

    let mut hasher = Sha512::new();

    let input = SALT.to_string() + password;
    hasher.update(input.as_bytes());

    let array = hasher.finalize().to_vec();
    BASE64_STANDARD.encode(array)
}
