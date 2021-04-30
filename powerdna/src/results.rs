use std::error::Error;
use std::fmt::Display;

#[derive(Debug, FromPrimitive)]
#[repr(i32)]
pub enum PowerDnaError {
    IllegalEntry = powerdna_sys::DQ_ILLEGAL_ENTRY,
    IllegalHandle = powerdna_sys::DQ_ILLEGAL_HANDLE,
    SocketError = powerdna_sys::DQ_SOCK_LIB_ERROR,
    TimeoutError = powerdna_sys::DQ_TIMEOUT_ERROR,
    SendingError = powerdna_sys::DQ_SEND_ERROR,
    ReceivingError = powerdna_sys::DQ_RECV_ERROR,
    IomError = powerdna_sys::DQ_IOM_ERROR,
    PacketTooLong = powerdna_sys::DQ_PKT_TOOLONG,
    IllegalPacketSize = powerdna_sys::DQ_ILLEGAL_PKTSIZE,
    InitError = powerdna_sys::DQ_INIT_ERROR,
    BadParameter = powerdna_sys::DQ_BAD_PARAMETER,
    BadDevNumber = powerdna_sys::DQ_BAD_DEVN,
    NotImplemented = powerdna_sys::DQ_NOT_IMPLEMENTED,
    NoMemory = powerdna_sys::DQ_NO_MEMORY,
    PacketFull = powerdna_sys::DQ_NOT_ENOUGH_ROOM,
    DeviceBusy = powerdna_sys::DQ_DEVICE_BUSY,
    EventError = powerdna_sys::DQ_EVENT_ERROR,
    BadConfig = powerdna_sys::DQ_BAD_CONFIG,
    DataError = powerdna_sys::DQ_DATA_ERROR,
    DeviceNotReady = powerdna_sys::DQ_DEVICE_NOTREADY,
    CalibrationError = powerdna_sys::DQ_CALIBRATION_ERROR,
    WrongDmap = powerdna_sys::DQ_WRONG_DMAP,
    DataNotAvailable = powerdna_sys::DQ_DATA_NOT_AVAILABLE,
    FifoOverflow = powerdna_sys::DQ_FIFO_OVERFLOW,
    IllegalIndex = powerdna_sys::DQ_ILLEGAL_INDEX,
    DioLineNotExist = powerdna_sys::DQ_DIO_LINE_NOT_EXIST,
    WrongPacketCounter = powerdna_sys::DQ_WRONG_PKT_COUNTER,
    AsyncOutReRequested = powerdna_sys::DQ_ASYNC_OUT_REREQST,
    ProtocolMismatch = powerdna_sys::DQ_PROTOCOL_MISMATCH,
    CmdNotAllowed = powerdna_sys::DQ_CMD_NOTALLOWED,
    CmdAccessDenied = powerdna_sys::DQ_CMD_ACCESSDENIED,
    DeviceLocked = powerdna_sys::DQ_DEVLOCKED,
    CrcCheckFailed = powerdna_sys::DQ_CRC_CHECK_FAILED,
    Unknown = 0,
}

impl Display for PowerDnaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "{}", *self)
    }
}

impl Error for PowerDnaError {}

#[derive(Debug, FromPrimitive)]
#[repr(u32)]
pub enum PowerDnaSuccess {
    NoError = powerdna_sys::DQ_NOERROR,
    Success = powerdna_sys::DQ_SUCCESS,
    WaitEnded = powerdna_sys::DQ_WAIT_ENDED,
    DataNotReady = powerdna_sys::DQ_DATA_NOTREADY,
    DataNotExist = powerdna_sys::DQ_DATA_NOTEXIST,
    DeviceStarted = powerdna_sys::DQ_DEV_STARTED,
    DeviceStopped = powerdna_sys::DQ_DEV_STOPPED,
    Unknown = 1000,
}

macro_rules! parse_err {
    ($f:expr) => {{
        use num_traits::FromPrimitive;
        use $crate::results::PowerDnaError;
        use $crate::results::PowerDnaSuccess;
        let code: i32;
        unsafe {
            code = $f;
        }
        if code < 0 {
            Err(FromPrimitive::from_i32(code).unwrap_or(PowerDnaError::Unknown))
        } else {
            Ok(PowerDnaSuccess::from_u32(code as u32).unwrap_or(PowerDnaSuccess::Unknown))
        }
    }};
}
