use crate::engine::{DqEngine, InterfaceType};
use crate::results::PowerDnaError;
use powerdna_sys::{
    pDATACONV, pDQBCB, DqCloseIOM, DqCmdReadStatus, DqCmdSetMode, DqConvFillConvData,
    DqConvGetDataConv, DqOpenIOM, DQ_IOMODE_CFG, DQ_LASTDEV, DQ_MAXDEVN, DQ_SS0IN, DQ_UDP_DAQ_PORT,
    STS_FW, STS_FW_OPER_MODE,
};
use std::ffi::CString;
use std::ptr;
use std::sync::Arc;

const TIMEOUT: u32 = 200;

pub struct Daq {
    handle: i32,
    dqe: Arc<DqEngine>,
}

unsafe impl Send for Daq {}

impl Daq {
    pub fn new(dqe: Arc<DqEngine>, ip: String) -> Result<Self, PowerDnaError> {
        // TODO introduce phantom data to track dqe lifetime?
        let mut handle = 0;
        let config = ptr::null_mut();

        let ip_ptr = CString::new(ip.as_str())
            .expect("Failed to allocate memory for IP address.")
            .into_raw();
        let result = parse_err!(DqOpenIOM(
            ip_ptr,
            DQ_UDP_DAQ_PORT as u16,
            TIMEOUT,
            &mut handle,
            config
        ));
        unsafe {
            let _ = CString::from_raw(ip_ptr); // reclaims memory
        }

        match result {
            Err(err) => Err(err),
            Ok(_) => Ok(Daq { handle, dqe }),
        }
    }

    pub(crate) fn create_acb(
        &self,
        device: u8,
        interface_type: InterfaceType,
    ) -> Result<pDQBCB, PowerDnaError> {
        self.dqe.create_acb(self.handle, device, interface_type)
    }

    pub(crate) fn enter_config_mode(&self, device: u8) -> Result<(), PowerDnaError> {
        let devices: u8 = device | (DQ_LASTDEV as u8);
        let mut num_devices: u32 = 1;
        let mut status_buffer: [u32; DQ_MAXDEVN as usize + 1] = [0; DQ_MAXDEVN as usize + 1];
        let mut status_size: u32 = status_buffer.len() as u32;

        // mutation
        parse_err!(DqCmdReadStatus(
            self.handle,
            &devices,
            &mut num_devices,
            &mut status_buffer[0],
            &mut status_size
        ))?;

        if status_buffer[STS_FW as usize] & STS_FW_OPER_MODE != 0 {
            parse_err!(DqCmdSetMode(
                self.handle,
                DQ_IOMODE_CFG,
                1 << (device as u32 & DQ_MAXDEVN)
            ))?;
        }

        Ok(())
    }

    pub(crate) fn get_data_converter(
        &self,
        device: u8,
        channels: &Vec<u32>,
    ) -> Result<pDATACONV, PowerDnaError> {
        parse_err!(DqConvFillConvData(
            self.handle,
            device as i32,
            DQ_SS0IN as i32,
            channels.as_ptr(),
            channels.len() as u32
        ))?;
        let mut pdc: pDATACONV = ptr::null_mut();
        // mutation
        parse_err!(DqConvGetDataConv(self.handle, device as i32, &mut pdc))?;
        Ok(pdc)
    }
}

impl Drop for Daq {
    fn drop(&mut self) {
        match parse_err!(DqCloseIOM(self.handle)) {
            Err(err) => {
                eprintln!("DqCloseIOM failed. Error: {:?}", err);
            }
            Ok(_) => {}
        };
    }
}
