extern crate byteorder;
#[macro_use]
extern crate trackable;

pub use encoder::{Encoder, HeaderBlockEncoder};
pub use name::Name;

macro_rules! track_io {
    ($e:expr) => {
        $e.map_err(|e| {
            use ::trackable::error::{Failed,ErrorKindExt};
            Failed.cause(e)
        })
    }
}

pub mod table;

pub mod decoder; // TODO: private
mod encoder;
pub mod huffman; // TODO: private
pub mod field; // TODO: private
pub mod literal; // TODO: private
mod name;

pub type Error = trackable::error::TrackableError<trackable::error::Failed>;
pub type Result<T> = ::std::result::Result<T, Error>;
