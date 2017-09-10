extern crate byteorder;
#[macro_use]
extern crate trackable;

use std::collections::VecDeque;

pub use error::{Error, ErrorKind};

mod error;
pub mod literal; // TODO: private

pub type Result<T> = ::std::result::Result<T, Error>;

#[derive(Debug)]
pub struct Context {
    dynamic_table: DynamicTable,
}

/// https://tools.ietf.org/html/rfc7540#section-6.5.2
pub const DEFAULT_MAX_TABLE_SIZE: usize = 4096;

#[derive(Debug)]
pub struct DynamicTable {
    entries: VecDeque<HeaderField<Vec<u8>>>,
    max_table_size: usize,
}
impl DynamicTable {
    /// https://tools.ietf.org/html/rfc7541#section-4.1
    pub fn table_size(&self) -> usize {
        self.entries
            .iter()
            .map(|h| h.name.len() + h.value.len() + 32)
            .sum()
    }
    pub fn set_max_table_size(&mut self, size: usize) {
        // TODO: https://tools.ietf.org/html/rfc7541#section-4.3
        self.max_table_size = size;
    }
}

#[derive(Debug)]
pub struct Encoder {
    context: Context,
}

#[derive(Debug)]
pub struct Decoder {
    context: Context,
}

#[derive(Debug)]
pub struct HeaderField<B> {
    pub name: B,
    pub value: B,
}

macro_rules! field {
    ($name:expr, $value: expr) => {
        HeaderField{ name: $name, value: $value }
    };
    ($name:expr) => {
        HeaderField{ name: $name, value: b"" }
    }
}

pub const STATIC_TABLE: &[HeaderField<&[u8]>; 61] = &[
    field!(b":authority"),
    field!(b":method", b"GET"),
    field!(b":method", b"POST"),
    field!(b":path", b"/"),
    field!(b":path", b"/index.html"),
    field!(b":scheme", b"http"),
    field!(b":scheme", b"https"),
    field!(b":status", b"200"),
    field!(b":status", b"204"),
    field!(b":status", b"206"),
    field!(b":status", b"304"),
    field!(b":status", b"400"),
    field!(b":status", b"404"),
    field!(b":status", b"500"),
    field!(b"accept-charset"),
    field!(b"accept-encoding", b"gzip, deflate"),
    field!(b"accept-language"),
    field!(b"accept-ranges"),
    field!(b"accept"),
    field!(b"access-control-allow-origin"),
    field!(b"age"),
    field!(b"allow"),
    field!(b"authorization"),
    field!(b"cache-control"),
    field!(b"content-disposition"),
    field!(b"content-encoding"),
    field!(b"content-language"),
    field!(b"content-length"),
    field!(b"content-location"),
    field!(b"content-range"),
    field!(b"content-type"),
    field!(b"cookie"),
    field!(b"date"),
    field!(b"etag"),
    field!(b"expect"),
    field!(b"expires"),
    field!(b"from"),
    field!(b"host"),
    field!(b"if-match"),
    field!(b"if-modified-since"),
    field!(b"if-none-match"),
    field!(b"if-range"),
    field!(b"if-unmodified-since"),
    field!(b"last-modified"),
    field!(b"link"),
    field!(b"location"),
    field!(b"max-forwards"),
    field!(b"proxy-authenticate"),
    field!(b"proxy-authorization"),
    field!(b"range"),
    field!(b"referer"),
    field!(b"refresh"),
    field!(b"retry-after"),
    field!(b"server"),
    field!(b"set-cookie"),
    field!(b"strict-transport-security"),
    field!(b"transfer-encoding"),
    field!(b"user-agent"),
    field!(b"vary"),
    field!(b"via"),
    field!(b"www-authenticate"),
];

/// https://tools.ietf.org/html/rfc7541#section-5.1
#[derive(Debug)]
pub struct IntegerOctets {
    prefix_bits: u8,
    octets: [u8; 10],
    octets_len: u8,
}
impl IntegerOctets {
    pub fn from_u64(prefix_bits: u8, value: u64) -> Result<Self> {
        track_assert!(
            1 <= prefix_bits && prefix_bits <= 8,
            ErrorKind::InvalidInput,
            "prefix_bits={}, value={}",
            prefix_bits,
            value
        );

        let prefix_value = (2 << prefix_bits) - 1;
        let mut octets = [0; 10];
        let octets_len = if value < prefix_value {
            octets[0] = value as u8;
            1
        } else {
            octets[0] = prefix_value as u8;
            let mut value = value - prefix_value;
            let mut i = 1;
            while value >= 128 {
                octets[i] = (value % 128 + 128) as u8;
                value /= 128;
                i += 1;
            }
            octets[i] = value as u8;
            i + 1
        } as u8;
        Ok(IntegerOctets {
            prefix_bits,
            octets,
            octets_len,
        })
    }
    pub fn to_u64(&self) -> u64 {
        panic!()
    }
}

/// https://tools.ietf.org/html/rfc7541#section-5.2
#[derive(Debug)]
pub struct Str {}
