use crate::{
    BulkString, RespArray, RespDecode, RespError, RespFrame, RespNull, RespNullArray,
    RespNullBulkString, SimpleError, SimpleString,
};
use bytes::{Buf, BytesMut};

use super::{RespMap, RespSet};

const CRLF: &[u8] = b"\r\n";
const CRLF_LEN: usize = CRLF.len();

impl RespDecode for RespFrame {
    const PREFIX: &'static str = "";
    fn decode(buf: &mut BytesMut) -> Result<Self, RespError> {
        let mut iter = buf.iter().peekable();
        match iter.peek() {
            Some(b'+') => {
                let frame = SimpleString::decode(buf)?;
                Ok(frame.into())
            }
            Some(b'-') => {
                let frame = SimpleError::decode(buf)?;
                Ok(frame.into())
            }
            Some(b':') => {
                let frame = i64::decode(buf)?;
                Ok(frame.into())
            }
            Some(b'$') => {
                // try null bulk string first
                match RespNullBulkString::decode(buf) {
                    Ok(frame) => Ok(frame.into()),
                    Err(RespError::NotComplete) => Err(RespError::NotComplete),
                    Err(_) => {
                        let frame = BulkString::decode(buf)?;
                        Ok(frame.into())
                    }
                }
            }
            Some(b'*') => {
                // try null array first
                match RespNullArray::decode(buf) {
                    Ok(frame) => Ok(frame.into()),
                    Err(RespError::NotComplete) => Err(RespError::NotComplete),
                    Err(_) => {
                        let frame = RespArray::decode(buf)?;
                        Ok(frame.into())
                    }
                }
            }
            Some(b'_') => {
                let frame = RespNull::decode(buf)?;
                Ok(frame.into())
            }
            Some(b'#') => {
                let frame = bool::decode(buf)?;
                Ok(frame.into())
            }
            Some(b',') => {
                let frame = f64::decode(buf)?;
                Ok(frame.into())
            }
            Some(b'%') => {
                let frame = RespMap::decode(buf)?;
                Ok(frame.into())
            }
            Some(b'~') => {
                let frame = RespSet::decode(buf)?;
                Ok(frame.into())
            }
            _ => Err(RespError::InvalidFrameType(format!(
                "expect_length: unknown frame type: {:?}",
                buf
            ))),
        }
    }

    fn expect_length(buf: &[u8]) -> Result<usize, RespError> {
        let mut iter = buf.iter().peekable();
        match iter.peek() {
            Some(b'*') => RespArray::expect_length(buf),
            Some(b'~') => RespSet::expect_length(buf),
            Some(b'%') => RespMap::expect_length(buf),
            Some(b'$') => BulkString::expect_length(buf),
            Some(b':') => i64::expect_length(buf),
            Some(b'+') => SimpleString::expect_length(buf),
            Some(b'-') => SimpleError::expect_length(buf),
            Some(b'#') => bool::expect_length(buf),
            Some(b',') => f64::expect_length(buf),
            Some(b'_') => RespNull::expect_length(buf),
            _ => Err(RespError::NotComplete),
        }
    }
}

impl RespDecode for SimpleString {
    const PREFIX: &'static str = "+";
    fn decode(buf: &mut BytesMut) -> Result<Self, RespError> {
        let end = extract_simple_frame_data(buf, Self::PREFIX)?;

        let data = buf.split_to(end + CRLF_LEN);
        let s = String::from_utf8_lossy(&data[Self::PREFIX.len()..end]);
        Ok(SimpleString::new(s.to_string()))
    }

    fn expect_length(buf: &[u8]) -> Result<usize, RespError> {
        let end = extract_simple_frame_data(buf, Self::PREFIX)?;
        Ok(end + CRLF_LEN)
    }
}

impl RespDecode for SimpleError {
    const PREFIX: &'static str = "-";
    fn decode(buf: &mut BytesMut) -> Result<Self, RespError> {
        // // split the buffer
        let end = extract_simple_frame_data(buf, Self::PREFIX)?;
        let data = buf.split_to(end + CRLF_LEN);
        let s = String::from_utf8_lossy(&data[Self::PREFIX.len()..end]);
        Ok(SimpleError::new(s.to_string()))
    }

    fn expect_length(buf: &[u8]) -> Result<usize, RespError> {
        let end = extract_simple_frame_data(buf, Self::PREFIX)?;
        Ok(end + CRLF_LEN)
    }
}

impl RespDecode for RespNull {
    const PREFIX: &'static str = "_";
    fn decode(buf: &mut BytesMut) -> Result<Self, RespError> {
        extract_fixed_data(buf, "_\r\n", "Null")?;
        Ok(RespNull)
    }

