use std::collections::VecDeque;
use std::ops::{Add, AddAssign};
use trackable::error::Failed;

use Result;

#[derive(Debug)]
pub struct Table {
    dynamic_table: DynamicTable,
}
impl Table {
    pub fn new(max_dynamic_table_size: u16) -> Self {
        Table { dynamic_table: DynamicTable::new(max_dynamic_table_size) }
    }
    pub fn dynamic(&self) -> &DynamicTable {
        &self.dynamic_table
    }
    pub fn dynamic_mut(&mut self) -> &mut DynamicTable {
        &mut self.dynamic_table
    }
    pub fn get(&self, index: Index) -> Result<Entry> {
        if let Some(entry) = StaticEntry::from_index(index) {
            Ok(entry.into())
        } else {
            let dynamic_table_entry_index =
                (index.as_u16() - Index::dynamic_table_offset().as_u16()) as usize;
            let entry = track_assert_some!(
                self.dynamic_table.entries().get(dynamic_table_entry_index),
                Failed,
                "Too large index: {:?}",
                index
            );
            Ok(entry.as_ref())
        }
    }
    pub fn len(&self) -> u16 {
        (StaticEntry::entries_count() + self.dynamic_table.entries().len()) as u16
    }

    pub(crate) fn validate_index(&self, index: Index) -> Result<()> {
        let index = index.as_u16();
        let max_index = self.len();
        track_assert!(
            index <= max_index,
            Failed,
            "Too large index: {} (max={})",
            index,
            max_index
        );
        Ok(())
    }
}

