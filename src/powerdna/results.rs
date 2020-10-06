use std::error::Error;
use std::fmt::Display;
use libpowerdna_sys::DQ_EVENT_ERROR;
use libpowerdna_sys::DQ_TIMEOUT_ERROR;
use libpowerdna_sys::DQ_CRC_CHECK_FAILED;
use libpowerdna_sys::DQ_DEVLOCKED;
use libpowerdna_sys::DQ_CMD_ACCESSDENIED;
use libpowerdna_sys::DQ_CMD_NOTALLOWED;
use libpowerdna_sys::DQ_PROTOCOL_MISMATCH;
use libpowerdna_sys::DQ_ASYNC_OUT_REREQST;
use libpowerdna_sys::DQ_WRONG_PKT_COUNTER;
use libpowerdna_sys::DQ_DIO_LINE_NOT_EXIST;
use libpowerdna_sys::DQ_ILLEGAL_INDEX;
use libpowerdna_sys::DQ_FIFO_OVERFLOW;
use libpowerdna_sys::DQ_DATA_NOT_AVAILABLE;
use libpowerdna_sys::DQ_WRONG_DMAP;
use libpowerdna_sys::DQ_CALIBRATION_ERROR;
use libpowerdna_sys::DQ_DEVICE_NOTREADY;
use libpowerdna_sys::DQ_DATA_ERROR;
use libpowerdna_sys::DQ_BAD_CONFIG;
use libpowerdna_sys::DQ_DEVICE_BUSY;
use libpowerdna_sys::DQ_NOT_ENOUGH_ROOM;
use libpowerdna_sys::DQ_NO_MEMORY;
use libpowerdna_sys::DQ_NOT_IMPLEMENTED;
use libpowerdna_sys::DQ_BAD_DEVN;
use libpowerdna_sys::DQ_BAD_PARAMETER;
use libpowerdna_sys::DQ_INIT_ERROR;
use libpowerdna_sys::DQ_ILLEGAL_PKTSIZE;
use libpowerdna_sys::DQ_PKT_TOOLONG;
use libpowerdna_sys::DQ_IOM_ERROR;
use libpowerdna_sys::DQ_RECV_ERROR;
use libpowerdna_sys::DQ_SEND_ERROR;
use libpowerdna_sys::DQ_SOCK_LIB_ERROR;
use libpowerdna_sys::DQ_ILLEGAL_HANDLE;
use libpowerdna_sys::DQ_ILLEGAL_ENTRY;
use libpowerdna_sys::DQ_DEV_STOPPED;
use libpowerdna_sys::DQ_DEV_STARTED;
use libpowerdna_sys::DQ_DATA_NOTEXIST;
use libpowerdna_sys::DQ_DATA_NOTREADY;
use libpowerdna_sys::DQ_WAIT_ENDED;
use libpowerdna_sys::DQ_SUCCESS;
use libpowerdna_sys::DQ_NOERROR;

use enum_primitive::*;

enum_from_primitive! {
    #[derive(Debug)]
    #[repr(i32)]
    pub enum PowerDnaError {
        IllegalEntry = DQ_ILLEGAL_ENTRY,
        IllegalHandle = DQ_ILLEGAL_HANDLE,
        SocketError = DQ_SOCK_LIB_ERROR,
        TimeoutError = DQ_TIMEOUT_ERROR,
        SendingError = DQ_SEND_ERROR,
        ReceivingError = DQ_RECV_ERROR,
        IomError = DQ_IOM_ERROR,
        PacketTooLong = DQ_PKT_TOOLONG,
        IllegalPacketSize = DQ_ILLEGAL_PKTSIZE,
        InitError = DQ_INIT_ERROR,
        BadParameter = DQ_BAD_PARAMETER,
        BadDevNumber = DQ_BAD_DEVN,
        NotImplemented = DQ_NOT_IMPLEMENTED,
        NoMemory = DQ_NO_MEMORY,
        PacketFull = DQ_NOT_ENOUGH_ROOM,
        DeviceBusy = DQ_DEVICE_BUSY,
        EventError = DQ_EVENT_ERROR,
        BadConfig = DQ_BAD_CONFIG,
        DataError = DQ_DATA_ERROR,
        DeviceNotReady = DQ_DEVICE_NOTREADY,
        CalibrationError = DQ_CALIBRATION_ERROR,
        WrongDmap = DQ_WRONG_DMAP,
        DataNotAvailable = DQ_DATA_NOT_AVAILABLE,
        FifoOverflow = DQ_FIFO_OVERFLOW,
        IllegalIndex = DQ_ILLEGAL_INDEX,
        DioLineNotExist = DQ_DIO_LINE_NOT_EXIST,
        WrongPacketCounter = DQ_WRONG_PKT_COUNTER,
        AsyncOutReRequested = DQ_ASYNC_OUT_REREQST,
        ProtocolMismatch = DQ_PROTOCOL_MISMATCH,
        CmdNotAllowed = DQ_CMD_NOTALLOWED,
        CmdAccessDenied = DQ_CMD_ACCESSDENIED,
        DeviceLocked = DQ_DEVLOCKED,
        CrcCheckFailed = DQ_CRC_CHECK_FAILED,
        Unknown = 0,
    }
}

impl Display for PowerDnaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "{}", *self)
    }
}

impl Error for PowerDnaError {}


enum_from_primitive! {
    #[derive(Debug)]
    #[repr(u32)]
    pub enum PowerDnaSuccess {
        NoError = DQ_NOERROR,
        Success = DQ_SUCCESS,
        WaitEnded = DQ_WAIT_ENDED,
        DataNotReady = DQ_DATA_NOTREADY,
        DataNotExist = DQ_DATA_NOTEXIST,
        DeviceStarted = DQ_DEV_STARTED,
        DeviceStopped = DQ_DEV_STOPPED,
        Unknown = 1000,
    }
}


macro_rules! parse_err {
    ($f:expr) => {
        {
            use enum_primitive::*;
            use crate::powerdna::results::PowerDnaError;
            use crate::powerdna::results::PowerDnaSuccess;
            let code: i32;
            unsafe {
                code = $f;
            }
            if code < 0 {
                Err(PowerDnaError::from_i32(code).unwrap_or(PowerDnaError::Unknown))
            } else {
                Ok(PowerDnaSuccess::from_u32(code as u32).unwrap_or(PowerDnaSuccess::Unknown))
            }
        }
    }
}
