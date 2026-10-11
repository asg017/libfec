//! Errors as the JS layer expects them: the `code` (napi's error status) says
//! what went wrong, and `js/errors.ts` (`fromNative`) maps it to the public
//! class.
//!
//! | code | meaning |
//! |---|---|
//! | `ENOENT`, `EACCES`, `EISDIR`, `EIO` | opening a path failed (Node-style) |
//! | `FEC_PARSE` | header, cover or row could not be parsed |
//! | `FEC_MISSING_MAPPING` | no column mapping for a row type and version |
//! | `ERR_FILING_CLOSED` | the reader was closed |
//! | `ERR_FILING_CONSUMED` | the reader was already read |

/// Named `Result` so `#[napi]` recognizes it as a fallible return.
pub type Result<T> = std::result::Result<T, napi::Error<String>>;

/// An I/O error opening `path`. The message is the bare OS text; JS builds
/// Node's `"ENOENT: no such file or directory, open '<path>'"` shape.
pub fn io_error(e: std::io::Error, path: &str) -> napi::Error<String> {
    use std::io::ErrorKind;
    let code = match e.kind() {
        ErrorKind::NotFound => "ENOENT",
        ErrorKind::PermissionDenied => "EACCES",
        ErrorKind::IsADirectory => "EISDIR",
        _ => "EIO",
    };
    napi::Error::new(code.to_owned(), format!("{e} ({path})"))
}

/// Anything the parser rejected.
pub fn parse_error(e: &dyn std::fmt::Display) -> napi::Error<String> {
    napi::Error::new("FEC_PARSE".to_owned(), format!("{e:#}"))
}