    fn expect_length(_buf: &[u8]) -> Result<usize, RespError> {
        Ok(3)
    }
}

impl RespDecode for RespNullArray {
    const PREFIX: &'static str = "*";
    fn decode(buf: &mut BytesMut) -> Result<Self, RespError> {
        extract_fixed_data(buf, "*-1\r\n", "NullArray")?;
        Ok(RespNullArray)
    }

    fn expect_length(_buf: &[u8]) -> Result<usize, RespError> {
        Ok(4)
    }
}

impl RespDecode for RespNullBulkString {
    const PREFIX: &'static str = "$";
    fn decode(buf: &mut BytesMut) -> Result<Self, RespError> {
        extract_fixed_data(buf, "$-1\r\n", "NullBulkString")?;
        Ok(RespNullBulkString)
    }

    fn expect_length(_buf: &[u8]) -> Result<usize, RespError> {
        Ok(5)
    }
}

impl RespDecode for i64 {
    const PREFIX: &'static str = ":";
    fn decode(buf: &mut BytesMut) -> Result<Self, RespError> {
        let end = extract_simple_frame_data(buf, Self::PREFIX)?;
        // split the buffer
        let data = buf.split_to(end + CRLF_LEN);
        let s = String::from_utf8_lossy(&data[Self::PREFIX.len()..end]);
        Ok(s.parse()?)
    }

    fn expect_length(buf: &[u8]) -> Result<usize, RespError> {
        let end = extract_simple_frame_data(buf, Self::PREFIX)?;
        Ok(end + CRLF_LEN)
    }
}

impl RespDecode for bool {
    const PREFIX: &'static str = "#";
    /// Decode a RESP boolean frame.
    ///
    /// `#t\r\n` is `true`, `#f\r\n` is `false`.
    ///
    /// Errors:
    ///
    /// - `RespError::NotComplete` if the buffer is not complete.
    /// - `RespError::InvalidFrameType` if the prefix is not `#`.
    /// - `RespError::InvalidFrameLength` if the length is not 1 or 2.
    fn decode(buf: &mut BytesMut) -> Result<Self, RespError> {
        match extract_fixed_data(buf, "#t\r\n", "Bool") {
            Ok(_) => Ok(true),
            Err(RespError::NotComplete) => Err(RespError::NotComplete),
            Err(_) => match extract_fixed_data(buf, "#f\r\n", "Bool") {
                Ok(_) => Ok(false),
                Err(e) => Err(e),
            },
        }
    }

    fn expect_length(_buf: &[u8]) -> Result<usize, RespError> {
        Ok(4)
    }
}

// $<length>\r\n<data>\r\n
impl RespDecode for BulkString {
    const PREFIX: &'static str = "$";
    fn decode(buf: &mut BytesMut) -> Result<Self, RespError> {
        let (end, len) = parse_length(buf, Self::PREFIX)?;
        let remained = &buf[end + CRLF_LEN..];
        if remained.len() < len + CRLF_LEN {
            return Err(RespError::NotComplete);
        }
        buf.advance(end + CRLF_LEN);
        let data = buf.split_to(len + CRLF_LEN);

        Ok(BulkString::new(data[..len].to_vec()))
    }

    fn expect_length(buf: &[u8]) -> Result<usize, RespError> {
        let (end, len) = parse_length(buf, Self::PREFIX)?;
        Ok(end + CRLF_LEN + len + CRLF_LEN)
    }
}

// Arrays: *<number-of-elements>\r\n<element-1>...<element-n>
impl RespDecode for RespArray {
    const PREFIX: &'static str = "*";
    fn decode(buf: &mut BytesMut) -> Result<Self, RespError> {
        let (end, len) = parse_length(buf, Self::PREFIX)?;
        let total_len = calc_total_length(buf, end, len, Self::PREFIX)?;
        if buf.len() < total_len {
            return Err(RespError::NotComplete);
        }
        buf.advance(end + CRLF_LEN);
        let mut frames = Vec::with_capacity(len);
        for _ in 0..len {
            frames.push(RespFrame::decode(buf)?);
        }

        Ok(RespArray::new(frames))
    }

    fn expect_length(buf: &[u8]) -> Result<usize, RespError> {
        let (end, len) = parse_length(buf, Self::PREFIX)?;
        calc_total_length(buf, end, len, Self::PREFIX)
    }
}

