use base64ct::{Base64UrlUnpadded, Encoding};
use sha1::{Digest, Sha1};

pub fn hash(s: &String) -> String {
    let mut key = Base64UrlUnpadded::encode_string(&Sha1::digest(s));
    key.truncate(16);
    key
}
