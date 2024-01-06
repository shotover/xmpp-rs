// Copyright (c) 2017 Emmanuel Gil Peyrot <linkmauve@linkmauve.fr>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::util::error::Error;
use base64::{engine::general_purpose::STANDARD as Base64Engine, Engine};
use jid::Jid;
use std::str::FromStr;

/// A trait for codecs that can decode and encode text nodes.
pub trait Codec {
    type Decoded;

    /// Decode the given string into the codec’s output.
    fn decode(s: &str) -> Result<Self::Decoded, Error>;

    /// Encode the given value; return None to not produce a text node at all.
    fn encode(decoded: &Self::Decoded) -> Option<String>;
}

/// Codec for text content.
pub struct Text;

impl Codec for Text {
    type Decoded = String;

    fn decode(s: &str) -> Result<String, Error> {
        Ok(s.to_owned())
    }

    fn encode(decoded: &String) -> Option<String> {
        Some(decoded.to_owned())
    }
}

/// Codec transformer that makes the text optional; a "" string is decoded as None.
pub struct OptionalCodec<T: Codec>(std::marker::PhantomData<T>);

impl<T> Codec for OptionalCodec<T>
where
    T: Codec,
{
    type Decoded = Option<T::Decoded>;

    fn decode(s: &str) -> Result<Option<T::Decoded>, Error> {
        if s.is_empty() {
            return Ok(None);
        }

        Ok(Some(T::decode(s)?))
    }

    fn encode(decoded: &Option<T::Decoded>) -> Option<String> {
        decoded.as_ref().and_then(T::encode)
    }
}

/// Codec that trims whitespace around the text.
pub struct Trimmed<T: Codec>(std::marker::PhantomData<T>);

impl<T> Codec for Trimmed<T>
where
    T: Codec,
{
    type Decoded = T::Decoded;

    fn decode(s: &str) -> Result<T::Decoded, Error> {
        match s.trim() {
            // TODO: This error message can be a bit opaque when used
            // in-context; ideally it'd be configurable.
            "" => Err(Error::ParseError(
                "The text in the element's text node was empty after trimming.",
            )),
            trimmed => T::decode(trimmed),
        }
    }

    fn encode(decoded: &T::Decoded) -> Option<String> {
        T::encode(decoded)
    }
}

/// Codec wrapping that encodes/decodes a string as base64.
pub struct Base64;

impl Codec for Base64 {
    type Decoded = Vec<u8>;

    fn decode(s: &str) -> Result<Vec<u8>, Error> {
        Ok(Base64Engine.decode(s)?)
    }

    fn encode(decoded: &Vec<u8>) -> Option<String> {
        Some(Base64Engine.encode(decoded))
    }
}

/// Codec wrapping base64 encode/decode, while ignoring whitespace characters.
pub struct WhitespaceAwareBase64;

impl Codec for WhitespaceAwareBase64 {
    type Decoded = Vec<u8>;

    fn decode(s: &str) -> Result<Self::Decoded, Error> {
        let s: String = s
            .chars()
            .filter(|ch| *ch != ' ' && *ch != '\n' && *ch != '\t')
            .collect();

        Ok(Base64Engine.decode(s)?)
    }

    fn encode(decoded: &Self::Decoded) -> Option<String> {
        Some(Base64Engine.encode(decoded))
    }
}

/// Codec for bytes of lowercase hexadecimal, with a fixed length `N` (in bytes).
pub struct FixedHex<const N: usize>;

impl<const N: usize> Codec for FixedHex<N> {
    type Decoded = [u8; N];

    fn decode(s: &str) -> Result<Self::Decoded, Error> {
        if s.len() != 2 * N {
            return Err(Error::ParseError("Invalid length"));
        }

        let mut bytes = [0u8; N];
        for i in 0..N {
            bytes[i] = u8::from_str_radix(&s[2 * i..2 * i + 2], 16)?;
        }

        Ok(bytes)
    }

    fn encode(decoded: &Self::Decoded) -> Option<String> {
        let mut bytes = String::with_capacity(N * 2);
        for byte in decoded {
            bytes.extend(format!("{:02x}", byte).chars());
        }
        Some(bytes)
    }
}

/// Codec for colon-separated bytes of uppercase hexadecimal.
pub struct ColonSeparatedHex;

impl Codec for ColonSeparatedHex {
    type Decoded = Vec<u8>;

    fn decode(s: &str) -> Result<Self::Decoded, Error> {
        let mut bytes = vec![];
        for i in 0..(1 + s.len()) / 3 {
            let byte = u8::from_str_radix(&s[3 * i..3 * i + 2], 16)?;
            if 3 * i + 2 < s.len() {
                assert_eq!(&s[3 * i + 2..3 * i + 3], ":");
            }
            bytes.push(byte);
        }
        Ok(bytes)
    }

    fn encode(decoded: &Self::Decoded) -> Option<String> {
        let mut bytes = vec![];
        for byte in decoded {
            bytes.push(format!("{:02X}", byte));
        }
        Some(bytes.join(":"))
    }
}

/// Codec for a JID.
pub struct JidCodec;

impl Codec for JidCodec {
    type Decoded = Jid;

    fn decode(s: &str) -> Result<Jid, Error> {
        Ok(Jid::from_str(s)?)
    }

    fn encode(jid: &Jid) -> Option<String> {
        Some(jid.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_hex() {
        let value = [0x01, 0xfe, 0xef];

        // Test that we support both lowercase and uppercase as input.
        let hex = FixedHex::<3>::decode("01feEF").unwrap();
        assert_eq!(&hex, &value);

        // Test that we do output lowercase.
        let hex = FixedHex::<3>::encode(&value).unwrap();
        assert_eq!(hex, "01feef");

        // What if we give it a string that's too long?
        let err = FixedHex::<3>::decode("01feEF01").unwrap_err();
        assert_eq!(err.to_string(), "parse error: Invalid length");

        // Too short?
        let err = FixedHex::<3>::decode("01fe").unwrap_err();
        assert_eq!(err.to_string(), "parse error: Invalid length");

        // Not-even numbers?
        let err = FixedHex::<3>::decode("01feE").unwrap_err();
        assert_eq!(err.to_string(), "parse error: Invalid length");

        // No colon supported.
        let err = FixedHex::<3>::decode("0:f:EF").unwrap_err();
        assert_eq!(
            err.to_string(),
            "integer parsing error: invalid digit found in string"
        );

        // No non-hex character allowed.
        let err = FixedHex::<3>::decode("01defg").unwrap_err();
        assert_eq!(
            err.to_string(),
            "integer parsing error: invalid digit found in string"
        );
    }
}
