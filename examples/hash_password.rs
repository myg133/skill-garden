// Quick password hash generator
// Run with: cd D:\MyCodes\Rust\anspire-skillgarden && cargo run --release --example hash_password

use bcrypt::{hash, DEFAULT_COST};

fn main() {
    let password = "admin123";
    match hash(password, DEFAULT_COST) {
        Ok(h) => println!("Hash for '{}': {}", password, h),
        Err(e) => eprintln!("Error: {}", e),
    }
}
