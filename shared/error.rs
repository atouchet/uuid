use crate::std::fmt;

/// A general error that can occur when working with UUIDs.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Error(pub(crate) ErrorKind);

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) enum ErrorKind {
    /// Invalid character in the [`Uuid`] string.
    ///
    /// [`Uuid`]: ../struct.Uuid.html
    Char { character: char, index: usize },
    /// A simple [`Uuid`] didn't contain 32 characters.
    ///
    /// [`Uuid`]: ../struct.Uuid.html
    SimpleLength { len: usize },
    /// A hyphenated [`Uuid`] didn't contain 5 groups
    ///
    /// [`Uuid`]: ../struct.Uuid.html
    GroupCount { count: usize },
    /// A hyphenated [`Uuid`] had a group that wasn't the right length
    ///
    /// [`Uuid`]: ../struct.Uuid.html
    GroupLength {
        group: usize,
        len: usize,
        index: usize,
    },
}

/// A string that is guaranteed to fail to parse to a [`Uuid`].
///
/// This type acts as a lightweight error indicator, suggesting
/// that the string cannot be parsed but offering no error
/// details. To get details, use `InvalidUuid::into_err`.
///
/// [`Uuid`]: ../struct.Uuid.html
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct InvalidUuid<'a>(pub(crate) &'a str);

impl<'a> InvalidUuid<'a> {
    /// Converts the lightweight error type into detailed diagnostics.
    pub fn into_err(self) -> Error {
        let (s, offset, simple) = match self.0.as_bytes() {
            [b'{', s @ .., b'}'] => (s, 1, false),
            [b'u', b'r', b'n', b':', b'u', b'u', b'i', b'd', b':', s @ ..] => {
                (s, "urn:uuid:".len(), false)
            }
            s => (s, 0, true),
        };

        let mut hyphen_count = 0;
        let mut group_bounds = [0; 4];
        // SAFETY: the byte array came from a valid utf8 string,
        // and is aligned along char boundries.
        let string = unsafe { std::str::from_utf8_unchecked(s) };
        for (index, character) in string.char_indices() {
            let byte = character as u8;
            if character as u32 - byte as u32 > 0 {
                // Multibyte char
                return Error(ErrorKind::Char {
                    character,
                    index: index + offset + 1,
                });
            } else if byte == b'-' {
                // While we search, also count group breaks
                if hyphen_count < 4 {
                    group_bounds[hyphen_count] = index;
                }
                hyphen_count += 1;
            } else if !matches!(byte, b'0'..=b'9' | b'a'..=b'f' | b'A'..=b'F') {
                // Non-hex char
                return Error(ErrorKind::Char {
                    character: byte as char,
                    index: index + offset + 1,
                });
            }
        }

        if hyphen_count == 0 && simple {
            // This means that we tried and failed to parse a simple uuid.
            // Since we verified that all the characters are valid, this means
            // that it MUST have an invalid length.
            Error(ErrorKind::SimpleLength { len: s.len() })
        } else if hyphen_count != 4 {
            // We tried to parse a hyphenated variant, but there weren't
            // 5 groups (4 hyphen splits).
            Error(ErrorKind::GroupCount {
                count: hyphen_count + 1,
            })
        } else {
            // There are 5 groups, one of them has an incorrect length
            const BLOCK_STARTS: [usize; 5] = [0, 9, 14, 19, 24];
            for i in 0..4 {
                if group_bounds[i] != BLOCK_STARTS[i + 1] - 1 {
                    return Error(ErrorKind::GroupLength {
                        group: i,
                        len: group_bounds[i] - BLOCK_STARTS[i],
                        index: offset + BLOCK_STARTS[i] + 1,
                    });
                }
            }

            // The last group must be too long
            Error(ErrorKind::GroupLength {
                group: 4,
                len: s.len() - BLOCK_STARTS[4],
                index: offset + BLOCK_STARTS[4] + 1,
            })
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0 {
            ErrorKind::Char {
                character, index, ..
            } => {
                write!(f, "invalid character: expected an optional prefix of `urn:uuid:` followed by [0-9a-zA-Z], found `{}` at {}", character, index)
            }
            ErrorKind::SimpleLength { len } => {
                write!(
                    f,
                    "invalid length: expected length 32 for simple format, found {}",
                    len
                )
            }
            ErrorKind::GroupCount { count } => {
                write!(f, "invalid group count: expected 5, found {}", count)
            }
            ErrorKind::GroupLength { group, len, .. } => {
                let expected = [8, 4, 4, 4, 12][group];
                write!(
                    f,
                    "invalid group length in group {}: expected {}, found {}",
                    group, expected, len
                )
            }
        }
    }
}

#[cfg(feature = "std")]
mod std_support {
    use super::*;
    use crate::std::error;

    impl error::Error for Error {}
}
