extern crate byteorder;
#[macro_use]
extern crate trackable;

macro_rules! track_io {
    ($e:expr) => {
        $e.map_err(|e| {
            use ::trackable::error::{Failed,ErrorKindExt};
            Failed.cause(e)
        })
    }
}

pub use decoder::{Decoder, HeaderBlockDecoder};
pub use encoder::{Encoder, HeaderBlockEncoder};

pub mod field;
pub mod table;

mod decoder;
mod encoder;
mod huffman;
mod io;
pub mod literal; // TODO: private
mod signal;

pub type Error = trackable::error::TrackableError<trackable::error::Failed>;
pub type Result<T> = ::std::result::Result<T, Error>;
