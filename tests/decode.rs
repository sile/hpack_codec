extern crate hpack_codec;
#[macro_use]
extern crate trackable;

use hpack_codec::decoder::Decoder;
use hpack_codec::field::Reader;

macro_rules! assert_decode {
    ($decoder:expr, $reader:expr, $key:expr, $value:expr) => {
        {
            let field = track_try_unwrap!($decoder.decode(&mut $reader));
            assert_eq!(field.name.as_ref(), $key);
            assert_eq!(field.value.as_ref(), $value);
        }
    }
}

#[test]
/// https://tools.ietf.org/html/rfc7541#appendix-C.3
fn request_examples_without_huffman_coding() {
    let mut decoder = Decoder::new(4096);

    // C.3.1. First Request
    let encoded_data;
    #[cfg_attr(rustfmt, rustfmt_skip)]
    {
        encoded_data = [
            0x82, 0x86, 0x84, 0x41, 0x0f, 0x77, 0x77, 0x77, 0x2e, 0x65,
            0x78, 0x61, 0x6d, 0x70, 0x6c, 0x65, 0x2e, 0x63, 0x6f, 0x6d
        ];
    }
    let mut reader = Reader::new(&encoded_data[..]);
    assert_decode!(decoder, reader, b":method", b"GET");
    assert_decode!(decoder, reader, b":scheme", b"http");
    assert_decode!(decoder, reader, b":path", b"/");
    assert_decode!(decoder, reader, b":authority", b"www.example.com");
    assert!(reader.eos());
    assert_eq!(decoder.table_size(), 57);

    // C.3.2. Second Request
    let encoded_data;
    #[cfg_attr(rustfmt, rustfmt_skip)]
    {
        encoded_data = [
            0x82, 0x86, 0x84, 0xbe, 0x58, 0x08, 0x6e,
            0x6f, 0x2d, 0x63, 0x61, 0x63, 0x68, 0x65
        ];
    }
    let mut reader = Reader::new(&encoded_data[..]);
    assert_decode!(decoder, reader, b":method", b"GET");
    assert_decode!(decoder, reader, b":scheme", b"http");
    assert_decode!(decoder, reader, b":path", b"/");
    assert_decode!(decoder, reader, b":authority", b"www.example.com");
    assert_decode!(decoder, reader, b"cache-control", b"no-cache");
    assert!(reader.eos());
    assert_eq!(decoder.table_size(), 110);

    // C.3.3. Third Request
    let encoded_data;
    #[cfg_attr(rustfmt, rustfmt_skip)]
    {
        encoded_data = [
            0x82, 0x87, 0x85, 0xbf, 0x40, 0x0a, 0x63, 0x75, 0x73, 0x74, 0x6f,
            0x6d, 0x2d, 0x6b, 0x65, 0x79, 0x0c, 0x63, 0x75, 0x73, 0x74, 0x6f,
            0x6d, 0x2d, 0x76, 0x61, 0x6c, 0x75 ,0x65
        ];
    }
    let mut reader = Reader::new(&encoded_data[..]);
    assert_decode!(decoder, reader, b":method", b"GET");
    assert_decode!(decoder, reader, b":scheme", b"https");
    assert_decode!(decoder, reader, b":path", b"/index.html");
    assert_decode!(decoder, reader, b":authority", b"www.example.com");
    assert_decode!(decoder, reader, b"custom-key", b"custom-value");
    assert!(reader.eos());
    assert_eq!(decoder.table_size(), 164);
}


