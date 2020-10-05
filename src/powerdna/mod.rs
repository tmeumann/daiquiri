use std::sync::Arc;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::atomic::AtomicBool;
use libpowerdna_sys::DqConvRaw2ScalePdc;
use libpowerdna_sys::DqAcbGetScansCopy;
use libpowerdna_sys::DQ_TIMEOUT_ERROR;
use libpowerdna_sys::DQ_EVENT_ERROR;
use libpowerdna_sys::DqeWaitForEvent;
use libpowerdna_sys::DqeEnable;
use libpowerdna_sys::pDATACONV;
use libpowerdna_sys::DqConvGetDataConv;
use libpowerdna_sys::DqConvFillConvData;
use libpowerdna_sys::DQ_eBufferDone;
use libpowerdna_sys::DQ_ePacketOOB;
use libpowerdna_sys::DQ_eBufferError;
use libpowerdna_sys::DQ_ePacketLost;
use libpowerdna_sys::DQ_eFrameDone;
use libpowerdna_sys::DqeSetEvent;
use libpowerdna_sys::DQ_AI201_MODEFIFO;
use libpowerdna_sys::DQ_LN_STREAMING;
use libpowerdna_sys::DQ_LN_CLCKSRC0;
use libpowerdna_sys::DQ_LN_IRQEN;
use libpowerdna_sys::DQ_LN_GETRAW;
use libpowerdna_sys::DQ_LN_ACTIVE;
use libpowerdna_sys::DQ_LN_ENABLED;
use libpowerdna_sys::DqAcbInitOps;
use libpowerdna_sys::DQ_LNCL_TIMESTAMP;
use libpowerdna_sys::DQ_ACB_DATA_RAW;
use libpowerdna_sys::DQ_ACB_DIRECTION_INPUT;
use libpowerdna_sys::DQ_ACBMODE_CYCLE;
use libpowerdna_sys::DQACBCFG;
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
use std::convert::TryInto;

#[macro_use]
pub mod results;

const TIMEOUT: u32 = 200;
const EVENT_TIMEOUT: i32 = 1000;
const CFG201: u32 = DQ_LN_ENABLED | DQ_LN_ACTIVE | DQ_LN_GETRAW | DQ_LN_IRQEN | DQ_LN_CLCKSRC0 | DQ_LN_STREAMING | DQ_AI201_MODEFIFO;


pub struct Daq {
    pub handle: i32,
    dqe: pDQE,
    stream: Option<SignalStream>,
}

impl Daq {
    fn new(ip: &String, dqe: pDQE) -> Result<Self, i32> {
        let mut handle = 0;
        let config = null_mut();
        let open_code;
        
        let ip_ptr = CString::new(ip.as_str()).expect("Failed to allocate memory for IP address.").into_raw();
        
        // match parse_err!(DqOpenIOM(ip_ptr, DQ_UDP_DAQ_PORT as u16, TIMEOUT, &mut handle, config)) {
        //     Ok(_) => {},
        //     Err(_) => eprintln!("floop"),
        // };

        unsafe {
            open_code = DqOpenIOM(ip_ptr, DQ_UDP_DAQ_PORT as u16, TIMEOUT, &mut handle, config);
            let _ = CString::from_raw(ip_ptr);  // reclaims memory
        }
        
        if open_code < 0 {
            Err(open_code)
        } else {
            Ok(Daq{
                handle,
                dqe,
                stream: None,
            })
        }
    }

    pub fn stream(&mut self, devices: Vec<u8>, freq: u32, stop: Arc<AtomicBool>) -> Result<&mut Option<SignalStream>, String> {
        self.stream = None;
        match devices.into_iter().map(|dev_n| { Ai201::new(self.dqe, self.handle, dev_n, freq) }).collect() {
            Ok(boards) => {
                self.stream = Some(SignalStream::new(boards, stop));
                Ok(&mut self.stream)
            },
            Err(s) => Err(s),
        }
    }
}

