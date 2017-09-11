extern crate byteorder;
#[macro_use]
extern crate trackable;

use std::collections::VecDeque;

pub use error::{Error, ErrorKind};

use field::Index;

macro_rules! track_io {
    ($e:expr) => {
        $e.map_err(|e| {
            use ::trackable::error::ErrorKindExt;
            ::ErrorKind::Io.cause(e)
        })
    }
}

pub mod decoder; // TODO: private
mod error;
pub mod field; // TODO: private
pub mod literal; // TODO: private

pub type Result<T> = ::std::result::Result<T, Error>;

#[derive(Debug)]
pub struct Context {
    dynamic_table: DynamicTable,
}
impl Context {
    pub fn new(max_table_size: u16) -> Self {
        Context { dynamic_table: DynamicTable::new(max_table_size) }
    }
    pub fn find_entry(&self, index: Index) -> Result<Entry<&[u8]>> {
        debug_assert_ne!(index.as_u16(), 0);
        let index = index.as_u16() as usize - 1;
        if index < 61 {
            Ok(STATIC_TABLE[index].clone())
        } else {
            track!(self.dynamic_table.find_entry(index - 61))
        }
    }
}

#[derive(Debug)]
pub struct DynamicTable {
    entries: VecDeque<Entry<Vec<u8>>>,
    entries_bytes: usize,
    max_table_size: u16,
    table_size_limit: u16,
}
impl DynamicTable {
    pub fn new(max_table_size: u16) -> Self {
        DynamicTable {
            entries: VecDeque::new(),
            entries_bytes: 0,
            max_table_size,
            table_size_limit: max_table_size,
        }
    }

    pub fn first_entry(&self) -> Option<Entry<&[u8]>> {
        self.entries.front().map(|e| {
            Entry {
                name: &e.name[..],
                value: &e.value[..],
            }
        })
    }
    pub fn size(&self) -> usize {
        self.entries_bytes
    }
    pub fn set_size_limit(&mut self, size: u16) {
        self.table_size_limit = size;
        if self.table_size_limit < self.max_table_size {
            self.set_max_size(size).expect("Never fails");
        }
    }
    pub fn set_max_size(&mut self, size: u16) -> Result<()> {
        track_assert!(
            size <= self.table_size_limit,
            ErrorKind::InvalidInput,
            "size={}, limit={}",
            size,
            self.table_size_limit
        );
        self.max_table_size = size;

        while (self.max_table_size as usize) < self.entries_bytes {
            let last = self.entries.pop_back();
            self.entries_bytes -= last.as_ref().unwrap().size();
        }

        Ok(())
    }
    pub fn push_entry(&mut self, name: Vec<u8>, value: Vec<u8>) -> Option<Entry<Vec<u8>>> {
        let entry = Entry { name, value };
        self.entries_bytes += entry.size();
        self.entries.push_front(entry);
        let mut last = None;
        while (self.max_table_size as usize) < self.entries_bytes {
            last = self.entries.pop_back();
            self.entries_bytes -= last.as_ref().unwrap().size();
        }
        if self.entries.is_empty() { last } else { None }
    }

    pub fn find_entry(&self, index: usize) -> Result<Entry<&[u8]>> {
        let entry = track_assert_some!(
            self.entries.get(index),
            ErrorKind::InvalidInput,
            "Unknown index: {}",
            index
        );
        Ok(Entry {
            name: &entry.name,
            value: &entry.value,
        })
    }

    /// https://tools.ietf.org/html/rfc7541#section-4.1
    pub fn table_size(&self) -> usize {
        self.entries
            .iter()
            .map(|h| h.name.len() + h.value.len() + 32)
            .sum()
    }
    pub fn set_max_table_size(&mut self, size: u16) {
        // TODO: https://tools.ietf.org/html/rfc7541#section-4.3
        self.max_table_size = size;
    }
}

#[derive(Debug)]
pub struct Encoder {
    context: Context,
}

#[derive(Debug, Clone)]
pub struct Entry<B> {
    pub name: B,
    pub value: B,
}
impl<B: AsRef<[u8]>> Entry<B> {
    pub fn size(&self) -> usize {
        self.name.as_ref().len() + self.value.as_ref().len() + 32
    }
}

macro_rules! field {
    ($name:expr, $value: expr) => {
        Entry{ name: $name, value: $value }
    };
    ($name:expr) => {
        Entry{ name: $name, value: b"" }
    }
}

pub const STATIC_TABLE: &[Entry<&[u8]>; 61] = &[
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