#[test]
/// https://tools.ietf.org/html/rfc7541#appendix-C.5
fn response_examples_without_huffman_coding() {
    let mut decoder = Decoder::new(256);

    // C.5.1. First Response
    let encoded_data;
    #[cfg_attr(rustfmt, rustfmt_skip)]
    {
        encoded_data = [
            0x48, 0x03, 0x33, 0x30, 0x32, 0x58, 0x07, 0x70, 0x72, 0x69,
            0x76, 0x61, 0x74, 0x65, 0x61, 0x1d, 0x4d, 0x6f, 0x6e, 0x2c,
            0x20, 0x32, 0x31, 0x20, 0x4f, 0x63, 0x74, 0x20, 0x32, 0x30,
            0x31, 0x33, 0x20, 0x32, 0x30, 0x3a, 0x31, 0x33, 0x3a, 0x32,
            0x31, 0x20, 0x47, 0x4d, 0x54, 0x6e, 0x17, 0x68, 0x74, 0x74,
            0x70, 0x73, 0x3a, 0x2f, 0x2f, 0x77, 0x77, 0x77, 0x2e, 0x65,
            0x78, 0x61, 0x6d, 0x70, 0x6c, 0x65, 0x2e, 0x63, 0x6f, 0x6d
        ];
    }
    let mut reader = Reader::new(&encoded_data[..]);
    assert_decode!(decoder, reader, b":status", b"302");
    assert_decode!(decoder, reader, b"cache-control", b"private");
    assert_decode!(decoder, reader, b"date", b"Mon, 21 Oct 2013 20:13:21 GMT");
    assert_decode!(decoder, reader, b"location", b"https://www.example.com");
    assert!(reader.eos());
    assert_eq!(decoder.table_size(), 222);

    // C.5.2. Second Response
    let encoded_data;
    #[cfg_attr(rustfmt, rustfmt_skip)]
    {
        encoded_data = [
            0x48, 0x03, 0x33, 0x30, 0x37, 0xc1, 0xc0, 0xbf
        ];
    }
    let mut reader = Reader::new(&encoded_data[..]);
    assert_decode!(decoder, reader, b":status", b"307");
    assert_decode!(decoder, reader, b"cache-control", b"private");
    assert_decode!(decoder, reader, b"date", b"Mon, 21 Oct 2013 20:13:21 GMT");
    assert_decode!(decoder, reader, b"location", b"https://www.example.com");
    assert!(reader.eos());
    assert_eq!(decoder.table_size(), 222);

    // C.5.3. Third Response
    let encoded_data;
    #[cfg_attr(rustfmt, rustfmt_skip)]
    {
        encoded_data = [
            0x88, 0xc1, 0x61, 0x1d, 0x4d, 0x6f, 0x6e, 0x2c,
            0x20, 0x32, 0x31, 0x20, 0x4f, 0x63, 0x74, 0x20,
            0x32, 0x30, 0x31, 0x33, 0x20, 0x32, 0x30, 0x3a,
            0x31, 0x33, 0x3a, 0x32, 0x32, 0x20, 0x47, 0x4d,
            0x54, 0xc0, 0x5a, 0x04, 0x67, 0x7a, 0x69, 0x70,
            0x77, 0x38, 0x66, 0x6f, 0x6f, 0x3d, 0x41, 0x53,
            0x44, 0x4a, 0x4b, 0x48, 0x51, 0x4b, 0x42, 0x5a,
            0x58, 0x4f, 0x51, 0x57, 0x45, 0x4f, 0x50, 0x49,
            0x55, 0x41, 0x58, 0x51, 0x57, 0x45, 0x4f, 0x49,
            0x55, 0x3b, 0x20, 0x6d, 0x61, 0x78, 0x2d, 0x61,
            0x67, 0x65, 0x3d, 0x33, 0x36, 0x30, 0x30, 0x3b,
            0x20, 0x76, 0x65, 0x72, 0x73, 0x69, 0x6f, 0x6e,
            0x3d, 0x31
        ];
    }
    let mut reader = Reader::new(&encoded_data[..]);
    assert_decode!(decoder, reader, b":status", b"200");
    assert_decode!(decoder, reader, b"cache-control", b"private");
    assert_decode!(decoder, reader, b"date", b"Mon, 21 Oct 2013 20:13:22 GMT");
    assert_decode!(decoder, reader, b"location", b"https://www.example.com");
    assert_decode!(decoder, reader, b"content-encoding", b"gzip");
    assert_decode!(
        decoder,
        reader,
        b"set-cookie",
        &b"foo=ASDJKHQKBZXOQWEOPIUAXQWEOIU; max-age=3600; version=1"[..]
    );
    assert!(reader.eos());
    assert_eq!(decoder.table_size(), 215);
}
