use libpowerdna_sys::STS_FW_OPER_MODE;
use libpowerdna_sys::STS_FW;
use libpowerdna_sys::DQ_MAXDEVN;
use libpowerdna_sys::DQ_LASTDEV;
use libpowerdna_sys::DqCmdReadStatus;
use libpowerdna_sys::DQ_IOMODE_CFG;
use libpowerdna_sys::DqCmdSetMode;
use std::collections::HashMap;
use libpowerdna_sys::DqCloseIOM;
use libpowerdna_sys::DQ_UDP_DAQ_PORT;
use libpowerdna_sys::DqOpenIOM;
use libpowerdna_sys::DqStopDQEngine;
use libpowerdna_sys::pDQE;
use std::ptr::null_mut;
use libpowerdna_sys::DqStartDQEngine;
use std::ffi::CString;
use libpowerdna_sys::DqCleanUpDAQLib;
use libpowerdna_sys::DqInitDAQLib;

const TIMEOUT: u32 = 200;

pub struct Daq {
    pub handle: i32,
    pub boards: HashMap<u8, IoBoard201>,
}

impl Daq {
    fn new(ip: &String) -> Result<Self, i32> {
        let mut handle = 0;
        let config = null_mut();
        let open_code;
        
        let ip_ptr = CString::new(ip.as_str()).expect("Failed to allocate memory for IP address.").into_raw();
        
        unsafe {
            open_code = DqOpenIOM(ip_ptr, DQ_UDP_DAQ_PORT as u16, TIMEOUT, &mut handle, config);
            let _ = CString::from_raw(ip_ptr);  // reclaims memory
        }
        
        if open_code < 0 {
            Err(open_code)
        } else {
            Ok(Daq{
                handle,
                boards: HashMap::new(),
            })
        }
    }

    pub fn add_201(&mut self, dev_n: u8, freq: u32) -> Result<&IoBoard201, String> {
        if self.boards.contains_key(&dev_n) {
            Err(format!("Board already present. Device number: {}", dev_n))
        } else {
            match IoBoard201::new(self.handle, dev_n, freq) {
                Ok(board) => {
                    self.boards.insert(dev_n, board);
                    match self.boards.get(&dev_n) {
                        Some(board) => Ok(board),
                        None => Err(format!("Something weird happened. Device number: {}", dev_n)),
                    }
                },
                Err(code) => Err(format!("Failed to initialise board {} with frequency {}. Code: {}", dev_n, freq, code))
            }
        }
    }
}

impl Drop for Daq {
    fn drop(&mut self) {
        let code;
        
        unsafe {
            code = DqCloseIOM(self.handle);
        }
        
        if code < 0 {
            eprintln!("DqCloseIOM failed. Error code {}", code);
        }
    }
}

pub struct IoBoard201 {
    
}

impl IoBoard201 {
    fn new(handle: i32, dev_n: u8, _freq: u32) -> Result<Self, i32> {
        // TODO consider powering down between sampling sessions?
        let mut result_code: i32;
        let mut device: u8 = dev_n | (DQ_LASTDEV as u8);
        let mut num_devices: u32 = 1;
        let mut status_buffer: [u32; DQ_MAXDEVN as usize + 1] = [0; DQ_MAXDEVN as usize + 1];
        let mut status_size: u32 = status_buffer.len() as u32;

        unsafe {
            result_code = DqCmdReadStatus(handle, &mut device, &mut num_devices, &mut status_buffer[0], &mut status_size);
        }
        
        if result_code < 0 {
            eprintln!("Failed to read device status. Handle: {} Device number: {}", handle, dev_n);
            return Err(result_code);
        }

        if status_buffer[STS_FW as usize] & STS_FW_OPER_MODE != 0 {
            unsafe {
                result_code = DqCmdSetMode(handle, DQ_IOMODE_CFG, 1 << (device as u32 & DQ_MAXDEVN));
            }
        }

        if result_code < 0 {
            Err(result_code)
        } else {
            Ok(IoBoard201 {})
        }
    }
}

pub struct DqEngine {
    reference: pDQE,
    daqs: HashMap<String, Daq>,
}

impl DqEngine {
    pub fn new(clock_period: u32) -> Result<Self, i32> {
        let mut reference = null_mut();
        let code;
        
        unsafe {
            DqInitDAQLib();
            code = DqStartDQEngine(clock_period, &mut reference, std::ptr::null_mut());
        }

        if code < 0 {
            Err(code)
        } else {
            Ok(
                DqEngine {
                    reference,
                    daqs: HashMap::new(),
                }
            )
        }
    }

    pub fn open_daq(&mut self, ip: String) -> Result<&mut Daq, String> {
        if self.daqs.contains_key(&ip) {
            Err(format!("IP already in use. IP: {}", ip))
        } else {
            match Daq::new(&ip) {
                Ok(daq) => {
                    self.daqs.insert(ip.clone(), daq);
                    match self.daqs.get_mut(&ip) {
                        Some(daq) => Ok(daq),
                        None => Err(format!("Something weird happened. IP: {}", ip)),
                    }
                },
                Err(code) => Err(format!("Failed to connect to {}. Code: {}", ip, code))
            }
        }
    }
}

impl Drop for DqEngine {
    fn drop(&mut self) {
        self.daqs.clear();
        let code;
        unsafe {
            code = DqStopDQEngine(self.reference);
        }

        if code < 0 {
            eprintln!("DqStopDQEngine failed. Error code {}", code);
        }

        unsafe {
            DqCleanUpDAQLib();
        }
    }
}