impl Drop for Daq {
    fn drop(&mut self) {
        let code;
        
        self.stream = None;

        unsafe {
            code = DqCloseIOM(self.handle);
        }
        
        if code < 0 {
            eprintln!("DqCloseIOM failed. Error code {}", code);
        }
    }
}



// TODO keep reference to boards, not config values
pub struct SignalStream {
    boards: Vec<Ai201>,
    raw_buffer: Vec<u16>,
    stop: Arc<AtomicBool>,
}

impl SignalStream {
    fn new(boards: Vec<Ai201>, stop: Arc<AtomicBool>) -> SignalStream {
        let result_code;

        let Ai201 { mut bcb, acb_cfg, .. } = boards[0];

        unsafe {
            result_code = DqeEnable(1, &mut bcb, 1, 0);
        }

        if result_code < 0 {
            eprintln!("DqeEnable -> true failed.");
        }

        let buffer_size = (acb_cfg.framesize * acb_cfg.frames * acb_cfg.scansz).try_into().unwrap();
        let raw_buffer = vec![0; buffer_size];  // TODO replace 'as' type coercion with safer options

        return SignalStream {
            boards,
            raw_buffer,
            stop,
        }
    }
}

impl Iterator for SignalStream {
    type Item = Vec<f64>;

    fn next(&mut self) -> std::option::Option<<Self as std::iter::Iterator>::Item> {
        if self.stop.load(SeqCst) {
            return None;
        };

        let Ai201 { bcb, acb_cfg, channels, pdc, .. } = &mut self.boards[0];

        let mut result_code = DQ_TIMEOUT_ERROR;
        let mut events: u32 = 0;
        
        while result_code == DQ_TIMEOUT_ERROR || events & DQ_eFrameDone == 0 {
            unsafe {
                result_code = DqeWaitForEvent(bcb, 1, 0, EVENT_TIMEOUT, &mut events);
            }

            if result_code == DQ_EVENT_ERROR {
                eprintln!("Error in DqeWaitForEvent()");
                return None;
            }
    
            if events & (DQ_ePacketLost|DQ_eBufferError|DQ_ePacketOOB) != 0 {  // TODO recover from errors
                if events & DQ_ePacketLost != 0 {
                    eprintln!("AI:DQ_ePacketLost");
                }
                if events & DQ_eBufferError != 0 {
                    eprintln!("AI:DQ_eBufferError");
                }
                if events & DQ_ePacketOOB != 0 {
                    eprintln!("AI:DQ_ePacketOOB");
                }
                return None;
            }
        }

        let framesize: u32 = acb_cfg.framesize;
        let mut received_scans: u32 = 0;
        let mut remaining_scans: u32 = 0;

        let buffer_size = (acb_cfg.framesize * acb_cfg.frames * acb_cfg.scansz).try_into().unwrap();
        let buffer_ptr = self.raw_buffer.as_mut_ptr() as *mut i8;
        
        unsafe {
            // TODO experiment with DqAcbGetScans to avoid copy
           result_code = DqAcbGetScansCopy(*bcb, buffer_ptr, framesize, framesize, &mut received_scans, &mut remaining_scans);
        }

        if result_code < 0 {
            eprintln!("DqAcbGetScansCopy failed.");
            return None;
        }

        let chans = channels.len() as u32;
        let mut scaled_buffer = vec![0.0; buffer_size];

        unsafe {
            result_code = DqConvRaw2ScalePdc(*pdc, channels.as_mut_ptr(), chans, received_scans * chans, buffer_ptr, scaled_buffer.as_mut_ptr());
        }

        if result_code < 0 {
            eprintln!("DqConvRaw2ScalePdc failed.");
            return None;
        }

        Some(scaled_buffer)
    }
}

impl Drop for SignalStream {
    fn drop(&mut self) {
        let result_code;

        unsafe {
            result_code = DqeEnable(0, &mut self.boards[0].bcb, 1, 0);
        }

        self.boards.clear();

        if result_code < 0 {
            eprintln!("DqeEnable -> false failed.");
        }
    }
}



