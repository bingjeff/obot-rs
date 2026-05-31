use core::fmt::Write;

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

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ApiValue<'a> {
    Bool(bool),
    U8(u8),
    U16(u16),
    U32(u32),
    I32(i32),
    Fixed3(i64),
    #[cfg(not(target_os = "none"))]
    F32(f32),
    Str(&'a str),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ApiEntry<'a> {
    pub name: &'a str,
    pub value: ApiValue<'a>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApiDispatchError {
    Parse(ApiParseError),
    UnknownName,
    ReadOnly,
    NameIndexOutOfRange,
    ResponseTooLong,
}

pub struct ApiCatalog<'a> {
    entries: &'a [ApiEntry<'a>],
}

impl<'a> ApiCatalog<'a> {
    pub const fn new(entries: &'a [ApiEntry<'a>]) -> Self {
        Self { entries }
    }

    pub const fn len(&self) -> usize {
        self.entries.len()
    }

    pub const fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn dispatch<'out>(
        &self,
        input: &str,
        output: &'out mut [u8],
    ) -> Result<&'out str, ApiDispatchError> {
        match parse_request(input).map_err(ApiDispatchError::Parse)? {
            ApiRequest::Get { name } => self.read(name, output),
            ApiRequest::Set { name, .. } => {
                if self.find(name).is_some() {
                    Err(ApiDispatchError::ReadOnly)
                } else {
                    Err(ApiDispatchError::UnknownName)
                }
            }
            ApiRequest::NameAt { index } => self.name_at(index, output),
        }
    }

    pub fn read<'out>(
        &self,
        name: &str,
        output: &'out mut [u8],
    ) -> Result<&'out str, ApiDispatchError> {
        let entry = self.find(name).ok_or(ApiDispatchError::UnknownName)?;
        format_value(entry.value, output)
    }

    pub fn name_at<'out>(
        &self,
        index: u16,
        output: &'out mut [u8],
    ) -> Result<&'out str, ApiDispatchError> {
        let entry = self
            .entries
            .get(index as usize)
            .ok_or(ApiDispatchError::NameIndexOutOfRange)?;
        let mut writer = ResponseWriter::new(output);
        writer
            .write_str(entry.name)
            .map_err(|_| ApiDispatchError::ResponseTooLong)?;
        writer.finish()
    }

    fn find(&self, name: &str) -> Option<ApiEntry<'a>> {
        self.entries
            .iter()
            .copied()
            .find(|entry| entry.name == name)
    }
}

impl<'a> ApiEntry<'a> {
    pub const fn new(name: &'a str, value: ApiValue<'a>) -> Self {
        Self { name, value }
    }
}

pub fn format_value<'out>(
    value: ApiValue<'_>,
    output: &'out mut [u8],
) -> Result<&'out str, ApiDispatchError> {
    let mut writer = ResponseWriter::new(output);
    write_value(&mut writer, value).map_err(|_| ApiDispatchError::ResponseTooLong)?;
    writer.finish()
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

struct ResponseWriter<'a> {
    bytes: &'a mut [u8],
    len: usize,
}

impl<'a> ResponseWriter<'a> {
    fn new(bytes: &'a mut [u8]) -> Self {
        Self { bytes, len: 0 }
    }

    fn finish(self) -> Result<&'a str, ApiDispatchError> {
        core::str::from_utf8(&self.bytes[..self.len]).map_err(|_| ApiDispatchError::ResponseTooLong)
    }
}

impl core::fmt::Write for ResponseWriter<'_> {
    fn write_str(&mut self, value: &str) -> core::fmt::Result {
        let remaining = self.bytes.len().saturating_sub(self.len);
        if value.len() > remaining {
            return Err(core::fmt::Error);
        }

        let end = self.len + value.len();
        self.bytes[self.len..end].copy_from_slice(value.as_bytes());
        self.len = end;
        Ok(())
    }
}

fn write_value(writer: &mut ResponseWriter<'_>, value: ApiValue<'_>) -> core::fmt::Result {
    use core::fmt::Write;

    match value {
        ApiValue::Bool(value) => write!(writer, "{}", value),
        ApiValue::U8(value) => write!(writer, "{}", value),
        ApiValue::U16(value) => write!(writer, "{}", value),
        ApiValue::U32(value) => write!(writer, "{}", value),
        ApiValue::I32(value) => write!(writer, "{}", value),
        ApiValue::Fixed3(value) => write_fixed3(writer, value),
        #[cfg(not(target_os = "none"))]
        ApiValue::F32(value) => write!(writer, "{}", value),
        ApiValue::Str(value) => writer.write_str(value),
    }
}

