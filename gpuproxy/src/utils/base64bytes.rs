use serde::{Deserialize, Serialize};
use serde::{Deserializer, Serializer};

/// compatibility with golang's data method， used to receive or send byte slice data
#[derive(Debug)]
pub struct Base64Byte(pub Vec<u8>);

impl Base64Byte {
    pub fn new(data: Vec<u8>) -> Self {
        Base64Byte(data)
    }
}

impl From<Base64Byte> for Vec<u8> {
    fn from(v: Base64Byte) -> Vec<u8> {
        v.0
    }
}

impl Serialize for Base64Byte {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(base64::encode(&self.0).as_str())
    }
}

impl<'de> Deserialize<'de> for Base64Byte {
    fn deserialize<D>(deserializer: D) -> Result<Base64Byte, D::Error>
    where
        D: Deserializer<'de>,
    {
        let bytes_str = <String>::deserialize(deserializer)?;
        Ok(Base64Byte(base64::decode(bytes_str).unwrap()))
    }
}
