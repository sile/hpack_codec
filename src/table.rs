//! Header Field Table.
//!
//! See: [2.3.  Indexing Tables](https://tools.ietf.org/html/rfc7541#section-2.3)
use crate::field::HeaderField;
use crate::Result;
use std::borrow::Cow;
use std::collections::VecDeque;
use std::ops::{Add, AddAssign};
use trackable::error::Failed;

/// Table for associating header fields to indexes.
///
/// See: [2.3.  Indexing Tables](https://tools.ietf.org/html/rfc7541#section-2.3)
#[derive(Debug)]
pub struct Table {
    dynamic_table: DynamicTable,
}
impl Table {
    /// Makes a new `Table` instance.
    pub fn new(max_dynamic_table_size: u16) -> Self {
        Table {
            dynamic_table: DynamicTable::new(max_dynamic_table_size),
        }
    }

    /// Returns the reference to `DynamicTable` instance.
    pub fn dynamic(&self) -> &DynamicTable {
        &self.dynamic_table
    }

    /// Returns the mutable reference to the `DynamicTable` instance.
    pub fn dynamic_mut(&mut self) -> &mut DynamicTable {
        &mut self.dynamic_table
    }

    /// Returns the entry associated with the specified index.
    ///
    /// # Errors
    ///
    /// If `index` value is too large, an error will be returned.
    pub fn get(&self, index: Index) -> Result<HeaderField> {
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
            Ok(entry.as_borrowed())
        }
    }

    /// Returns the number of indexed entries.
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

/// Dynamic Indexing Table.
///
/// See: [2.3.2.  Dynamic Table](https://tools.ietf.org/html/rfc7541#section-2.3.2)
#[derive(Debug)]
pub struct DynamicTable {
    entries: VecDeque<HeaderField<'static>>,
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

    /// Returns the reference to the dynamically indexed entries
    pub fn entries(&self) -> &VecDeque<HeaderField<'static>> {
        &self.entries
    }

    /// Returns the size of this table.
    ///
    /// See: [4.1.  Calculating Table Size](https://tools.ietf.org/html/rfc7541#section-4.1)
    pub fn size(&self) -> u16 {
        self.size
    }

    /// Returns the hard limit of the size of this table.
    ///
    /// See: [4.2.  Maximum Table Size](https://tools.ietf.org/html/rfc7541#section-4.2)
    pub fn size_hard_limit(&self) -> u16 {
        self.size_hard_limit
    }

    /// Returns the soft limit of the size of this table.
    ///
    /// See: [4.2.  Maximum Table Size](https://tools.ietf.org/html/rfc7541#section-4.2)
    pub fn size_soft_limit(&self) -> u16 {
        self.size_soft_limit
    }

    /// Sets the hard limit of the size of this table.
    ///
    /// Note that the soft limit will be truncated to `max_size` if it is greater than `max_size`.
    pub fn set_size_hard_limit(&mut self, max_size: u16) {
        self.size_hard_limit = max_size;
        if self.size_hard_limit < self.size_soft_limit {
            self.set_size_soft_limit(max_size).expect("Never fails");
        }
    }

    /// Sets the soft limit of the size of this table.
    ///
    /// # Errors
    ///
    /// If `max_size` exceeds the hard limit of this table, an error will be returned.
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

    pub(crate) fn push(&mut self, name: Vec<u8>, value: Vec<u8>) -> Option<HeaderField<'static>> {
        let field = HeaderField::from_cow(Cow::Owned(name), Cow::Owned(value));
        let entry_size = field.entry_size();
        if self.size_soft_limit < entry_size {
            self.entries.clear();
            Some(field)
        } else {
            self.evict_exceeded_entries(entry_size);
            self.size += entry_size;
            self.entries.push_front(field);
            None
        }
    }

    fn evict_exceeded_entries(&mut self, new_entry_size: u16) {
        while self.size_soft_limit - new_entry_size < self.size {
            let evicted = self.entries.pop_back().expect("Never fails");
            self.size -= evicted.entry_size();
        }
    }
}

/// Entry Index.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Index(u16);
impl Index {
    /// Makes a new `Index` instance.
    ///
    /// The value of `index` must be greater than zero.
    ///
    /// # Examples
    ///
    /// ```
    /// use hpack_codec::table::Index;
    ///
    /// assert!(Index::new(1).is_ok());
    /// assert!(Index::new(0).is_err());
    /// ```
    pub fn new(index: u16) -> Result<Self> {
        track_assert_ne!(index, 0, Failed);
        Ok(Index(index))
    }

    /// Returns the `Index` instance which has the starting index of the dynamic table.
    ///
    /// See: [2.3.3.  Index Address Space](https://tools.ietf.org/html/rfc7541#section-2.3.3)
    ///
    /// # Examples
    ///
    /// ```
    /// use hpack_codec::table::Index;
    ///
    /// assert_eq!(Index::dynamic_table_offset().as_u16(), 62);
    /// assert_eq!(Index::dynamic_table_offset() + 8, Index::new(70).unwrap());
    /// ```
    pub fn dynamic_table_offset() -> Self {
        Index(StaticEntry::entries_count() as u16 + 1)
    }