fn write_fixed3(writer: &mut ResponseWriter<'_>, value: i64) -> core::fmt::Result {
    use core::fmt::Write;

    let magnitude = value.unsigned_abs();
    let whole = magnitude / 1_000;
    let mut fraction = magnitude % 1_000;

    if value < 0 {
        writer.write_str("-")?;
    }
    write!(writer, "{}", whole)?;
    if fraction == 0 {
        return Ok(());
    }

    writer.write_str(".")?;
    let hundreds = fraction / 100;
    fraction %= 100;
    let tens = fraction / 10;
    let ones = fraction % 10;
    write!(writer, "{}", hundreds)?;
    if tens != 0 || ones != 0 {
        write!(writer, "{}", tens)?;
    }
    if ones != 0 {
        write!(writer, "{}", ones)?;
    }
    Ok(())
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

    #[test]
    fn dispatches_named_reads_from_fixed_catalog() {
        let entries = [
            ApiEntry::new("ready", ApiValue::Bool(true)),
            ApiEntry::new("bus_voltage_raw", ApiValue::U16(1963)),
            ApiEntry::new("mean_fast_loop_cycles", ApiValue::F32(435.708)),
            ApiEntry::new("fault", ApiValue::Str("none")),
        ];
        let catalog = ApiCatalog::new(&entries);
        let mut output = [0; 32];

        assert_eq!(catalog.dispatch("ready", &mut output), Ok("true"));
        assert_eq!(catalog.dispatch("bus_voltage_raw", &mut output), Ok("1963"));
        assert_eq!(
            catalog.dispatch("mean_fast_loop_cycles", &mut output),
            Ok("435.708")
        );
        assert_eq!(catalog.dispatch("fault", &mut output), Ok("none"));
    }

    #[test]
    fn dispatches_fixed3_without_unnecessary_trailing_zeroes() {
        let entries = [
            ApiEntry::new("positive", ApiValue::Fixed3(435_708)),
            ApiEntry::new("whole", ApiValue::Fixed3(17_000_000)),
            ApiEntry::new("negative", ApiValue::Fixed3(-3_421_250)),
        ];
        let catalog = ApiCatalog::new(&entries);
        let mut output = [0; 32];

        assert_eq!(catalog.dispatch("positive", &mut output), Ok("435.708"));
        assert_eq!(catalog.dispatch("whole", &mut output), Ok("17000"));
        assert_eq!(catalog.dispatch("negative", &mut output), Ok("-3421.25"));
    }

    #[test]
    fn dispatches_api_name_lookup_from_fixed_catalog_order() {
        let entries = [
            ApiEntry::new("max_fast_loop_cycles", ApiValue::U32(836)),
            ApiEntry::new("bus_voltage_raw", ApiValue::U16(2)),
        ];
        let catalog = ApiCatalog::new(&entries);
        let mut output = [0; 32];

        assert_eq!(
            catalog.dispatch("api_name=0", &mut output),
            Ok("max_fast_loop_cycles")
        );
        assert_eq!(
            catalog.dispatch("api_name=1", &mut output),
            Ok("bus_voltage_raw")
        );
        assert_eq!(
            catalog.dispatch("api_name=2", &mut output),
            Err(ApiDispatchError::NameIndexOutOfRange)
        );
    }

    #[test]
    fn rejects_unknown_and_read_only_dispatches() {
        let entries = [ApiEntry::new("kp", ApiValue::F32(100.0))];
        let catalog = ApiCatalog::new(&entries);
        let mut output = [0; 32];

        assert_eq!(
            catalog.dispatch("missing", &mut output),
            Err(ApiDispatchError::UnknownName)
        );
        assert_eq!(
            catalog.dispatch("kp=99", &mut output),
            Err(ApiDispatchError::ReadOnly)
        );
    }

    #[test]
    fn rejects_responses_that_do_not_fit_output_buffer() {
        let entries = [ApiEntry::new("long", ApiValue::Str("abcdef"))];
        let catalog = ApiCatalog::new(&entries);
        let mut output = [0; 3];

        assert_eq!(
            catalog.dispatch("long", &mut output),
            Err(ApiDispatchError::ResponseTooLong)
        );
    }
}
