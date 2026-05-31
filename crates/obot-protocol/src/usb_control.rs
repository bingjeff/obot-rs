pub const SETUP_PACKET_LEN: usize = 8;
pub const BULK_MAX_PACKET_SIZE: u8 = 64;
pub const VENDOR_ID: u16 = 0x3293;
pub const PRODUCT_ID: u16 = 0x0100;

const DESCRIPTOR_TYPE_DEVICE: u8 = 1;
const DESCRIPTOR_TYPE_CONFIGURATION: u8 = 2;
const DESCRIPTOR_TYPE_STRING: u8 = 3;
const DESCRIPTOR_TYPE_INTERFACE: u8 = 4;
const DESCRIPTOR_TYPE_ENDPOINT: u8 = 5;

const STRING_LANGID: u8 = 0;
const STRING_MANUFACTURER: u8 = 1;
const STRING_PRODUCT: u8 = 2;
const STRING_SERIAL: u8 = 3;
const STRING_CONFIGURATION: u8 = 4;
const STRING_INTERFACE: u8 = 5;
const STRING_DFU: u8 = 6;

pub const MANUFACTURER_STRING: &str = "Unhuman Inc.";
pub const PRODUCT_STRING: &str = "OBOT motor controller";
pub const DFU_INTERFACE_STRING: &str = "ST DFU mode";

pub const DEVICE_DESCRIPTOR: [u8; 18] = [
    18,
    DESCRIPTOR_TYPE_DEVICE,
    0x00,
    0x02,
    0xFF,
    0x00,
    0x00,
    BULK_MAX_PACKET_SIZE,
    (VENDOR_ID & 0xFF) as u8,
    (VENDOR_ID >> 8) as u8,
    (PRODUCT_ID & 0xFF) as u8,
    (PRODUCT_ID >> 8) as u8,
    0x00,
    0x02,
    STRING_MANUFACTURER,
    STRING_PRODUCT,
    STRING_SERIAL,
    1,
];

pub const CONFIGURATION_DESCRIPTOR: [u8; 64] = [
    9,
    DESCRIPTOR_TYPE_CONFIGURATION,
    64,
    0,
    2,
    1,
    STRING_CONFIGURATION,
    0xC0,
    0x32,
    9,
    DESCRIPTOR_TYPE_INTERFACE,
    0,
    0,
    4,
    0,
    0,
    0,
    STRING_INTERFACE,
    7,
    DESCRIPTOR_TYPE_ENDPOINT,
    0x82,
    0x02,
    BULK_MAX_PACKET_SIZE,
    0,
    0,
    7,
    DESCRIPTOR_TYPE_ENDPOINT,
    0x02,
    0x02,
    BULK_MAX_PACKET_SIZE,
    0,
    0,
    7,
    DESCRIPTOR_TYPE_ENDPOINT,
    0x81,
    0x02,
    BULK_MAX_PACKET_SIZE,
    0,
    0x10,
    7,
    DESCRIPTOR_TYPE_ENDPOINT,
    0x01,
    0x02,
    BULK_MAX_PACKET_SIZE,
    0,
    0x10,
    9,
    DESCRIPTOR_TYPE_INTERFACE,
    1,
    0,
    0,
    0xFE,
    0x01,
    0x01,
    STRING_DFU,
    9,
    0x21,
    0x0B,
    0xFF,
    0,
    0,
    0x08,
    0x1A,
    0x01,
];

