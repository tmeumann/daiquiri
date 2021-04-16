use crate::engine::{DqEngine, InterfaceType};
use crate::results::{PowerDnaError, PowerDnaSuccess};
use powerdna_sys::{
    event401_t_EV401_CLEAR, event401_t_EV401_DI_CHANGE, pDATACONV, pDQBCB, pDQEVENT, DqAcbDestroy,
    DqAddIOMPort, DqAdv40xConfigEvents, DqAdv40xWrite, DqCloseIOM, DqCmdReadStatus,
    DqCmdReceiveEvent, DqCmdSetCfg, DqCmdSetMode, DqConvFillConvData, DqConvGetDataConv, DqNtohl,
    DqOpenIOM, DqRtAsyncEnableEvents, DQSETCFG, DQ_IOMODE_CFG, DQ_IOMODE_OPS, DQ_LASTDEV,
    DQ_LN_ACTIVE, DQ_LN_ENABLED, DQ_LN_MAPPED, DQ_MAXDEVN, DQ_SS0IN, DQ_UDP_DAQ_PORT,
    DQ_UDP_DAQ_PORT_ASYNC, STS_FW, STS_FW_OPER_MODE,
};
use std::ffi::CString;
use std::ptr;
use std::sync::Arc;

const TIMEOUT: u32 = 200;

pub struct Daq {
    handle: i32,
    async_handle: i32,
    dqe: Arc<DqEngine>,
}

unsafe impl Send for Daq {}

impl Daq {
    pub fn new(dqe: Arc<DqEngine>, ip: String) -> Result<Self, PowerDnaError> {
        let mut handle = 0;
        let config = ptr::null_mut();

        let ip_ptr = CString::new(ip.as_str())
            .expect("Failed to allocate memory for IP address.")
            .into_raw();
        parse_err!(DqOpenIOM(
            ip_ptr,
            DQ_UDP_DAQ_PORT as u16,
            TIMEOUT,
            &mut handle,
            config
        ))?;
        unsafe {
            let _ = CString::from_raw(ip_ptr); // reclaims memory
        }

        let mut async_handle = 0;
        parse_err!(DqAddIOMPort(
            handle,
            &mut async_handle,
            DQ_UDP_DAQ_PORT_ASYNC as u16,
            TIMEOUT
        ))?;

        Ok(Daq {
            handle,
            async_handle,
            dqe,
        })
    }

    pub(crate) fn create_acb(
        &self,
        device: u8,
        interface_type: InterfaceType,
    ) -> Result<pDQBCB, PowerDnaError> {
        self.dqe.create_acb(self.handle, device, interface_type)
    }

    pub(crate) fn destroy_acb(&self, bcb: pDQBCB) -> Result<PowerDnaSuccess, PowerDnaError> {
        parse_err!(DqAcbDestroy(bcb))
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
            parse_err!(DqCmdSetMode(self.handle, DQ_IOMODE_CFG, 1 << device))?;
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

    pub(crate) fn setup_edge_events(&self, device: u8) -> Result<(), PowerDnaError> {
        // clears any existing events
        parse_err!(DqAdv40xConfigEvents(
            self.async_handle,
            device as i32,
            event401_t_EV401_CLEAR,
            0,
            0
        ))?;
        // set edge detection events (bit 0 is line 0, bit 1 is line 1 etc.)
        parse_err!(DqAdv40xConfigEvents(
            self.async_handle,
            device as i32,
            event401_t_EV401_DI_CHANGE,
            0x1, // rising edge
            0x0, // falling edge
        ))?;
        // multiple devices might need to be coalesced into one call here
        parse_err!(DqRtAsyncEnableEvents(self.async_handle, 0, 1 << device))?;

        let mut cfg: DQSETCFG = DQSETCFG {
            dev: device | DQ_LASTDEV as u8,
            ss: DQ_SS0IN as u8,
            cfg: DQ_LN_ACTIVE | DQ_LN_ENABLED | DQ_LN_MAPPED,
        };

        let mut status = 0;
        let mut entries = 1;

        parse_err!(DqCmdSetCfg(
            self.handle,
            &mut cfg,
            &mut status,
            &mut entries
        ))?;
        // TODO check status and entries

        parse_err!(DqCmdSetMode(self.handle, DQ_IOMODE_OPS, 1 << device))?;

        Ok(())
    }

    pub(crate) fn receive_event(&self, event_ptr: &mut pDQEVENT) -> Result<i32, PowerDnaError> {
        let mut size: i32 = 0;
        parse_err!(DqCmdReceiveEvent(
            self.async_handle,
            0,         // reserved
            1000000,   // timeout, microseconds
            event_ptr, // allocated by powerdna lib
            &mut size,
        ))?;
        Ok(size)
    }

    pub(crate) fn to_host_repr(&self, value: u64) -> u64 {
        unsafe { DqNtohl(self.handle, value) }
    }

    pub(crate) fn teardown_edge_events(&self, device: u8) -> Result<(), PowerDnaError> {
        // TODO turn edge event handler into a struct?
        parse_err!(DqRtAsyncEnableEvents(self.handle, 0, 0))?;
        parse_err!(DqCmdSetMode(self.handle, DQ_IOMODE_CFG, 1 << device))?;
        Ok(())
    }

    pub(crate) fn write(&self, device: u8, value: u32) -> Result<(), PowerDnaError> {
        parse_err!(DqAdv40xWrite(self.handle, device as i32, value))?;
        Ok(())
    }
}

impl Drop for Daq {
    fn drop(&mut self) {
        match parse_err!(DqCloseIOM(self.async_handle)) {
            Err(err) => {
                eprintln!("Async DqCloseIOM failed. Error: {:?}", err);
            }
            Ok(_) => {}
        }
        match parse_err!(DqCloseIOM(self.handle)) {
            Err(err) => {
                eprintln!("DqCloseIOM failed. Error: {:?}", err);
            }
            Ok(_) => {}
        };
    }
}
