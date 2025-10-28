use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;

/// A Universally Unique Identifier (UUID) represented as 16 bytes.
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct Uuid([u8; 16]);

/// Error type returned when parsing a UUID string.
#[derive(Debug, PartialEq, Eq)]
pub enum UuidError {
    /// The input does not have the correct length.
    Length,
    /// The input contains an invalid hexadecimal character.
    Character,
    /// The version bits are not set correctly for a v4 UUID.
    Version,
    /// The variant bits are not set correctly (expected RFC4122 variant).
    Variant,
}

impl fmt::Display for UuidError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UuidError::Length => write!(f, "invalid length for a UUID"),
            UuidError::Character => write!(f, "invalid character in UUID"),
            UuidError::Version => write!(f, "invalid UUID version (expected v4)"),
            UuidError::Variant => write!(f, "invalid UUID variant (expected RFC4122)"),
        }
    }
}

impl std::error::Error for UuidError {}

impl Uuid {
    /// Generates a new random (version 4) UUID.
    ///
    /// This method fills 16 bytes with random data from fastrand,
    /// then sets the version (4) and variant (RFC4122) bits as required.
    pub fn new_v4() -> Self {
        let mut bytes = [0u8; 16];
        fastrand::fill(&mut bytes);
        // Set version (bits 4-7 of byte 6) to 4.
        bytes[6] = (bytes[6] & 0x0f) | 0x40;
        // Set variant (bits 6-7 of byte 8) to 10.
        bytes[8] = (bytes[8] & 0x3f) | 0x80;
        Uuid(bytes)
    }

    /// Returns a reference to the inner 16-byte array.
    #[allow(dead_code)]
    pub fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }

    pub const _NIL: Self = Uuid([0; 16]);

    /// Encodes the UUID into a fixed 36-byte hyphenated lowercase representation.
    /// This function is allocation-free and designed for efficient serialization.
    #[inline]
    fn encode_hyphenated_lower(&self) -> [u8; 36] {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let b = self.0;
        let mut out = [0u8; 36];

        #[inline(always)]
        fn put(out: &mut [u8], i: usize, byte: u8) {
            out[i] = HEX[(byte >> 4) as usize];
            out[i + 1] = HEX[(byte & 0x0f) as usize];
        }

        let mut i = 0usize;
        for (idx, &byte) in b.iter().enumerate() {
            if idx == 4 || idx == 6 || idx == 8 || idx == 10 {
                out[i] = b'-';
                i += 1;
            }
            put(&mut out, i, byte);
            i += 2;
        }

        out
    }
}

/// Formats the UUID in the standard hyphenated lowercase form.
///
/// The output format is 8-4-4-4-12 hexadecimal digits.
impl fmt::Display for Uuid {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let b = self.0;
        let d1 = u32::from_be_bytes([b[0], b[1], b[2], b[3]]);
        let d2 = u16::from_be_bytes([b[4], b[5]]);
        let d3 = u16::from_be_bytes([b[6], b[7]]);
        let d4 = u16::from_be_bytes([b[8], b[9]]);
        let d5 = ((b[10] as u64) << 40)
            | ((b[11] as u64) << 32)
            | ((b[12] as u64) << 24)
            | ((b[13] as u64) << 16)
            | ((b[14] as u64) << 8)
            | (b[15] as u64);
        write!(f, "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}", d1, d2, d3, d4, d5)
    }
}

/// Debug prints the UUID as `Uuid(<display>)`.
impl fmt::Debug for Uuid {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Uuid({})", self)
    }
}

/// Parsing support for a UUID string.
///
/// This implementation accepts strings in several forms:
///
/// - A simple string of 32 hexadecimal digits
/// - A hyphenated string (e.g. "67e55044-10b1-426f-9247-bb680e5fe0c8")
/// - With optional braces `{...}` or the prefix `"urn:uuid:"` (case-insensitive)
///
/// After removing these optional parts and any hyphens, the remaining string
/// must be exactly 32 hexadecimal digits. Then the code parses two characters
/// at a time into a byte. Finally, it validates that the version and variant bits
/// are as required for a version 4 UUID.
impl FromStr for Uuid {
    type Err = UuidError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        let s = if s.to_lowercase().starts_with("urn:uuid:") {
            &s[9..]
        } else {
            s
        };

