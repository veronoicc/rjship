#![feature(min_specialization)] 
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![cfg_attr(not(feature = "std"), no_std)]

use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};

pub enum RjShip<D, C, FD = (), ED = ()> where D: Serialize, C: num_traits::PrimInt + Serialize, FD: Serialize, ED: Serialize {
    Success {
        data: D,
    },
    Fail {
        message: String,
        code: Option<C>,
        data: Option<FD>,
    },
    Error {
        message: String,
        code: Option<C>,
        data: Option<ED>,
    },
}

// Constructors functions
impl<D, C, FD, ED> RjShip<D, C, FD, ED> where D: Serialize, C: num_traits::PrimInt + Serialize, FD: Serialize, ED: Serialize  {
    #[inline]
    pub const fn new(data: D) -> Self {
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
impl<D, C, FD, ED> RjShip<D, C, FD, ED> where D: Serialize, C: num_traits::PrimInt + Serialize, FD: Serialize, ED: Serialize {
    #[inline]
    pub fn from_fail<F: ToString>(message: F, code: Option<C>, data: Option<FD>) -> Self
    {
        Self::Fail {
            message: message.to_string(),
            code,
            data: data,
        }
    }

    #[inline]
    pub fn from_error<E: ToString>(message: E, code: Option<C>, data: Option<ED>) -> Self
    {
        Self::Error {
            message: message.to_string(),
            code,
            data: data,
        }
    }
}

// Extractor methods
impl<D, C, FD, ED> RjShip<D, C, FD, ED> where D: Serialize, C: num_traits::PrimInt + Serialize, FD: Serialize, ED: Serialize {
    #[inline]
    pub fn success(self) -> Option<D> {
        match self {
            Self::Success { data } => Some(data),
            _ => None,
        }
    }

    #[inline]
    pub fn fail(self) -> Option<ResultFields<String, C, FD>> {
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
    pub fn error(self) -> Option<ResultFields<String, C, ED>> {
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
impl<D, C, FD, ED> RjShip<D, C, FD, ED> where D: Serialize, C: num_traits::PrimInt + Serialize, FD: Serialize, ED: Serialize {
    #[inline]
    #[must_use]
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success { .. })
    }

    #[inline]
    #[must_use]
    pub fn is_success_and<F: FnOnce(D) -> bool>(self, f: F) -> bool {
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
    pub fn is_fail_and<Fn: FnOnce(ResultFields<String, C, FD>) -> bool>(self, f: Fn) -> bool {
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
    pub fn is_error_and<F: FnOnce(ResultFields<String, C, ED>) -> bool>(self, f: F) -> bool {
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
impl<D, C, FD, ED> RjShip<D, C, FD, ED> where D: Serialize, C: num_traits::PrimInt + Serialize, FD: Serialize, ED: Serialize {
    #[inline]
    pub fn from_fail_fields(
        ResultFields {
            message,
            code,
            data,
        }: ResultFields<String, C, FD>,
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
        }: ResultFields<String, C, ED>,
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
impl<D, C, FD, ED> Serialize for RjShip<D, C, FD, ED> where D: Serialize, C: num_traits::PrimInt + Serialize, FD: Serialize, ED: Serialize
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer
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
                    Some(data) => state.serialize_field("data", data)?,
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
                    Some(data) => state.serialize_field("data", data)?,
                    None => state.skip_field("data")?,
                }

                state.end()
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResultFields<Msg, C, D> {
    pub message: Msg,
    pub code: Option<C>,
    pub data: Option<D>,
}

#[cfg(not(any(feature = "std", feature = "alloc")))]
compile_error!(
    "rjsend requires that either the `std` feature (default) or `alloc` feature is enabled"
);
