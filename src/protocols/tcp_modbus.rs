use super::ModbusProtocol;
use crate::ModbusError;

/// TCP MODBUS protocol implementation
///
/// TCP MODBUS has a header known as the MODBUS Application Protocol header (MBAP). It includes a
/// length field that can be used to easily separate Application Data Units (ADUs) from each other.
/// However, the length field includes everything after itself, including the function code (not
/// part of the MBAP) and the unit identifier field (which is part of the MBAP). This means that
/// the MBAP length (7 bytes) and the excluded length (6 bytes) are different values.
///
/// Immediately after the MBAP, the protocol data unit (PDU) begins, starting with the function
/// code.
///
/// Visually, a TCP MODBUS ADU looks like this:
///
/// <table>
///   <tr>
///     <th>Offset</th>
///     <th>Field</th>
///     <th>Section</th>
///     <th>Included in length?</th>
///   </tr>
///   <tr>
///     <td>0</td>
///     <td rowspan="2" style="vertical-align:middle">Transaction ID</td>
///     <td rowspan="7" style="vertical-align:middle">MBAP</td>
///     <td rowspan="6" style="vertical-align:middle">No</td>
///   </tr>
///   <tr>
///     <td>1</td>
///   </tr>
///   <tr>
///     <td>2</td>
///     <td rowspan="2" style="vertical-align:middle">Protocol ID</td>
///   </tr>
///   <tr>
///     <td>3</td>
///   </tr>
///   <tr>
///     <td>4</td>
///     <td rowspan="2" style="vertical-align:middle">Length</td>
///   </tr>
///   <tr>
///     <td>5</td>
///   </tr>
///   <tr>
///     <td>6</td>
///     <td>Unit ID</td>
///     <td rowspan="3" style="vertical-align:middle">Yes</td>
///   </tr>
///   <tr>
///     <td>7</td>
///     <td>Function Code</td>
///     <td rowspan="2" style="vertical-align:middle">PDU</td>
///   </tr>
///   <tr>
///     <td>8...</td>
///     <td>Continuing PDU Data</td>
///   </tr>
/// </table>
///
/// This has some implications for implementing `ModbusProtocol` for TCP.
/// - `Header` includes all of the items in the MBAP, including the unit ID, but not the function
///   code.
/// - `adu_length` returns the length field + 6, because the length field already includes the unit
///   ID.
/// - `pdu_body` returns PDU data starting at index 7. If you want the unit ID, you need to get it
///   with `adu_header`.
pub struct TcpModbus;

// Length of the MODBUS Application Protocol header
// 2-byte transaction ID, 2-byte protocol ID, 2-byte length, 1-byte unit ID
const MBAP_LENGTH: usize = 7;

// Number of APU bytes excluded from the length field
// This is slightly different from the MBAP length because the 1-byte unit ID is
// included in the MBAP but falls after the length field, and thus excluded from
// the length field
const EXCLUDED_LENGTH: usize = 6;

/// TCP MODBUS header data
#[derive(Debug, Clone)]
pub struct TcpModbusHeader {
    pub transaction_id: u16,
    pub protocol_id: u16,
    pub length: u16,
    pub unit_id: u8,
}

impl TcpModbus {
    fn protocol_id(data: &[u8]) -> Option<u16> {
        Some(u16::from_be_bytes([*data.get(2)?, *data.get(3)?]))
    }

    fn transaction_id(data: &[u8]) -> Option<u16> {
        Some(u16::from_be_bytes([*data.get(0)?, *data.get(1)?]))
    }

    fn length(data: &[u8]) -> Option<u16> {
        Some(u16::from_be_bytes([*data.get(4)?, *data.get(5)?]))
    }

    fn unit_id(data: &[u8]) -> Option<u8> {
        data.get(6).map(|&x| x)
    }
}

impl ModbusProtocol for TcpModbus {
    const ADU_MAX_LENGTH: usize = 260;

    type Header = TcpModbusHeader;

    fn adu_length(data: &[u8]) -> Result<usize, ModbusError> {
        match Self::length(data) {
            None => Err(ModbusError::NotEnoughData),
            Some(v) => Ok(v as usize + MBAP_LENGTH),
        }
    }

    fn adu_header(data: &[u8]) -> Result<Self::Header, ModbusError> {
        use ModbusError::NotEnoughData;

        Ok(Self::Header {
            transaction_id: Self::transaction_id(data).ok_or(NotEnoughData)?,
            protocol_id: Self::protocol_id(data).ok_or(NotEnoughData)?,
            length: Self::length(data).ok_or(NotEnoughData)?,
            unit_id: Self::unit_id(data).ok_or(NotEnoughData)?,
        })
    }

    /// TCP MODBUS doesn't have checksums, so this just confirms that there's
    /// enough data to make up a whole ADU
    fn adu_check(data: &[u8]) -> Result<(), ModbusError> {
        use ModbusError::NotEnoughData;

        let length = Self::adu_length(data)?;

        if data.len() > length {
            Ok(())
        } else {
            Err(NotEnoughData)
        }
    }

    fn pdu_body(data: &[u8]) -> Result<&[u8], ModbusError> {
        Self::adu_check(data)?;

        // We just checked that the length is correct in adu_check, so this
        // won't panic
        Ok(&data[MBAP_LENGTH..])
    }
}