        let s = if s.starts_with('{') && s.ends_with('}') {
            &s[1..s.len() - 1]
        } else {
            s
        };

        let s: String = s.replace("-", "");

        if s.len() != 32 {
            return Err(UuidError::Length);
        }

        let mut bytes = [0u8; 16];
        for i in 0..16 {
            let hex_byte = &s[i * 2..i * 2 + 2];
            bytes[i] = u8::from_str_radix(hex_byte, 16).map_err(|_| UuidError::Character)?;
        }

        // Validate that the version is 4.
        if (bytes[6] & 0xF0) != 0x40 {
            return Err(UuidError::Version);
        }

        // Validate that the variant is RFC4122 (the two most significant bits of byte 8 must be 10).
        if (bytes[8] & 0xC0) != 0x80 {
            return Err(UuidError::Variant);
        }

        Ok(Uuid(bytes))
    }
}

/// Allows creation of a `Uuid` directly from a 16-byte array.
impl From<[u8; 16]> for Uuid {
    fn from(bytes: [u8; 16]) -> Self {
        Uuid(bytes)
    }
}

/// Serialize the UUID as a hyphenated lowercase string without any heap allocation.
impl Serialize for Uuid {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let buf = self.encode_hyphenated_lower();
        // SAFETY: only ASCII hex digits and '-' are written, all valid UTF-8.
        let s = unsafe { std::str::from_utf8_unchecked(&buf) };
        serializer.serialize_str(s)
    }
}

/// Deserialize a UUID from a string in the standard hyphenated format (or one
/// of the acceptable alternate formats).
impl<'de> Deserialize<'de> for Uuid {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct UuidVisitor;