#[derive(Debug)]
pub struct DynamicTable {
    entries: VecDeque<OwnedEntry>,
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
    pub fn entries(&self) -> &VecDeque<OwnedEntry> {
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

    pub(crate) fn push(&mut self, name: Vec<u8>, value: Vec<u8>) -> Option<OwnedEntry> {
        let entry = OwnedEntry { name, value };
        let entry_size = entry.as_ref().size();
        if self.size_soft_limit < entry_size {
            self.entries.clear();
            Some(entry)
        } else {
            self.evict_exceeded_entries(entry_size);
            self.size += entry_size;
            self.entries.push_front(entry);
            None
        }
    }

    fn evict_exceeded_entries(&mut self, new_entry_size: u16) {
        while self.size_soft_limit - new_entry_size < self.size {
            let evicted = self.entries.pop_back().expect("Never fails");
            self.size -= evicted.as_ref().size();
        }
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Index(u16);
impl Index {
    pub fn new(index: u16) -> Result<Self> {
        track_assert_ne!(index, 0, Failed);
        Ok(Index(index))
    }
    pub fn dynamic_table_offset() -> Self {
        Index(StaticEntry::entries_count() as u16 + 1)
    }
    pub fn as_u16(&self) -> u16 {
        self.0
    }
}
impl Add<u16> for Index {
    type Output = Self;
    fn add(self, rhs: u16) -> Self::Output {
        Index(self.0.checked_add(rhs).expect("Overflow"))
    }
}
impl AddAssign<u16> for Index {
    fn add_assign(&mut self, rhs: u16) {
        self.0 = self.0.checked_add(rhs).expect("Overflow");
    }
}
impl From<StaticEntry> for Index {
    fn from(f: StaticEntry) -> Self {
        Index(match f {
            StaticEntry::Authority => 1,
            StaticEntry::Method => 2, 
            StaticEntry::MethodGet => 2,
            StaticEntry::MethodPost => 3,
            StaticEntry::Path => 4,
            StaticEntry::PathRoot => 4,
            StaticEntry::PathIndexHtml => 5,
            StaticEntry::Scheme => 6,
            StaticEntry::SchemeHttp => 6,
            StaticEntry::SchemeHttps => 7,
            StaticEntry::Status => 8,
            StaticEntry::Status200 => 8,
            StaticEntry::Status204 => 9,
            StaticEntry::Status206 => 10,
            StaticEntry::Status304 => 11,
            StaticEntry::Status400 => 12,
            StaticEntry::Status404 => 13,
            StaticEntry::Status500 => 14,
            StaticEntry::AcceptCharset => 15,
            StaticEntry::AcceptEncoding => 16,
            StaticEntry::AcceptEncodingGzipDeflate => 16,
            StaticEntry::AcceptLanguage => 17,
            StaticEntry::AcceptRanges => 18,
            StaticEntry::Accept => 19,
            StaticEntry::AccessControlAllowOrigin => 20,
            StaticEntry::Age => 21,
            StaticEntry::Allow => 22,
            StaticEntry::Authorization => 23,
            StaticEntry::CacheControl => 24,
            StaticEntry::ContentDisposition => 25,
            StaticEntry::ContentEncoding => 26,
            StaticEntry::ContentLanguage => 27,
            StaticEntry::ContentLength => 28,
            StaticEntry::ContentLocation => 29,
            StaticEntry::ContentRange => 30,
            StaticEntry::ContentType => 31,
            StaticEntry::Cookie => 32,
            StaticEntry::Date => 33,
            StaticEntry::Etag => 34,
            StaticEntry::Expect => 35,
            StaticEntry::Expires => 36,
            StaticEntry::From => 37,
            StaticEntry::Host => 38,
            StaticEntry::IfMatch => 39,
            StaticEntry::IfModifiedSince => 40,
            StaticEntry::IfNoneMatch => 41,
            StaticEntry::IfRange => 42,
            StaticEntry::IfUnmodifiedSince => 43,
            StaticEntry::LastModified => 44,
            StaticEntry::Link => 45,
            StaticEntry::Location => 46,
            StaticEntry::MaxForwards => 47,
            StaticEntry::ProxyAuthenticate => 48,
            StaticEntry::ProxyAuthorization => 49,
            StaticEntry::Range => 50,
            StaticEntry::Referer => 51,
            StaticEntry::Refresh => 52,
            StaticEntry::RetryAfter => 53,
            StaticEntry::Server => 54,
            StaticEntry::SetCookie => 55,
            StaticEntry::StrictTransportSecurity => 56,
            StaticEntry::TransferEncoding => 57,
            StaticEntry::UserAgent => 58,
            StaticEntry::Vary => 59,
            StaticEntry::Via => 60,
            StaticEntry::WwwAuthenticate => 61,
        })
    }
}

#[derive(Debug)]
pub struct Entry<'a> {
    pub name: &'a [u8],
    pub value: &'a [u8],
}
impl<'a> Entry<'a> {
    pub fn size(&self) -> u16 {
        (self.name.len() + self.value.len() + 32) as u16
    }
}
impl From<StaticEntry> for Entry<'static> {
    fn from(f: StaticEntry) -> Self {
        macro_rules! entry {
            ($name:expr, $value: expr) => { Entry{ name: $name, value: $value } };
            ($name:expr) => { Entry{ name: $name, value: b"" } }
        }
        match f {
            StaticEntry::Authority => entry!(b":authority"),
            StaticEntry::Method => entry!(b":method", b"GET"),
            StaticEntry::MethodGet => entry!(b":method", b"GET"),            
            StaticEntry::MethodPost => entry!(b":method", b"POST"),
            StaticEntry::Path => entry!(b":path", b"/"),
            StaticEntry::PathRoot => entry!(b":path", b"/"),            
            StaticEntry::PathIndexHtml => entry!(b":path", b"/index.html"),
            StaticEntry::Scheme => entry!(b":scheme", b"http"),
            StaticEntry::SchemeHttp => entry!(b":scheme", b"http"),
            StaticEntry::SchemeHttps => entry!(b":scheme", b"https"),
            StaticEntry::Status => entry!(b":status", b"200"),
            StaticEntry::Status200 => entry!(b":status", b"200"),            
            StaticEntry::Status204 => entry!(b":status", b"204"),
            StaticEntry::Status206 => entry!(b":status", b"206"),
            StaticEntry::Status304 => entry!(b":status", b"304"),
            StaticEntry::Status400 => entry!(b":status", b"400"),
            StaticEntry::Status404 => entry!(b":status", b"404"),
            StaticEntry::Status500 => entry!(b":status", b"500"),
            StaticEntry::AcceptCharset => entry!(b"accept-charset"),
            StaticEntry::AcceptEncoding => entry!(b"accept-encoding", b"gzip, deflate"),
            StaticEntry::AcceptEncodingGzipDeflate => entry!(b"accept-encoding", b"gzip, deflate"),
            StaticEntry::AcceptLanguage => entry!(b"accept-language"),
            StaticEntry::AcceptRanges => entry!(b"accept-ranges"),
            StaticEntry::Accept => entry!(b"accept"),
            StaticEntry::AccessControlAllowOrigin => entry!(b"access-control-allow-origin"),
            StaticEntry::Age => entry!(b"age"),
            StaticEntry::Allow => entry!(b"allow"),
            StaticEntry::Authorization => entry!(b"authorization"),
            StaticEntry::CacheControl => entry!(b"cache-control"),
            StaticEntry::ContentDisposition => entry!(b"content-disposition"),
            StaticEntry::ContentEncoding => entry!(b"content-encoding"),
            StaticEntry::ContentLanguage => entry!(b"content-language"),
            StaticEntry::ContentLength => entry!(b"content-length"),
            StaticEntry::ContentLocation => entry!(b"content-location"),
            StaticEntry::ContentRange => entry!(b"content-range"),
            StaticEntry::ContentType => entry!(b"content-type"),
            StaticEntry::Cookie => entry!(b"cookie"),
            StaticEntry::Date => entry!(b"date"),
            StaticEntry::Etag => entry!(b"etag"),
            StaticEntry::Expect => entry!(b"expect"),
            StaticEntry::Expires => entry!(b"expires"),
            StaticEntry::From => entry!(b"from"),
            StaticEntry::Host => entry!(b"host"),
            StaticEntry::IfMatch => entry!(b"if-match"),
            StaticEntry::IfModifiedSince => entry!(b"if-modified-since"),
            StaticEntry::IfNoneMatch => entry!(b"if-none-match"),
            StaticEntry::IfRange => entry!(b"if-range"),
            StaticEntry::IfUnmodifiedSince => entry!(b"if-unmodified-since"),
            StaticEntry::LastModified => entry!(b"last-modified"),
            StaticEntry::Link => entry!(b"link"),
            StaticEntry::Location => entry!(b"location"),
            StaticEntry::MaxForwards => entry!(b"max-forwards"),
            StaticEntry::ProxyAuthenticate => entry!(b"proxy-authenticate"),
            StaticEntry::ProxyAuthorization => entry!(b"proxy-authorization"),
            StaticEntry::Range => entry!(b"range"),
            StaticEntry::Referer => entry!(b"referer"),
            StaticEntry::Refresh => entry!(b"refresh"),
            StaticEntry::RetryAfter => entry!(b"retry-after"),
            StaticEntry::Server => entry!(b"server"),
            StaticEntry::SetCookie => entry!(b"set-cookie"),
            StaticEntry::StrictTransportSecurity => entry!(b"strict-transport-security"),
            StaticEntry::TransferEncoding => entry!(b"transfer-encoding"),
            StaticEntry::UserAgent => entry!(b"user-agent"),
            StaticEntry::Vary => entry!(b"vary"),
            StaticEntry::Via => entry!(b"via"),
            StaticEntry::WwwAuthenticate => entry!(b"www-authenticate"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct OwnedEntry {
    pub name: Vec<u8>,
    pub value: Vec<u8>,
}
impl OwnedEntry {
    pub fn as_ref(&self) -> Entry {
        Entry {
            name: &self.name,
            value: &self.value,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum StaticEntry {
    Authority,
    Method,
    MethodGet,
    MethodPost,
    Path,
    PathRoot,
    PathIndexHtml,
    Scheme,
    SchemeHttp,
    SchemeHttps,
    Status,
    Status200,
    Status204,
    Status206,
    Status304,
    Status400,
    Status404,
    Status500,
    AcceptCharset,
    AcceptEncoding,
    AcceptEncodingGzipDeflate,
    AcceptLanguage,
    AcceptRanges,
    Accept,
    AccessControlAllowOrigin,
    Age,
    Allow,
    Authorization,
    CacheControl,
    ContentDisposition,
    ContentEncoding,
    ContentLanguage,
    ContentLength,
    ContentLocation,
    ContentRange,
    ContentType,
    Cookie,
    Date,
    Etag,
    Expect,
    Expires,
    From,
    Host,
    IfMatch,
    IfModifiedSince,
    IfNoneMatch,
    IfRange,
    IfUnmodifiedSince,
    LastModified,
    Link,
    Location,
    MaxForwards,
    ProxyAuthenticate,
    ProxyAuthorization,
    Range,
    Referer,
    Refresh,
    RetryAfter,
    Server,
    SetCookie,
    StrictTransportSecurity,
    TransferEncoding,
    UserAgent,
    Vary,
    Via,
    WwwAuthenticate,
}
impl StaticEntry {
    pub fn entries_count() -> usize {
        61
    }
    pub fn from_index(index: Index) -> Option<Self> {
        Some(match index.as_u16() {
            1 => StaticEntry::Authority,
            2 => StaticEntry::MethodGet,
            3 => StaticEntry::MethodPost,
            4 => StaticEntry::PathRoot,
            5 => StaticEntry::PathIndexHtml,
            6 => StaticEntry::SchemeHttp,
            7 => StaticEntry::SchemeHttps,
            8 => StaticEntry::Status200,
            9 => StaticEntry::Status204,
            10 => StaticEntry::Status206,
            11 => StaticEntry::Status304,
            12 => StaticEntry::Status400,
            13 => StaticEntry::Status404,
            14 => StaticEntry::Status500,
            15 => StaticEntry::AcceptCharset,
            16 => StaticEntry::AcceptEncodingGzipDeflate,
            17 => StaticEntry::AcceptLanguage,
            18 => StaticEntry::AcceptRanges,
            19 => StaticEntry::Accept,
            20 => StaticEntry::AccessControlAllowOrigin,
            21 => StaticEntry::Age,
            22 => StaticEntry::Allow,
            23 => StaticEntry::Authorization,
            24 => StaticEntry::CacheControl,
            25 => StaticEntry::ContentDisposition,
            26 => StaticEntry::ContentEncoding,
            27 => StaticEntry::ContentLanguage,
            28 => StaticEntry::ContentLength,
            29 => StaticEntry::ContentLocation,
            30 => StaticEntry::ContentRange,
            31 => StaticEntry::ContentType,
            32 => StaticEntry::Cookie,
            33 => StaticEntry::Date,
            34 => StaticEntry::Etag,
            35 => StaticEntry::Expect,
            36 => StaticEntry::Expires,
            37 => StaticEntry::From,
            38 => StaticEntry::Host,
            39 => StaticEntry::IfMatch,
            40 => StaticEntry::IfModifiedSince,
            41 => StaticEntry::IfNoneMatch,
            42 => StaticEntry::IfRange,
            43 => StaticEntry::IfUnmodifiedSince,
            44 => StaticEntry::LastModified,
            45 => StaticEntry::Link,
            46 => StaticEntry::Location,
            47 => StaticEntry::MaxForwards,
            48 => StaticEntry::ProxyAuthenticate,
            49 => StaticEntry::ProxyAuthorization,
            50 => StaticEntry::Range,
            51 => StaticEntry::Referer,
            52 => StaticEntry::Refresh,
            53 => StaticEntry::RetryAfter,
            54 => StaticEntry::Server,
            55 => StaticEntry::SetCookie,
            56 => StaticEntry::StrictTransportSecurity,
            57 => StaticEntry::TransferEncoding,
            58 => StaticEntry::UserAgent,
            59 => StaticEntry::Vary,
            60 => StaticEntry::Via,
            61 => StaticEntry::WwwAuthenticate,
            _ => return None,
        })
    }
}
