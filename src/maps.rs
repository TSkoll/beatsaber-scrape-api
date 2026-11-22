use std::fmt::Write as _;

include!(concat!(env!("OUT_DIR"), "/maps.rs"));

impl SongProto {
    /// `hash_bytes` encoded as an uppercase hexadecimal string.
    pub fn hash(&self) -> String {
        let mut hash = String::with_capacity(self.hash_bytes.len() * 2);
        for byte in &self.hash_bytes {
            write!(hash, "{byte:02X}").expect("writing to a string cannot fail");
        }
        hash
    }
}

#[cfg(test)]
mod tests {
    use super::SongProto;

    #[test]
    fn hash_is_uppercase_hex() {
        let song = SongProto {
            hash_bytes: vec![0x0a, 0xbc, 0xde, 0xf1],
            ..Default::default()
        };

        assert_eq!(song.hash(), "0ABCDEF1");
    }
}