        impl serde::de::Visitor<'_> for UuidVisitor {
            type Value = Uuid;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a UUID in a valid format")
            }

            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Uuid, E> {
                v.parse::<Uuid>().map_err(E::custom)
            }
        }

        deserializer.deserialize_str(UuidVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_v4_sets_version_and_variant() {
        let uuid = Uuid::new_v4();
        let bytes = uuid.as_bytes();
        // Check version (should be 4).
        assert_eq!(bytes[6] >> 4, 0x4, "Version should be 4");
        // Check variant (should be 0b10 in the two highest bits).
        assert_eq!(bytes[8] >> 6, 0b10, "Variant should be RFC4122");
    }

    #[test]
    fn test_display_format() {
        let uuid = Uuid([
            0x67, 0xe5, 0x50, 0x44, 0x10, 0xb1, 0x42, 0x6f, 0x92, 0x47, 0xbb, 0x68, 0xe5, 0xfe,
            0x0c, 0x8a,
        ]);
        let s = uuid.to_string();
        // The hyphens should be in positions 8, 13, 18, and 23.
        assert_eq!(s.len(), 36);
        assert_eq!(s.chars().nth(8), Some('-'));
        assert_eq!(s.chars().nth(13), Some('-'));
        assert_eq!(s.chars().nth(18), Some('-'));
        assert_eq!(s.chars().nth(23), Some('-'));
    }

    #[test]
    fn test_encode_hyphenated_lower_matches_display() {
        let uuid = Uuid::new_v4();
        let buf = uuid.encode_hyphenated_lower();
        let encoded = std::str::from_utf8(&buf).unwrap();
        let display_str = uuid.to_string();
        assert_eq!(
            encoded, display_str,
            "encode_hyphenated_lower() should match Display output"
        );
    }

    #[test]
    fn test_from_str_valid_formats() {
        let original = Uuid::new_v4();
        let s = original.to_string();

        // Basic hyphenated form.
        let parsed = s.parse::<Uuid>().expect("Parsing should succeed");
        assert_eq!(original, parsed);

        // With braces.
        let s_braced = format!("{{{}}}", s);
        let parsed_braced = s_braced
            .parse::<Uuid>()
            .expect("Parsing with braces should succeed");
        assert_eq!(original, parsed_braced);

        // With URN prefix.
        let s_urn = format!("urn:uuid:{}", s);
        let parsed_urn = s_urn
            .parse::<Uuid>()
            .expect("Parsing with URN prefix should succeed");
        assert_eq!(original, parsed_urn);
    }

    #[test]
    fn test_from_str_invalid_length() {
        let s = "12345678";
        let err = s.parse::<Uuid>().unwrap_err();
        assert_eq!(err, UuidError::Length);
    }

    #[test]
    fn test_from_str_invalid_character() {
        // Replace a valid character with an invalid one.
        let valid = "67e55044-10b1-426f-9247-bb680e5fe0c8";
        let s = valid.replace("e", "G"); // 'G' is not a valid hex character.
        let err = s.parse::<Uuid>().unwrap_err();
        assert_eq!(err, UuidError::Character);
    }

    #[test]
    fn test_from_str_invalid_version() {
        let uuid = Uuid::new_v4();
        let mut bytes = *uuid.as_bytes();
        // Change version from 4 to 5.
        bytes[6] = (bytes[6] & 0x0f) | 0x50;
        let s = Uuid(bytes).to_string();
        let err = s.parse::<Uuid>().unwrap_err();
        assert_eq!(err, UuidError::Version);
    }

    #[test]
    fn test_from_str_invalid_variant() {
        let uuid = Uuid::new_v4();
        let mut bytes = *uuid.as_bytes();
        // Change the variant bits to an invalid setting.
        bytes[8] &= 0x3f;
        let s = Uuid(bytes).to_string();
        let err = s.parse::<Uuid>().unwrap_err();
        assert_eq!(err, UuidError::Variant);
    }

    #[test]
    fn test_serde_roundtrip() {
        let uuid = Uuid::new_v4();
        // Serialize to JSON.
        let json = serde_json::to_string(&uuid).expect("Serialization should succeed");
        // Deserialize back.
        let parsed: Uuid = serde_json::from_str(&json).expect("Deserialization should succeed");
        assert_eq!(uuid, parsed);
    }

    #[test]
    fn test_uuid_error_display() {
        // Test display formatting for each variant of UuidError
        assert_eq!(UuidError::Length.to_string(), "invalid length for a UUID");
        assert_eq!(
            UuidError::Character.to_string(),
            "invalid character in UUID"
        );
        assert_eq!(
            UuidError::Version.to_string(),
            "invalid UUID version (expected v4)"
        );
        assert_eq!(
            UuidError::Variant.to_string(),
            "invalid UUID variant (expected RFC4122)"
        );
    }

    #[test]
    fn test_uuid_debug_format() {
        let uuid = Uuid::new_v4();
        let debug_str = format!("{:?}", uuid);
        let display_str = uuid.to_string();
        assert_eq!(debug_str, format!("Uuid({})", display_str));
    }

    #[test]
    fn test_uuid_from_bytes() {
        let bytes = [
            0x67, 0xe5, 0x50, 0x44, 0x10, 0xb1, 0x42, 0x6f, 0x92, 0x47, 0xbb, 0x68, 0xe5, 0xfe,
            0x0c, 0x8a,
        ];
        let uuid = Uuid::from(bytes);
        assert_eq!(uuid.as_bytes(), &bytes);
    }

    #[test]
    fn test_serde_deserialization_errors() {
        // Test various invalid inputs that should trigger the visitor

        // Test with a number instead of string
        let result: Result<Uuid, _> = serde_json::from_str("42");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("invalid")); // Should contain error about invalid format

        // Test with null instead of string
        let result: Result<Uuid, _> = serde_json::from_str("null");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("invalid"));

        // Test with malformed UUID string
        let result: Result<Uuid, _> = serde_json::from_str("\"not-a-uuid\"");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("length")); // Should mention invalid length

        // Test with empty string
        let result: Result<Uuid, _> = serde_json::from_str("\"\"");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("length"));

        // Test with array instead of string
        let result: Result<Uuid, _> = serde_json::from_str("[1,2,3]");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("invalid"));
    }
}
