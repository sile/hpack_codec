use std;
use std::collections::VecDeque;
use trackable::error::Failed;

use Result;

#[derive(Debug)]
pub struct DynamicTable {
    entries: VecDeque<Entry<Vec<u8>>>,
    size: u16,
    size_soft_limit: u16,
    size_hard_limit: u16,
}
impl DynamicTable {
    pub(crate) fn new(max_size: u16) -> Self {
        DynamicTable {
            entries: VecDeque::new(),
            size: 0,
            size_soft_limit: max_size,
            size_hard_limit: max_size,
        }
    }
    pub fn entries(&self) -> &VecDeque<Entry<Vec<u8>>> {
        &self.entries
    }
    pub fn size(&self) -> u16 {
        self.size
    }
    pub fn size_soft_limit(&self) -> u16 {
        self.size_soft_limit
    }
    pub fn size_hard_limit(&self) -> u16 {
        self.size_hard_limit
    }
    pub fn set_size_hard_limit(&mut self, max_size: u16) {
        self.size_hard_limit = max_size;
        if self.size_hard_limit < self.size_soft_limit {
            self.set_size_soft_limit(max_size).expect("Never fails");
        }
    }
    pub fn set_size_soft_limit(&mut self, max_size: u16) -> Result<()> {
        track_assert!(
            max_size <= self.size_hard_limit,
            Failed,
            "new_soft_limit={}, hard_limit={}",
            max_size,
            self.size_hard_limit
        );
        self.size_soft_limit = max_size;
        self.evict_exceeded_entries(0);
        Ok(())
    }

    pub(crate) fn push_entry(&mut self, name: Vec<u8>, value: Vec<u8>) -> Option<Entry<Vec<u8>>> {
        let entry = Entry { name, value };
        if self.size_soft_limit < entry.size() {
            self.entries.clear();
            Some(entry)
        } else {
            self.evict_exceeded_entries(entry.size());
            self.size += entry.size();
            self.entries.push_front(entry);
            None
        }
    }

    fn evict_exceeded_entries(&mut self, new_entry_size: u16) {
        while self.size_soft_limit - new_entry_size < self.size {
            let evicted = self.entries.pop_back().expect("Never fails");
            self.size -= evicted.size();
        }
    }
}

#[derive(Debug, Clone)]
pub struct Entry<B> {
    pub name: B,
    pub value: B,
}
impl<B: AsRef<[u8]>> Entry<B> {
    pub fn size(&self) -> u16 {
        let size = self.name.as_ref().len() + self.value.as_ref().len() + 32;
        debug_assert!(size <= std::u16::MAX as usize);
        size as u16
    }
    pub fn as_ref(&self) -> Entry<&[u8]> {
        Entry {
            name: self.name.as_ref(),
            value: self.value.as_ref(),
        }
    }
}

macro_rules! entry {
    ($name:expr, $value: expr) => {
        Entry{ name: $name, value: $value }
    };
    ($name:expr) => {
        Entry{ name: $name, value: b"" }
    }
}

pub const STATIC_TABLE: &[Entry<&[u8]>; 61] = &[
    entry!(b":authority"),
    entry!(b":method", b"GET"),
    entry!(b":method", b"POST"),
    entry!(b":path", b"/"),
    entry!(b":path", b"/index.html"),
    entry!(b":scheme", b"http"),
    entry!(b":scheme", b"https"),
    entry!(b":status", b"200"),
    entry!(b":status", b"204"),
    entry!(b":status", b"206"),
    entry!(b":status", b"304"),
    entry!(b":status", b"400"),
    entry!(b":status", b"404"),
    entry!(b":status", b"500"),
    entry!(b"accept-charset"),
    entry!(b"accept-encoding", b"gzip, deflate"),
    entry!(b"accept-language"),
    entry!(b"accept-ranges"),
    entry!(b"accept"),
    entry!(b"access-control-allow-origin"),
    entry!(b"age"),
    entry!(b"allow"),
    entry!(b"authorization"),
    entry!(b"cache-control"),
    entry!(b"content-disposition"),
    entry!(b"content-encoding"),
    entry!(b"content-language"),
    entry!(b"content-length"),
    entry!(b"content-location"),
    entry!(b"content-range"),
    entry!(b"content-type"),
    entry!(b"cookie"),
    entry!(b"date"),
    entry!(b"etag"),
    entry!(b"expect"),
    entry!(b"expires"),
    entry!(b"from"),
    entry!(b"host"),
    entry!(b"if-match"),
    entry!(b"if-modified-since"),
    entry!(b"if-none-match"),
    entry!(b"if-range"),
    entry!(b"if-unmodified-since"),
    entry!(b"last-modified"),
    entry!(b"link"),
    entry!(b"location"),
    entry!(b"max-forwards"),
    entry!(b"proxy-authenticate"),
    entry!(b"proxy-authorization"),
    entry!(b"range"),
    entry!(b"referer"),
    entry!(b"refresh"),
    entry!(b"retry-after"),
    entry!(b"server"),
    entry!(b"set-cookie"),
    entry!(b"strict-transport-security"),
    entry!(b"transfer-encoding"),
    entry!(b"user-agent"),
    entry!(b"vary"),
    entry!(b"via"),
    entry!(b"www-authenticate"),
];
