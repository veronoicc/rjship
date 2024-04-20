#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![cfg_attr(not(feature = "std"), no_std)]

use serde::{ser::SerializeStruct, Serialize};

#[derive(Debug, PartialEq)]
pub enum RjShip<
    S,
    F,
    E,
> where S: Serialize, F: std::error::Error, E: std::error::Error {
    Success {
        data: S,
    },
    Fail {
        message: String,
        code: Option<serde_json::Number>,
        data: Option<F>,
    },
    Error {
        message: String,
        code: Option<serde_json::Number>,
        data: Option<E>,
    },
}

// Constructors functions
impl<S, F, E> RjShip<S, F, E> where S: Serialize, F: std::error::Error, E: std::error::Error {
    #[inline]
    pub const fn new(data: S) -> Self {
        Self::Success { 
            data
        }
    }

    #[inline]
    pub const fn new_error(message: String) -> Self {
        Self::Error {
            message,
            code: None,
            data: None,
        }
    }

    #[inline]
    pub const fn new_fail(message: String) -> Self {
        Self::Fail {
            message,
            code: None,
            data: None,
        }
    }
}

// `std` dependant contructor functions
#[cfg(feature = "std")]
impl<S, F, E> RjShip<S, F, E> where S: Serialize, F: std::error::Error, E: std::error::Error {
    #[inline]
    pub fn from_error(data: E) -> Self
    {
        let message = data.to_string();

        Self::Error {
            message,
            code: None,
            data: Some(data),
        }
    }

    #[inline]
    pub fn from_fail(data: F) -> Self
    {
        let message = data.to_string();

        Self::Fail {
            message,
            code: None,
            data: Some(data),
        }
    }
}

// Extractor methods
impl<S, F, E> RjShip<S, F, E> where S: Serialize, F: std::error::Error, E: std::error::Error {
    #[inline]
    pub fn success(self) -> Option<S> {
        match self {
            Self::Success { data } => Some(data),
            _ => None,
        }
    }

    #[inline]
    pub fn fail(self) -> Option<ResultFields<String, F>> {
        match self {
            Self::Fail {
                message,
                code,
                data,
            } => Some(ResultFields {
                message,
                code,
                data,
            }),
            _ => None,
        }
    }

    #[inline]
    pub fn error(self) -> Option<ResultFields<String, E>> {
        match self {
            Self::Error {
                message,
                code,
                data,
            } => Some(ResultFields {
                message,
                code,
                data,
            }),
            _ => None,
        }
    }
}

// Variant evaluation methods
impl<S, F, E> RjShip<S, F, E> where S: Serialize, F: std::error::Error, E: std::error::Error {
    #[inline]
    #[must_use]
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success { .. })
    }

    #[inline]
    #[must_use]
    pub fn is_success_and<Fn: FnOnce(S) -> bool>(self, f: Fn) -> bool {
        match self {
            Self::Success { data } => f(data),
            _ => false,
        }
    }

    #[inline]
    #[must_use]
    pub fn is_fail(&self) -> bool {
        matches!(self, Self::Fail { .. })
    }

    #[inline]
    #[must_use]
    pub fn is_fail_and<Fn: FnOnce(ResultFields<String, F>) -> bool>(self, f: Fn) -> bool {
        match self {
            Self::Fail {
                message,
                code,
                data,
            } => f(ResultFields {
                message,
                code,
                data,
            }),
            _ => false,
        }
    }

    #[inline]
    #[must_use]
    pub fn is_error(&self) -> bool {
        matches!(self, Self::Error { .. })
    }

    #[inline]
    #[must_use]
    pub fn is_error_and<Fn: FnOnce(ResultFields<String, E>) -> bool>(self, f: Fn) -> bool {
        match self {
            Self::Error {
                message,
                code,
                data,
            } => f(ResultFields {
                message,
                code,
                data,
            }),
            _ => false,
        }
    }
}