pub const LANGID_STRING_DESCRIPTOR: [u8; 4] = [4, DESCRIPTOR_TYPE_STRING, 0x09, 0x04];
pub const GET_STATUS_RESPONSE: [u8; 2] = [0, 0];
pub const DFU_GET_STATUS_RESPONSE: [u8; 6] = [0, 0, 0, 0, 0, 0];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SetupPacket {
    pub request_type: u8,
    pub request: u8,
    pub value: u16,
    pub index: u16,
    pub length: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ControlRequest {
    GetStatus,
    GetDescriptor { descriptor_type: u8, index: u8 },
    SetAddress(u8),
    SetConfiguration(u8),
    SetInterface { interface: u16 },
    DfuGetStatus { interface: u16 },
    DfuDetach { interface: u16 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ControlResponse<'a> {
    Bytes(&'a [u8]),
    StringAscii(&'a str),
    StatusAck,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UsbControlError {
    InvalidSetupLength,
    UnsupportedRequest,
    UnsupportedDescriptor,
    StringDescriptorTooLong,
    OutputBufferTooSmall,
}

impl SetupPacket {
    pub fn decode(input: &[u8]) -> Result<Self, UsbControlError> {
        let bytes: &[u8; SETUP_PACKET_LEN] = input
            .try_into()
            .map_err(|_| UsbControlError::InvalidSetupLength)?;
        Ok(Self {
            request_type: bytes[0],
            request: bytes[1],
            value: u16::from_le_bytes([bytes[2], bytes[3]]),
            index: u16::from_le_bytes([bytes[4], bytes[5]]),
            length: u16::from_le_bytes([bytes[6], bytes[7]]),
        })
    }

    pub const fn control_request(self) -> Result<ControlRequest, UsbControlError> {
        match (self.request_type, self.request) {
            (0x80, 0x00) => Ok(ControlRequest::GetStatus),
            (0x80, 0x06) => Ok(ControlRequest::GetDescriptor {
                descriptor_type: (self.value >> 8) as u8,
                index: self.value as u8,
            }),
            (0x00, 0x05) => Ok(ControlRequest::SetAddress(self.value as u8)),
            (0x00, 0x09) => Ok(ControlRequest::SetConfiguration(self.value as u8)),
            (0x01, 0x0B) => Ok(ControlRequest::SetInterface {
                interface: self.index,
            }),
            (0xA1, 0x03) => Ok(ControlRequest::DfuGetStatus {
                interface: self.index,
            }),
            (0x21, 0x00) => Ok(ControlRequest::DfuDetach {
                interface: self.index,
            }),
            _ => Err(UsbControlError::UnsupportedRequest),
        }
    }
}

pub fn control_response<'a>(
    request: ControlRequest,
    serial: &'a str,
    configuration: &'a str,
    interface: &'a str,
) -> Result<ControlResponse<'a>, UsbControlError> {
    match request {
        ControlRequest::GetStatus => Ok(ControlResponse::Bytes(&GET_STATUS_RESPONSE)),
        ControlRequest::GetDescriptor {
            descriptor_type,
            index,
        } => descriptor_response(descriptor_type, index, serial, configuration, interface),
        ControlRequest::SetAddress(_)
        | ControlRequest::SetConfiguration(_)
        | ControlRequest::SetInterface { .. }
        | ControlRequest::DfuDetach { .. } => Ok(ControlResponse::StatusAck),
        ControlRequest::DfuGetStatus { interface: 1 } => {
            Ok(ControlResponse::Bytes(&DFU_GET_STATUS_RESPONSE))
        }
        ControlRequest::DfuGetStatus { .. } => Err(UsbControlError::UnsupportedRequest),
    }
}

pub fn descriptor_response<'a>(
    descriptor_type: u8,
    index: u8,
    serial: &'a str,
    configuration: &'a str,
    interface: &'a str,
) -> Result<ControlResponse<'a>, UsbControlError> {
    match descriptor_type {
        DESCRIPTOR_TYPE_DEVICE => Ok(ControlResponse::Bytes(&DEVICE_DESCRIPTOR)),
        DESCRIPTOR_TYPE_CONFIGURATION => Ok(ControlResponse::Bytes(&CONFIGURATION_DESCRIPTOR)),
        DESCRIPTOR_TYPE_STRING => {
            string_descriptor_response(index, serial, configuration, interface)
        }
        _ => Err(UsbControlError::UnsupportedDescriptor),
    }
}

fn string_descriptor_response<'a>(
    index: u8,
    serial: &'a str,
    configuration: &'a str,
    interface: &'a str,
) -> Result<ControlResponse<'a>, UsbControlError> {
    match index {
        STRING_LANGID => Ok(ControlResponse::Bytes(&LANGID_STRING_DESCRIPTOR)),
        STRING_MANUFACTURER => Ok(ControlResponse::StringAscii(MANUFACTURER_STRING)),
        STRING_PRODUCT => Ok(ControlResponse::StringAscii(PRODUCT_STRING)),
        STRING_SERIAL => Ok(ControlResponse::StringAscii(serial)),
        STRING_CONFIGURATION => Ok(ControlResponse::StringAscii(configuration)),
        STRING_INTERFACE => Ok(ControlResponse::StringAscii(interface)),
        STRING_DFU => Ok(ControlResponse::StringAscii(DFU_INTERFACE_STRING)),
        _ => Err(UsbControlError::UnsupportedDescriptor),
    }
}

pub fn write_ascii_string_descriptor<'out>(
    value: &str,
    output: &'out mut [u8],
) -> Result<&'out [u8], UsbControlError> {
    let len = value
        .len()
        .checked_mul(2)
        .and_then(|len| len.checked_add(2))
        .ok_or(UsbControlError::StringDescriptorTooLong)?;
    if len > u8::MAX as usize {
        return Err(UsbControlError::StringDescriptorTooLong);
    }
    if output.len() < len {
        return Err(UsbControlError::OutputBufferTooSmall);
    }

    output[0] = len as u8;
    output[1] = DESCRIPTOR_TYPE_STRING;
    for (index, byte) in value.as_bytes().iter().copied().enumerate() {
        output[2 + index * 2] = byte;
        output[3 + index * 2] = 0;
    }
    Ok(&output[..len])
}

