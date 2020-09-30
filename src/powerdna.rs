use libpowerdna_sys::DqAcbDestroy;
use libpowerdna_sys::pDQBCB;
use libpowerdna_sys::DQ_SS0IN;
use libpowerdna_sys::DqAcbCreate;
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
    dqe: pDQE,
    boards: Vec<Ai201>,
}

impl Daq {
    fn new(ip: &String, dqe: pDQE) -> Result<Self, i32> {
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
                boards: Vec::new(),
                dqe,
            })
        }
    }

    pub fn configure_inputs(&mut self, devices: Vec<u8>, freq: u32) -> Result<(), String> {
        self.boards.clear();
        match devices.into_iter().map(|dev_n| { Ai201::new(self.dqe, self.handle, dev_n, freq) }).collect() {
            Ok(boards) => {
                self.boards = boards;
                Ok(())
            },
            Err(s) => Err(s),
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

pub struct Ai201 {
    handle: i32,
    dev_n: u8,
    dqe: pDQE,
    bcb: pDQBCB,
}

impl Ai201 {
    fn new(dqe: pDQE, handle: i32, dev_n: u8, freq: u32) -> Result<Self, String> {
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
            return Err(format!("Failed to read device status. Handle: {} Device number: {} Code: {}", handle, dev_n, result_code));
        }

        if status_buffer[STS_FW as usize] & STS_FW_OPER_MODE != 0 {
            unsafe {
                result_code = DqCmdSetMode(handle, DQ_IOMODE_CFG, 1 << (device as u32 & DQ_MAXDEVN));
            }
        }

        if result_code < 0 {
            return Err(format!("Failed to read device status. Handle: {} Device number: {} Code: {}", handle, dev_n, result_code));
        }

        let mut bcb: pDQBCB = null_mut();

        unsafe {
            DqAcbCreate(dqe, handle, dev_n as u32, DQ_SS0IN, &mut bcb);
        }

        // DqAcbInitOps
        // DqeSetEvent
        // -- DqConvFillConvData
        // -- DqConvFillConvData
        // DqeEnable

        Ok(Ai201 {
            handle,
            dev_n,
            dqe,
            bcb,
        })
    }

    fn stream(&mut self, freq: u32) {
        
    }
}

impl Drop for Ai201 {
    fn drop(&mut self) {
        let result_code;
        // DqeEnable -> FALSE
        unsafe {
            result_code = DqAcbDestroy(self.bcb);
        }

        if result_code < 0 {
            eprintln!("DqAcbDestroy failed. Error code {}", result_code);
        }
    }
}

pub struct DqEngine {
    dqe: pDQE,
    daqs: HashMap<String, Daq>,
}

impl DqEngine {
    pub fn new(clock_period: u32) -> Result<Self, i32> {
        let mut dqe = null_mut();
        let code;
        
        unsafe {
            DqInitDAQLib();
            code = DqStartDQEngine(clock_period, &mut dqe, std::ptr::null_mut());
        }

        if code < 0 {
            Err(code)
        } else {
            Ok(
                DqEngine {
                    dqe,
                    daqs: HashMap::new(),
                }
            )
        }
    }

    pub fn open_daq(&mut self, ip: String) -> Result<&mut Daq, String> {
        if self.daqs.contains_key(&ip) {
            Err(format!("IP already in use. IP: {}", ip))
        } else {
            match Daq::new(&ip, self.dqe) {
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
            code = DqStopDQEngine(self.dqe);
        }

        if code < 0 {
            eprintln!("DqStopDQEngine failed. Error code {}", code);
        }

        unsafe {
            DqCleanUpDAQLib();
        }
    }
}