// Doubles: ,[<+|->]<integral>[.<fractional>][<E|e>[sign]<exponent>]\r\n
impl RespDecode for f64 {
    const PREFIX: &'static str = ",";
    fn decode(buf: &mut BytesMut) -> Result<Self, RespError> {
        let end = extract_simple_frame_data(buf, Self::PREFIX)?;
        let data = buf.split_to(end + CRLF_LEN);
        let s = String::from_utf8_lossy(&data[Self::PREFIX.len()..end]);
        Ok(s.parse()?)
    }

    fn expect_length(buf: &[u8]) -> Result<usize, RespError> {
        let end = extract_simple_frame_data(buf, Self::PREFIX)?;
        Ok(end + CRLF_LEN)
    }
}

// Maps: %<number-of-entries>\r\n<key-1><value-1>...<key-n><value-n>
impl RespDecode for RespMap {
    const PREFIX: &'static str = "%";
    fn decode(buf: &mut BytesMut) -> Result<Self, RespError> {
        let (end, len) = parse_length(buf, Self::PREFIX)?;
        let total_len = calc_total_length(buf, end, len, Self::PREFIX)?;
        if buf.len() < total_len {
            return Err(RespError::NotComplete);
        }
        buf.advance(end + CRLF_LEN);
        let mut frames = RespMap::new();
        for _ in 0..len {
            let key = SimpleString::decode(buf)?;
            let value = RespFrame::decode(buf)?;
            frames.insert(key.0, value);
        }

        Ok(frames)
    }

    fn expect_length(buf: &[u8]) -> Result<usize, RespError> {
        let (end, len) = parse_length(buf, Self::PREFIX)?;
        calc_total_length(buf, end, len, Self::PREFIX)
    }
}

// ~<number-of-elements>\r\n<element-1>...<element-n>
impl RespDecode for RespSet {
    const PREFIX: &'static str = "~";
    fn decode(buf: &mut BytesMut) -> Result<Self, RespError> {
        let (end, len) = parse_length(buf, Self::PREFIX)?;
        let total_length = calc_total_length(buf, end, len, Self::PREFIX)?;
        if buf.len() < total_length {
            return Err(RespError::NotComplete);
        }
        buf.advance(end + CRLF_LEN);
        let mut frames = Vec::new();
        for _ in 0..len {
            frames.push(RespFrame::decode(buf)?);
        }
        Ok(RespSet::new(frames))
    }

    fn expect_length(buf: &[u8]) -> Result<usize, RespError> {
        let (end, len) = parse_length(buf, Self::PREFIX)?;
        calc_total_length(buf, end, len, Self::PREFIX)
    }
}

fn extract_fixed_data(
    buf: &mut BytesMut,
    expect: &str,
    expect_type: &str,
) -> Result<(), RespError> {
    if buf.len() < expect.len() {
        return Err(RespError::NotComplete);
    }
    if !buf.starts_with(expect.as_bytes()) {
        return Err(RespError::InvalidFrameType(format!(
            "expect: {}, got: {:?}",
            expect_type, buf
        )));
    }
    buf.advance(expect.len());
    Ok(())
}

/// Extracts the data of a simple RESP frame from the buffer.
///
/// # Parameters
///
/// * `buf`: A byte slice containing the RESP data.
/// * `prefix`: The prefix of the RESP frame.
///
/// # Returns
///
/// * `Result<usize, RespError>`: The index of the end of the extracted data, or an error if the data is incomplete.
///
/// # Errors
///
/// * `RespError::NotComplete`: If the data is incomplete.
/// * `RespError::InvalidFrameType`: If the frame type is invalid.
fn extract_simple_frame_data(buf: &[u8], prefix: &str) -> Result<usize, RespError> {
    if buf.len() < 3 {
        return Err(RespError::NotComplete);
    }
    if !buf.starts_with(prefix.as_bytes()) {
        return Err(RespError::InvalidFrameType(format!(
            "expect: SimpleString({}), got: {:?}",
            prefix, buf
        )));
    }
    let end = find_crlf(buf, 1).ok_or(RespError::NotComplete)?;
    Ok(end)
}

// find the nth CRLF in the buffer
fn find_crlf(buf: &[u8], nth: usize) -> Option<usize> {
    let mut count = 0;
    for i in 1..buf.len() - 1 {
        if buf[i] == b'\r' && buf[i + 1] == b'\n' {
            count += 1;
            if count == nth {
                return Some(i);
            }
        }
    }
    None
}

fn parse_length(buf: &[u8], prefix: &str) -> Result<(usize, usize), RespError> {
    let end = extract_simple_frame_data(buf, prefix)?;
    let s = String::from_utf8_lossy(&buf[prefix.len()..end]);

    Ok((end, s.parse()?))
}

