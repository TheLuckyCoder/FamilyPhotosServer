use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use rand::{Rng, SeedableRng};
use rand_hc::Hc128Rng;

pub fn generate_random_password() -> String {
    let mut rng = Hc128Rng::from_entropy();
    let mut password = String::new();
    password.reserve(14);

    for _ in 0..14 {
        let random_char: u8 = rng.gen_range(33..=126);
        password.push(random_char as char);
    }

    password
}

pub fn generate_hash_from_password<T: AsRef<str>>(password: T) -> String {
    let salt = SaltString::generate(&mut rand::thread_rng());

    return Argon2::default()
        .hash_password(password.as_ref().as_bytes(), &salt)
        .expect("Failed to hash password")
        .to_string();
}

pub fn validate_credentials(
    password: &String,
    expected_password_hash: &String,
) -> Result<bool, String> {
    let expected_password_hash =
        PasswordHash::new(expected_password_hash).map_err(|e| e.to_string())?;

    return Ok(Argon2::default()
        .verify_password(password.as_bytes(), &expected_password_hash)
        .is_ok());
}
