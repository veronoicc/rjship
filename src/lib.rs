#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![cfg_attr(not(feature = "std"), no_std)]

use core::fmt;
#[cfg(feature = "std")]
use std::error::Error;

use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "status")]
#[serde(rename_all = "lowercase")]
pub enum RJSend<D, FD, Msg = &'static str, ED = serde_json::Value> {
    Success {
        data: D,
    },
    Fail {
        data: FD,
    },
    Error {
        #[serde(bound(deserialize = "Msg: fmt::Display + Deserialize<'de>",))]
        message: Msg,
        code: Option<serde_json::Number>,
        data: Option<ED>,
    },
}

// Constructors functions
impl<D, FD, Msg, ED> RJSend<D, FD, Msg, ED> {
    #[inline]
    pub const fn new_error(message: Msg) -> Self {
        Self::Error {
            message,
            code: None,
            data: None,
        }
    }

    #[inline]
    pub fn from_error_fields(
        ErrorFields {
            message,
            code,
            data,
        }: ErrorFields<Msg, ED>,
    ) -> Self {
        Self::Error {
            message,
            code,
            data,
        }
    }
}

// `std` dependant contructor functions
#[cfg(feature = "std")]
impl<D, FD, ED> RJSend<D, FD, String, ED> {
    #[inline]
    pub fn from_error(data: ED) -> Self
    where
        ED: Error,
    {
        let message = data.to_string();

        Self::Error {
            message,
            code: None,
            data: Some(data),
        }
    }
}

// Because `ErrorFields` is designed to map to `RJSend::Error`
// as directly as possible, it might be useful to have
// an implementation which maps directly back...
//
// This also means `ErrorFields` can be used as an ad hoc builder
// for the variant as well...
impl<D, FD, Msg, ED> From<ErrorFields<Msg, ED>> for RJSend<D, FD, Msg, ED> {
    fn from(fields: ErrorFields<Msg, ED>) -> Self {
        Self::from_error_fields(fields)
    }
}

#[cfg(feature = "std")]
impl<D, FD, ED> From<ED> for RJSend<D, FD, String, ED>
where
    ED: Error,
{
    fn from(data: ED) -> Self {
        Self::from_error(data)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ErrorFields<Msg, ED> {
    pub message: Msg,
    pub code: Option<serde_json::Number>,
    pub data: Option<ED>,
}

#[cfg(not(any(feature = "std", feature = "alloc")))]
compile_error!(
    "rjsend requires that either the `std` feature (default) or `alloc` feature is enabled"
);
