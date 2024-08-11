use std::num::NonZeroUsize;

use winnow::{
    ascii::{digit1, float},
    combinator::{alt, dispatch, fail, opt, preceded, terminated},
    error::{ContextError, ErrMode, Needed},
    token::{any, take, take_until},
    PResult, Parser,
};

use crate::{
    BulkString, RespArray, RespError, RespFrame, RespMap, RespNull, RespNullArray,
    RespNullBulkString, SimpleError, SimpleString,
};

const CRLF: &[u8] = b"\r\n";

pub fn parse_frame_length(input: &[u8]) -> Result<usize, RespError> {
    let target = &mut (&*input);
    let ret = parse_frame_len(target);
    match ret {
        Ok(_) => {
            let start = input.as_ptr();
            let end = (*target).as_ptr();
            let len = end as usize - start as usize;
            Ok(len)
        }
        Err(_) => Err(RespError::NotComplete),
    }
}

fn parse_frame_len(input: &mut &[u8]) -> PResult<()> {
    let mut simple_parser = terminated(take_until(0.., CRLF), CRLF).value(());
    dispatch! {any;
        b'+' => simple_parser,
        b'-' => simple_parser,
        b':' => simple_parser,
        b'$' => bulk_string_len,
        b'*' => array_len,
        b'_' => simple_parser,
        b'#' => simple_parser,
        b',' => simple_parser,
        b'%' => map_len,
        _v => fail::<_, _, _>
    }
    .parse_next(input)
}

pub fn parse_frame(input: &mut &[u8]) -> PResult<RespFrame> {
    dispatch! {any;
        b'+' => simple_string.map(RespFrame::SimpleString),
        b'-' => error.map(RespFrame::Error),
        b':' => integer.map(RespFrame::Integer),
        b'$' => alt((null_bulk_string.map(RespFrame::NullBulkString), bulk_string.map(RespFrame::BulkString))),
        b'*' => alt((null_array.map(RespFrame::NullArray), array.map(RespFrame::Array))),
        b'_' => null.map(RespFrame::Null),
        b'#' => boolean.map(RespFrame::Boolean),
        b',' => decimal.map(RespFrame::Double),
        b'%' => map.map(RespFrame::Map),
        _v => fail::<_, _, _>

    }
    .parse_next(input)
}

// - simple string: "OK\r\n"
fn simple_string(input: &mut &[u8]) -> PResult<SimpleString> {
    parse_string(input).map(SimpleString)
}

fn error(input: &mut &[u8]) -> PResult<SimpleError> {
    parse_string(input).map(SimpleError)
}

fn integer(input: &mut &[u8]) -> PResult<i64> {
    let sign = opt(alt(('+', '-'))).parse_next(input)?.unwrap_or('+');
    let sign = if sign == '+' { 1 } else { -1 };
    let v: i64 = terminated(digit1.parse_to(), CRLF).parse_next(input)?;
    Ok(sign * v)
}

// - null bulk string: "$-1\r\n"
fn null_bulk_string(input: &mut &[u8]) -> PResult<RespNullBulkString> {
    "-1\r\n".value(RespNullBulkString).parse_next(input)
}

// - bulk string: "$<length>\r\n<data>\r\n"
#[allow(clippy::comparison_chain)]
fn bulk_string(input: &mut &[u8]) -> PResult<BulkString> {
    let len = integer(input)?;
    if len == 0 {
        return Ok(BulkString::new(vec![]));
    } else if len < 0 {
        return Err(err_cur("Invalid length"));
    }
    let data = terminated(take(len as usize), CRLF).parse_next(input)?;
    Ok(BulkString::new(data.to_vec()))
}

fn bulk_string_len(input: &mut &[u8]) -> PResult<()> {
    let len = integer(input)?;
    if len == -1 || len == 0 {
        return Ok(());
    } else if len < -1 {
        return Err(err_cur("Invalid length"));
    }
    let len_with_crlf = len as usize + 2;
    if input.len() < len_with_crlf {
        let size = NonZeroUsize::new((len_with_crlf - input.len()) as usize).unwrap();
        return Err(ErrMode::Incomplete(Needed::Size(size)));
    }
    *input = &input[(len + 2) as usize..];
    Ok(())
    /* terminated(take(len as usize), CRLF)
    .value(())
    .parse_next(input) */
}

fn null_array(input: &mut &[u8]) -> PResult<RespNullArray> {
    "-1\r\n".value(RespNullArray).parse_next(input)
}

// "*2\r\n$3\r\nget\r\n$5\r\nhello\r\n"
#[allow(clippy::comparison_chain)]
fn array(input: &mut &[u8]) -> PResult<RespArray> {
    let len = integer(input)?;
    if len == 0 {
        return Ok(RespArray::new(vec![]));
    } else if len < 0 {
        return Err(err_cur("Invalid length"));
    }

    let mut arr = Vec::with_capacity(len as usize);
    for _ in 0..len {
        arr.push(parse_frame(input)?);
    }
    Ok(RespArray::new(arr))
}

fn array_len(input: &mut &[u8]) -> PResult<()> {
    let len = integer(input)?;
    if len == 0 || len == -1 {
        return Ok(());
    } else if len < -1 {
        return Err(err_cur("Invalid length"));
    }

    for _ in 0..len {
        parse_frame_len(input)?;
    }
    Ok(())
}

// - boolean: "#<t|f>\r\n"
fn boolean(input: &mut &[u8]) -> PResult<bool> {
    let b = terminated(alt(('t', 'f')), CRLF).parse_next(input)?;
    Ok(b == 't')
}

// - double: ",[<+|->]<integral>[.<fractional>][<E|e>[sign]<exponent>]\r\n"
fn decimal(input: &mut &[u8]) -> PResult<f64> {
    terminated(float, CRLF).parse_next(input)
}

// - map: %2\r\n+key1\r\n$6\r\nvalue1\r\n+key2\r\n$6\r\nvalue2\r\n
fn map(input: &mut &[u8]) -> PResult<RespMap> {
    let len = integer(input)?;
    if len <= 0 {
        return Err(err_cur("Invalid length"));
    }

    let len = len / 2;
    let mut map = RespMap::new();
    for _ in 0..len {
        let key = preceded('+', parse_string).parse_next(input)?;
        let value = parse_frame(input)?;
        map.insert(key, value);
    }
    Ok(map)
}

fn map_len(input: &mut &[u8]) -> PResult<()> {
    let len = integer(input)?;
    if len <= 0 {
        return Err(err_cur("Invalid length"));
    }

    let len = len / 2;
    for _ in 0..len {
        terminated(take_until(0.., CRLF), CRLF)
            .value(())
            .parse_next(input)?;
        parse_frame_len(input)?;
    }
    Ok(())
}

// null: "_\r\n"
fn null(input: &mut &[u8]) -> PResult<RespNull> {
    "\r\n".value(RespNull).parse_next(input)
}

fn parse_string(input: &mut &[u8]) -> PResult<String> {
    terminated(take_until(0.., CRLF), CRLF)
        .map(|s: &[u8]| String::from_utf8_lossy(s).to_string())
        .parse_next(input)
}

fn err_cur(_s: impl Into<String>) -> ErrMode<ContextError> {
    let context = ContextError::default();
    ErrMode::Cut(context)
}