/// Calculates the total length of a RESP data structure based on its prefix and length.
///
/// # Parameters
///
/// * `buf`: A byte slice containing the RESP data structure.
/// * `end`: The index of the end of the length prefix in the buffer.
/// * `len`: The length of the RESP data structure.
/// * `prefix`: The prefix of the RESP data structure.
///
/// # Returns
///
/// * `Result<usize, RespError>`: The total length of the RESP data structure, or an error if the data structure is incomplete.
///
/// # Errors
///
/// * `RespError::NotComplete`: If the data structure is incomplete.
fn calc_total_length(buf: &[u8], end: usize, len: usize, prefix: &str) -> Result<usize, RespError> {
    let mut total = end + CRLF_LEN;
    let mut data = &buf[total..];
    match prefix {
        "*" | "~" => {
            // find nth CRLF in the buffer, for array and set, we need to find 1 CRLF for each element
            for _ in 0..len {
                let element_len = RespFrame::expect_length(data)?;
                data = &data[element_len..];
                total += element_len;
            }
            Ok(total)
        }
        "%" => {
            // find nth CRLF in the buffer, for map, we need to find 2 CRLF for each key-value pair
            for _ in 0..len {
                let key_len = SimpleString::expect_length(data)?;
                data = &data[key_len..];
                total += key_len;

                let value_len = RespFrame::expect_length(data)?;
                data = &data[value_len..];
                total += value_len;
            }
            Ok(total)
        }
        _ => Ok(len + CRLF_LEN),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use bytes::BufMut;

    #[test]
    fn test_simple_string_decode() -> Result<()> {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(b"+OK\r\n");
        let res = SimpleString::decode(&mut buf)?;
        assert_eq!(res, SimpleString::new("OK".to_string()));

        buf.extend_from_slice(b"+Hello World\r\n");
        let res = SimpleString::decode(&mut buf)?;
        assert_eq!(res, SimpleString::new("Hello World".to_string()));

        buf.extend_from_slice(b"+\r\n");
        let res = SimpleString::decode(&mut buf)?;
        assert_eq!(res, SimpleString::new("".to_string()));

        buf.extend_from_slice(b"+hello\r");
        let res = SimpleString::decode(&mut buf);
        assert_eq!(res.unwrap_err(), RespError::NotComplete);

        buf.put_u8(b'\n');
        let res = SimpleString::decode(&mut buf)?;
        assert_eq!(res, SimpleString::new("hello".to_string()));
        Ok(())
    }

    #[test]
    fn test_simple_error_decode() -> Result<()> {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(b"-Error message\r\n");
        let res = SimpleError::decode(&mut buf)?;
        assert_eq!(res, SimpleError::new("Error message".to_string()));

        buf.extend_from_slice(b"-Error\r");
        let res = SimpleError::decode(&mut buf);
        assert_eq!(res.unwrap_err(), RespError::NotComplete);

        buf.put_u8(b'\n');
        let res = SimpleError::decode(&mut buf)?;
        assert_eq!(res, SimpleError::new("Error".to_string()));
        Ok(())
    }

    #[test]
    fn test_integer_decode() -> Result<()> {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(b":+123\r\n");
        let frame = i64::decode(&mut buf)?;
        assert_eq!(frame, 123);
        buf.extend_from_slice(b":-123\r\n");
        let frame = i64::decode(&mut buf)?;
        assert_eq!(frame, -123);
        Ok(())
    }

    #[test]
    fn test_bulk_string_decode() -> Result<()> {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(b"$5\r\nhello\r\n");
        let frame = BulkString::decode(&mut buf)?;
        assert_eq!(frame, BulkString::new(b"hello"));
        buf.extend_from_slice(b"$5\r\nhello");
        let ret = BulkString::decode(&mut buf);
        assert_eq!(ret.unwrap_err(), RespError::NotComplete);
        buf.extend_from_slice(b"\r\n");
        let frame = BulkString::decode(&mut buf)?;
        assert_eq!(frame, BulkString::new(b"hello"));
        Ok(())
    }

    #[test]
    fn test_null_array_decode() -> Result<()> {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(b"*-1\r\n");
        let frame = RespNullArray::decode(&mut buf)?;
        assert_eq!(frame, RespNullArray);
        Ok(())
    }

    #[test]
    fn test_null_decode() -> Result<()> {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(b"_\r\n");
        let frame = RespNull::decode(&mut buf)?;
        assert_eq!(frame, RespNull);
        Ok(())
    }

    #[test]
    fn test_boolean_decode() -> Result<()> {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(b"#t\r\n");
        let frame = bool::decode(&mut buf)?;
        assert!(frame);
        buf.extend_from_slice(b"#f\r\n");
        let frame = bool::decode(&mut buf)?;
        assert!(!frame);
        buf.extend_from_slice(b"#f\r");
        let ret = bool::decode(&mut buf);
        assert_eq!(ret.unwrap_err(), RespError::NotComplete);
        buf.put_u8(b'\n');
        let frame = bool::decode(&mut buf)?;
        assert!(!frame);
        Ok(())
    }

    #[test]
    fn test_null_bulk_string_decode() -> Result<()> {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(b"$-1\r\n");
        let frame = RespNullBulkString::decode(&mut buf)?;
        assert_eq!(frame, RespNullBulkString);
        Ok(())
    }

    #[test]
    fn test_array_decode() -> Result<()> {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(b"*2\r\n$3\r\nset\r\n$5\r\nhello\r\n");

        let frame = RespArray::decode(&mut buf)?;
        assert_eq!(frame, RespArray::new([b"set".into(), b"hello".into()]));

        buf.extend_from_slice(b"*2\r\n$3\r\nset\r\n");
        let ret = RespArray::decode(&mut buf);
        assert_eq!(ret.unwrap_err(), RespError::NotComplete);
        buf.extend_from_slice(b"$5\r\nhello\r\n");
        let frame = RespArray::decode(&mut buf)?;
        assert_eq!(frame, RespArray::new([b"set".into(), b"hello".into()]));
        Ok(())
    }

    #[test]
    fn test_double_decode() -> Result<()> {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(b",123.45\r\n");

        let frame = f64::decode(&mut buf)?;
        assert_eq!(frame, 123.45);

        buf.extend_from_slice(b",+1.23456e-9\r\n");
        let frame = f64::decode(&mut buf)?;
        assert_eq!(frame, 1.23456e-9);

        buf.extend_from_slice(b",+inf\r\n");
        let frame = f64::decode(&mut buf)?;
        assert_eq!(frame, f64::INFINITY);
        Ok(())
    }

    #[test]
    fn test_map_decode() -> Result<()> {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(b"%2\r\n+hello\r\n$5\r\nworld\r\n+foo\r\n$3\r\nbar\r\n");
        let frame = RespMap::decode(&mut buf)?;
        let mut map = RespMap::new();
        map.insert(
            "hello".to_string(),
            BulkString::new(b"world".to_vec()).into(),
        );
        map.insert("foo".to_string(), BulkString::new(b"bar".to_vec()).into());
        assert_eq!(frame, map);
        Ok(())
    }

    #[test]
    fn test_set_decode() -> Result<()> {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(b"~2\r\n$3\r\nset\r\n$5\r\nhello\r\n");
        let frame = RespSet::decode(&mut buf)?;
        assert_eq!(
            frame,
            RespSet::new(vec![
                BulkString::new(b"set".to_vec()).into(),
                BulkString::new(b"hello".to_vec()).into()
            ])
        );
        Ok(())
    }

    #[test]
    fn test_calc_array_length() -> Result<()> {
        let buf = b"*2\r\n$3\r\nset\r\n$5\r\nhello\r\n";
        let (end, len) = parse_length(buf, "*")?;
        let total_len = calc_total_length(buf, end, len, "*")?;
        assert_eq!(total_len, buf.len());

        let buf = b"*2\r\n$3\r\nset\r\n";
        let (end, len) = parse_length(buf, "*")?;
        let ret = calc_total_length(buf, end, len, "*");
        assert_eq!(ret.unwrap_err(), RespError::NotComplete);
        Ok(())
    }

    #[test]
    fn test_array_decode_with_elements() -> Result<()> {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(b"*2\r\n$3\r\nset\r\n$5\r\nhello\r\n");
        let frame = RespArray::decode(&mut buf)?;
        assert_eq!(
            frame,
            RespArray::new(vec![
                BulkString::new(b"set".to_vec()).into(),
                BulkString::new(b"hello".to_vec()).into()
            ])
        );
        Ok(())
    }

    #[test]
    fn test_map_decode_with_empty_key() -> Result<()> {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(b"%2\r\n+\r\n$5\r\nworld\r\n+foo\r\n$3\r\nbar\r\n");
        let frame = RespMap::decode(&mut buf)?;
        let mut map = RespMap::new();
        map.insert("".to_string(), BulkString::new(b"world".to_vec()).into());
        map.insert("foo".to_string(), BulkString::new(b"bar".to_vec()).into());
        assert_eq!(frame, map);
        Ok(())
    }
}