pub struct Ai201 {
    bcb: pDQBCB,
    channels: Vec<u32>,
    pdc: pDATACONV,
    acb_cfg: DQACBCFG,
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
            return Err(format!("DqCmdReadStatus failed. Handle: {} Device number: {} Code: {}", handle, dev_n, result_code));
        }

        if status_buffer[STS_FW as usize] & STS_FW_OPER_MODE != 0 {
            unsafe {
                result_code = DqCmdSetMode(handle, DQ_IOMODE_CFG, 1 << (device as u32 & DQ_MAXDEVN));
            }
        }

        if result_code < 0 {
            return Err(format!("DqCmdSetMode failed. Handle: {} Device number: {} Code: {}", handle, dev_n, result_code));
        }

        let mut bcb: pDQBCB = null_mut();

        unsafe {
            result_code = DqAcbCreate(dqe, handle, dev_n as u32, DQ_SS0IN, &mut bcb);
        }

        if result_code < 0 {
            return Err(format!("DqAcbCreate failed. Handle: {} Device number: {} Code: {}", handle, dev_n, result_code));
        }

        let mut channels: Vec<u32> = (0..26).collect();
        channels[24] = 24;
        channels[25] = DQ_LNCL_TIMESTAMP;

        let mut acb_cfg = DQACBCFG::empty();

        acb_cfg.samplesz = 16;  // size of single reading
        acb_cfg.scansz = 26;  // size of 
        acb_cfg.framesize = 1000;  // frame size TODO
        acb_cfg.frames = 4;  // # of frames TODO
        acb_cfg.mode = DQ_ACBMODE_CYCLE;
        acb_cfg.samplesz = 16;
        acb_cfg.dirflags = DQ_ACB_DIRECTION_INPUT | DQ_ACB_DATA_RAW | DQ_ACB_DATA_RAW;

        let mut card_cfg = CFG201;
        let mut actual_freq = freq as f32;
        let mut num_channels = 26;

        unsafe {
            result_code = DqAcbInitOps(bcb, &mut card_cfg, null_mut(), null_mut(), &mut actual_freq, null_mut(), &mut num_channels, channels.as_mut_ptr(), null_mut(), &mut acb_cfg);
        }

        if result_code < 0 {
            return Err(format!("DqAcbInitOps failed. Handle: {} Device number: {} Code: {}", handle, dev_n, result_code));
        }

        unsafe {
            result_code = DqeSetEvent(bcb, DQ_eFrameDone | DQ_ePacketLost | DQ_eBufferError | DQ_ePacketOOB | DQ_eBufferDone)
        }

        if result_code < 0 {
            return Err(format!("DqeSetEvent failed. Handle: {} Device number: {} Code: {}", handle, dev_n, result_code));
        }

        unsafe {
            result_code = DqConvFillConvData(handle, dev_n as i32, DQ_SS0IN as i32, channels.as_mut_ptr(), num_channels);
        }

        if result_code < 0 {
            return Err(format!("DqConvFillConvData failed. Handle: {} Device number: {} Code: {}", handle, dev_n, result_code));
        }

        let mut pdc: pDATACONV = null_mut();

        unsafe {
            result_code = DqConvGetDataConv(handle, dev_n as i32, &mut pdc);
        }

        if result_code < 0 {
            return Err(format!("DqConvGetDataConv failed. Handle: {} Device number: {} Code: {}", handle, dev_n, result_code));
        }

        Ok(Ai201 {
            bcb,
            channels,
            pdc,
            acb_cfg,
        })
    }
}

impl Drop for Ai201 {
    fn drop(&mut self) {
        let result_code;
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



trait Empty {
    fn empty() -> Self;
}

impl Empty for DQACBCFG {
    fn empty() -> Self {
        Self {
            dirflags: 0,
            eucoeff: 0.0,
            euconvert: None,
            euoffset: 0.0,
            frames: 0,
            framesize: 0,
            hostringsz: 0,
            hwbufsize: 0,
            maxpktsize: 0,
            mode: 0,
            ppevent: 0,
            samplesz: 0,
            scansz: 0,
            valuesz: 0,
            wtrmark: 0,
        }
    }
}