    /// Returns the value of this index.
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
impl From<StaticEntry> for HeaderField<'static> {
    fn from(f: StaticEntry) -> Self {
        macro_rules! field {
            ($name:expr, $value: expr) => {
                HeaderField::from_cow(Cow::Borrowed($name), Cow::Borrowed($value))
            };
            ($name:expr) => {
                HeaderField::from_cow(Cow::Borrowed($name), Cow::Borrowed(b""))
            };
        }
        match f {
            StaticEntry::Authority => field!(b":authority"),
            StaticEntry::Method => field!(b":method", b"GET"),
            StaticEntry::MethodGet => field!(b":method", b"GET"),
            StaticEntry::MethodPost => field!(b":method", b"POST"),
            StaticEntry::Path => field!(b":path", b"/"),
            StaticEntry::PathRoot => field!(b":path", b"/"),
            StaticEntry::PathIndexHtml => field!(b":path", b"/index.html"),
            StaticEntry::Scheme => field!(b":scheme", b"http"),
            StaticEntry::SchemeHttp => field!(b":scheme", b"http"),
            StaticEntry::SchemeHttps => field!(b":scheme", b"https"),
            StaticEntry::Status => field!(b":status", b"200"),
            StaticEntry::Status200 => field!(b":status", b"200"),
            StaticEntry::Status204 => field!(b":status", b"204"),
            StaticEntry::Status206 => field!(b":status", b"206"),
            StaticEntry::Status304 => field!(b":status", b"304"),
            StaticEntry::Status400 => field!(b":status", b"400"),
            StaticEntry::Status404 => field!(b":status", b"404"),
            StaticEntry::Status500 => field!(b":status", b"500"),
            StaticEntry::AcceptCharset => field!(b"accept-charset"),
            StaticEntry::AcceptEncoding => field!(b"accept-encoding", b"gzip, deflate"),
            StaticEntry::AcceptEncodingGzipDeflate => field!(b"accept-encoding", b"gzip, deflate"),
            StaticEntry::AcceptLanguage => field!(b"accept-language"),
            StaticEntry::AcceptRanges => field!(b"accept-ranges"),
            StaticEntry::Accept => field!(b"accept"),
            StaticEntry::AccessControlAllowOrigin => field!(b"access-control-allow-origin"),
            StaticEntry::Age => field!(b"age"),
            StaticEntry::Allow => field!(b"allow"),
            StaticEntry::Authorization => field!(b"authorization"),
            StaticEntry::CacheControl => field!(b"cache-control"),
            StaticEntry::ContentDisposition => field!(b"content-disposition"),
            StaticEntry::ContentEncoding => field!(b"content-encoding"),
            StaticEntry::ContentLanguage => field!(b"content-language"),
            StaticEntry::ContentLength => field!(b"content-length"),
            StaticEntry::ContentLocation => field!(b"content-location"),
            StaticEntry::ContentRange => field!(b"content-range"),
            StaticEntry::ContentType => field!(b"content-type"),
            StaticEntry::Cookie => field!(b"cookie"),
            StaticEntry::Date => field!(b"date"),
            StaticEntry::Etag => field!(b"etag"),
            StaticEntry::Expect => field!(b"expect"),
            StaticEntry::Expires => field!(b"expires"),
            StaticEntry::From => field!(b"from"),
            StaticEntry::Host => field!(b"host"),
            StaticEntry::IfMatch => field!(b"if-match"),
            StaticEntry::IfModifiedSince => field!(b"if-modified-since"),
            StaticEntry::IfNoneMatch => field!(b"if-none-match"),
            StaticEntry::IfRange => field!(b"if-range"),
            StaticEntry::IfUnmodifiedSince => field!(b"if-unmodified-since"),
            StaticEntry::LastModified => field!(b"last-modified"),
            StaticEntry::Link => field!(b"link"),
            StaticEntry::Location => field!(b"location"),
            StaticEntry::MaxForwards => field!(b"max-forwards"),
            StaticEntry::ProxyAuthenticate => field!(b"proxy-authenticate"),
            StaticEntry::ProxyAuthorization => field!(b"proxy-authorization"),
            StaticEntry::Range => field!(b"range"),
            StaticEntry::Referer => field!(b"referer"),
            StaticEntry::Refresh => field!(b"refresh"),
            StaticEntry::RetryAfter => field!(b"retry-after"),
            StaticEntry::Server => field!(b"server"),
            StaticEntry::SetCookie => field!(b"set-cookie"),
            StaticEntry::StrictTransportSecurity => field!(b"strict-transport-security"),
            StaticEntry::TransferEncoding => field!(b"transfer-encoding"),
            StaticEntry::UserAgent => field!(b"user-agent"),
            StaticEntry::Vary => field!(b"vary"),
            StaticEntry::Via => field!(b"via"),
            StaticEntry::WwwAuthenticate => field!(b"www-authenticate"),
        }
    }
}

/// A list specifying the entries of the static table.
///
/// See: [Appendix A.  Static Table Definition)(https://tools.ietf.org/html/rfc7541#appendix-A)
#[derive(Debug, Clone, Copy)]
#[allow(missing_docs)]
pub enum StaticEntry {
    Authority,
    /// This is an alias of `MethodGet`.
    Method,
    MethodGet,
    MethodPost,
    /// This is an alias of `PathRoot`.
    Path,
    PathRoot,
    PathIndexHtml,
    /// This is an alias of `SchemeHttp`.
    Scheme,
    SchemeHttp,
    SchemeHttps,
    /// This is an alias of `Status200`.
    Status,
    Status200,
    Status204,
    Status206,
    Status304,
    Status400,
    Status404,
    Status500,
    AcceptCharset,
    /// This is an alias of `AcceptEncodingGzipDeflate`.
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
    /// Returns the entries count of the static table.
    ///
    /// # Examples
    ///
    /// ```
    /// use hpack_codec::table::StaticEntry;
    ///
    /// assert_eq!(StaticEntry::entries_count(), 61);
    /// ```
    pub fn entries_count() -> usize {
        61
    }

    /// Makes a new `StaticEntry` instance associated with the specified index.
    ///
    /// If the value of `index` is greater than `StaticEntry::entries_count()`,
    /// this function will return `None`.
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
