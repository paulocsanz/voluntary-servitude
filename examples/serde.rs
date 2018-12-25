#![allow(unused_imports)]

#[macro_use]
extern crate voluntary_servitude;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

use serde_json::{from_str, to_string};
use voluntary_servitude::VS;

#[cfg(feature = "serde-traits")]
#[derive(Serialize, Deserialize, Debug)]
struct User {
    pub username: String,
    pub emails: VS<String>,
}

#[cfg(feature = "serde-traits")]
fn main() {
    let user = User {
        username: "Username".into(),
        emails: vs![
            "username@example.com".into(),
            "alternative-username@example.com".into()
        ],
    };

    let string = to_string(&user).unwrap();
    println!("Serialized: {}", string);
    let user_des: User = from_str(&string).unwrap();
    assert_eq!(user.username, user_des.username);
    assert_eq!(
        user.emails.iter().collect::<Vec<_>>(),
        user_des.emails.iter().collect::<Vec<_>>()
    );

    println!("Serde example ended without errors");
}

#[cfg(not(feature = "serde-traits"))]
fn main() {}
