#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApiRequest<'a> {
    Get { name: &'a str },
    Set { name: &'a str, value: &'a str },
    NameAt { index: u16 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApiParseError {
    Empty,
    MissingName,
    MissingValue,
    InvalidIndex,
}

pub fn parse_request(input: &str) -> Result<ApiRequest<'_>, ApiParseError> {
    let input = trim_spaces(input);
    if input.is_empty() {
        return Err(ApiParseError::Empty);
    }

    let Some(equal_index) = find_byte(input, b'=') else {
        return Ok(ApiRequest::Get { name: input });
    };

    let name = trim_spaces(&input[..equal_index]);
    if name.is_empty() {
        return Err(ApiParseError::MissingName);
    }

    let value = trim_spaces(&input[equal_index + 1..]);
    if value.is_empty() {
        return Err(ApiParseError::MissingValue);
    }

    if name == "api_name" {
        return Ok(ApiRequest::NameAt {
            index: parse_u16(value).ok_or(ApiParseError::InvalidIndex)?,
        });
    }

    Ok(ApiRequest::Set { name, value })
}

fn trim_spaces(input: &str) -> &str {
    let bytes = input.as_bytes();
    let mut start = 0;
    let mut end = bytes.len();

    while start < end && bytes[start] == b' ' {
        start += 1;
    }
    while end > start && bytes[end - 1] == b' ' {
        end -= 1;
    }

    &input[start..end]
}

fn find_byte(input: &str, needle: u8) -> Option<usize> {
    input.as_bytes().iter().position(|byte| *byte == needle)
}

fn parse_u16(input: &str) -> Option<u16> {
    let mut value: u16 = 0;
    for byte in input.as_bytes().iter().copied() {
        let digit = byte.wrapping_sub(b'0');
        if digit > 9 {
            return None;
        }
        value = value.checked_mul(10)?.checked_add(digit as u16)?;
    }
    Some(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_variable_get_like_motorlib_api() {
        assert_eq!(parse_request(" kp "), Ok(ApiRequest::Get { name: "kp" }));
    }

    #[test]
    fn parses_variable_set_like_motorlib_api() {
        assert_eq!(
            parse_request("kp = 99"),
            Ok(ApiRequest::Set {
                name: "kp",
                value: "99",
            })
        );
    }

    #[test]
    fn parses_api_name_lookup() {
        assert_eq!(
            parse_request("api_name=12"),
            Ok(ApiRequest::NameAt { index: 12 })
        );
    }

    #[test]
    fn rejects_empty_or_incomplete_commands() {
        assert_eq!(parse_request("   "), Err(ApiParseError::Empty));
        assert_eq!(parse_request("=1"), Err(ApiParseError::MissingName));
        assert_eq!(parse_request("kp="), Err(ApiParseError::MissingValue));
    }

    #[test]
    fn rejects_invalid_api_name_index() {
        assert_eq!(
            parse_request("api_name=not-a-number"),
            Err(ApiParseError::InvalidIndex)
        );
        assert_eq!(
            parse_request("api_name=70000"),
            Err(ApiParseError::InvalidIndex)
        );
    }
}