// Because `ErrorFields` is designed to map to `RJSend::Error`
// as directly as possible, it might be useful to have
// an implementation which maps directly back...
//
// This also means `ErrorFields` can be used as an ad hoc builder
// for the variant as well...
impl<S, F, E> RjShip<S, F, E> where S: Serialize, F: std::error::Error, E: std::error::Error {
    #[inline]
    pub fn from_fail_fields(
        ResultFields {
            message,
            code,
            data,
        }: ResultFields<String, F>,
    ) -> Self {
        Self::Fail {
            message,
            code,
            data,
        }
    }

    #[inline]
    pub fn from_error_fields(
        ResultFields {
            message,
            code,
            data,
        }: ResultFields<String, E>,
    ) -> Self {
        Self::Error {
            message,
            code,
            data,
        }
    }
}

// Derived implementation falls back on some funky old tricks,
// due to the version of Rust `serde` uses,
// which I dislike, and would prefer to streamline.
impl<S, F, E> RjShip<S, F, E> where S: Serialize, F: std::error::Error, E: std::error::Error
{
    fn serialize<Se>(&self, serializer: Se) -> Result<Se::Ok, Se::Error>
    where
        Se: serde::Serializer,
    {
        match self {
            Self::Success { data } => {
                let mut state = serializer.serialize_struct("RJship", 2)?;
                // JSend resresents the different response variants
                // via the `"status"` field, which is why during serialization,
                // we're serializing the name of the variant as a struct field
                // rather than serializing the enum normally...
                state.serialize_field("status", "success")?;
                state.serialize_field("data", data)?;
                state.end()
            }
            // This is the variant this custom implementation
            // pretty much exclusively exists for,
            // because I hate the way `serde` has to handle
            // the `skip_serializing_if` attribute...
            Self::Fail {
                message,
                code,
                data,
            } => {
                // Casting `bool` values as `usize` is kind of a dumb approach,
                // but it's the most concise option in this case...
                let some_count = code.is_some() as usize + data.is_some() as usize;
                let mut state = serializer.serialize_struct("RJship", 2 + some_count)?;

                state.serialize_field("status", "fail")?;
                state.serialize_field("message", message)?;

                match code {
                    // We can extract the contents using patten matching,
                    // rather than serializing the option directly in this case,
                    // because we want to skip serializing
                    // in the case of a `None` value,
                    // not encode the `None` state.
                    Some(code) => state.serialize_field("code", code)?,
                    None => state.skip_field("code")?,
                }

                match data {
                    // Similarly to above, we want to skip serialization
                    // of this field in the case a value is `None`,
                    // rather than encode that state...
                    Some(data) => state.serialize_field("data", &serde_error::Error::new(data))?,
                    None => state.skip_field("data")?,
                }

                state.end()
            }
            // This is the variant this custom implementation
            // pretty much exclusively exists for,
            // because I hate the way `serde` has to handle
            // the `skip_serializing_if` attribute...
            Self::Error {
                message,
                code,
                data,
            } => {
                // Casting `bool` values as `usize` is kind of a dumb approach,
                // but it's the most concise option in this case...
                let some_count = code.is_some() as usize + data.is_some() as usize;
                let mut state = serializer.serialize_struct("RJship", 2 + some_count)?;

                state.serialize_field("status", "error")?;
                state.serialize_field("message", message)?;

                match code {
                    // We can extract the contents using patten matching,
                    // rather than serializing the option directly in this case,
                    // because we want to skip serializing
                    // in the case of a `None` value,
                    // not encode the `None` state.
                    Some(code) => state.serialize_field("code", code)?,
                    None => state.skip_field("code")?,
                }

                match data {
                    // Similarly to above, we want to skip serialization
                    // of this field in the case a value is `None`,
                    // rather than encode that state...
                    Some(data) => state.serialize_field("data", &serde_error::Error::new(data))?,
                    None => state.skip_field("data")?,
                }

                state.end()
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ErrorFields<Msg, D> {
    pub message: Msg,
    pub code: Option<serde_json::Number>,
    pub data: Option<D>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResultFields<Msg, D> {
    pub message: Msg,
    pub code: Option<serde_json::Number>,
    pub data: Option<D>,
}

#[cfg(not(any(feature = "std", feature = "alloc")))]
compile_error!(
    "rjsend requires that either the `std` feature (default) or `alloc` feature is enabled"
);
