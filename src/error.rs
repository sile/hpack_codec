use trackable::error::TrackableError;
use trackable::error::ErrorKind as TrackableErrorKind;

#[derive(Debug, Clone)]
pub struct Error(TrackableError<ErrorKind>);
derive_traits_for_trackable_error_newtype!(Error, ErrorKind);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// Input data is invalid.
    InvalidInput,

    /// I/O error.
    Io,

    /// Other error.
    Other,
}
impl TrackableErrorKind for ErrorKind {}