pub fn limit_to_setup_length(data: &[u8], setup: SetupPacket) -> &[u8] {
    let len = core::cmp::min(data.len(), setup.length as usize);
    &data[..len]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_setup_packets() {
        let setup = SetupPacket::decode(&[0x80, 0x06, 0, 1, 0, 0, 18, 0]).unwrap();

        assert_eq!(setup.request_type, 0x80);
        assert_eq!(setup.request, 0x06);
        assert_eq!(setup.value, 0x0100);
        assert_eq!(setup.length, 18);
        assert_eq!(
            setup.control_request(),
            Ok(ControlRequest::GetDescriptor {
                descriptor_type: DESCRIPTOR_TYPE_DEVICE,
                index: 0,
            })
        );
    }

    #[test]
    fn exposes_cpp_compatible_device_descriptor() {
        assert_eq!(DEVICE_DESCRIPTOR.len(), 18);
        assert_eq!(DEVICE_DESCRIPTOR[1], DESCRIPTOR_TYPE_DEVICE);
        assert_eq!(
            u16::from_le_bytes([DEVICE_DESCRIPTOR[8], DEVICE_DESCRIPTOR[9]]),
            VENDOR_ID
        );
        assert_eq!(
            u16::from_le_bytes([DEVICE_DESCRIPTOR[10], DEVICE_DESCRIPTOR[11]]),
            PRODUCT_ID
        );
        assert_eq!(DEVICE_DESCRIPTOR[14], STRING_MANUFACTURER);
        assert_eq!(DEVICE_DESCRIPTOR[15], STRING_PRODUCT);
        assert_eq!(DEVICE_DESCRIPTOR[16], STRING_SERIAL);
    }

    #[test]
    fn exposes_cpp_compatible_configuration_descriptor() {
        assert_eq!(CONFIGURATION_DESCRIPTOR.len(), 64);
        assert_eq!(CONFIGURATION_DESCRIPTOR[0], 9);
        assert_eq!(CONFIGURATION_DESCRIPTOR[1], DESCRIPTOR_TYPE_CONFIGURATION);
        assert_eq!(CONFIGURATION_DESCRIPTOR[2], 64);
        assert_eq!(CONFIGURATION_DESCRIPTOR[4], 2);
        assert_eq!(CONFIGURATION_DESCRIPTOR[7], 0xC0);
        assert_eq!(CONFIGURATION_DESCRIPTOR[8], 0x32);
        assert_eq!(
            &CONFIGURATION_DESCRIPTOR[18..25],
            &[7, 5, 0x82, 2, 64, 0, 0]
        );
        assert_eq!(
            &CONFIGURATION_DESCRIPTOR[25..32],
            &[7, 5, 0x02, 2, 64, 0, 0]
        );
        assert_eq!(
            &CONFIGURATION_DESCRIPTOR[32..39],
            &[7, 5, 0x81, 2, 64, 0, 0x10]
        );
        assert_eq!(
            &CONFIGURATION_DESCRIPTOR[39..46],
            &[7, 5, 0x01, 2, 64, 0, 0x10]
        );
    }

    #[test]
    fn maps_control_requests_to_responses() {
        let setup = SetupPacket::decode(&[0x80, 0x06, 0, 2, 0, 0, 64, 0]).unwrap();
        let request = setup.control_request().unwrap();

        assert_eq!(
            control_response(request, "1234", "version", "Jeff"),
            Ok(ControlResponse::Bytes(&CONFIGURATION_DESCRIPTOR))
        );

        let setup = SetupPacket::decode(&[0x80, 0x06, 3, 3, 0, 0, 64, 0]).unwrap();
        let request = setup.control_request().unwrap();
        assert_eq!(
            control_response(request, "1234", "version", "Jeff"),
            Ok(ControlResponse::StringAscii("1234"))
        );
    }

    #[test]
    fn writes_ascii_string_descriptor_as_utf16le() {
        let mut output = [0; 16];
        let descriptor = write_ascii_string_descriptor("J1", &mut output).unwrap();

        assert_eq!(descriptor, &[6, 3, b'J', 0, b'1', 0]);
    }

    #[test]
    fn limits_data_to_setup_length() {
        let setup = SetupPacket::decode(&[0x80, 0x06, 0, 1, 0, 0, 8, 0]).unwrap();

        assert_eq!(limit_to_setup_length(&DEVICE_DESCRIPTOR, setup).len(), 8);
    }
}
