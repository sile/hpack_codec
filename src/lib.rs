//! Encoder and Decoder for [HPACK (RFC 7541)][HPACK].
//!
//! [HPACk]: https://tools.ietf.org/html/rfc7541
//!
//! # Examples
//!
//! ```
//! use hpack_codec::{Encoder, Decoder};
//! use hpack_codec::field::{HeaderField, LiteralHeaderField as Field};
//! use hpack_codec::table::{StaticEntry, Index};
//!
//! // Encoding
//! let mut encoder = Encoder::new(4096);
//! let mut header = encoder.enter_header_block(Vec::new()).unwrap();
//! header.encode_field(StaticEntry::MethodGet).unwrap();
//! header.encode_field(Field::with_indexed_name(StaticEntry::Path, b"/hello")).unwrap();
//! header.encode_field(Field::new(b"foo", b"bar").with_indexing()).unwrap();
//! header.encode_field(Index::dynamic_table_offset() + 0).unwrap();
//! let encoded_data = header.finish();
//!
//! // Decoding
//! let mut decoder = Decoder::new(4096);
//! let mut header = decoder.enter_header_block(&encoded_data[..]).unwrap();
//! assert_eq!(header.decode_field().unwrap(), HeaderField::new(b":method", b"GET").ok());
//! assert_eq!(header.decode_field().unwrap(), HeaderField::new(b":path", b"/hello").ok());
//! assert_eq!(header.decode_field().unwrap(), HeaderField::new(b"foo", b"bar").ok());
//! assert_eq!(header.decode_field().unwrap(), HeaderField::new(b"foo", b"bar").ok());
//! ```
#![warn(missing_docs)]
#[macro_use]
extern crate trackable;

macro_rules! track_io {
    ($e:expr) => {
        $e.map_err(|e| {
            use trackable::error::{ErrorKindExt, Failed};
            Failed.cause(e)
        })
    };
}

pub use decoder::{Decoder, HeaderBlockDecoder};
pub use encoder::{Encoder, HeaderBlockEncoder};

pub mod field;
pub mod literal;
pub mod table;

mod decoder;
mod encoder;
mod huffman;
mod io;
mod signal;

/// This crate specific `Error` type.
pub type Error = trackable::error::TrackableError<trackable::error::Failed>;

/// This crate specific `Result` type.
pub type Result<T, E = Error> = std::result::Result<T, E>;
