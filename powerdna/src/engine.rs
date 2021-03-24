use crate::results::PowerDnaError;
use crate::DaqError;
use powerdna_sys::{
    pDQBCB, pDQE, DqAcbCreate, DqCleanUpDAQLib, DqInitDAQLib, DqStartDQEngine, DqStopDQEngine,
    DQ_SS0IN, DQ_SS0OUT,
};
use std::ptr;

pub enum InterfaceType {
    Input,
    Output,
}

pub struct DqEngine {
    dqe: pDQE,
}

impl DqEngine {
    pub fn new(clock_period: u32) -> Result<Self, DaqError> {
        let mut dqe = ptr::null_mut();

        unsafe {
            DqInitDAQLib();
        }
        parse_err!(DqStartDQEngine(clock_period, &mut dqe, ptr::null_mut()))?;

        Ok(DqEngine { dqe })
    }

    pub(crate) fn create_acb(
        &self,
        handle: i32,
        device: u8,
        interface_type: InterfaceType,
    ) -> Result<pDQBCB, PowerDnaError> {
        let mut bcb: pDQBCB = ptr::null_mut();

        let subsystem = match interface_type {
            InterfaceType::Input => DQ_SS0IN,
            InterfaceType::Output => DQ_SS0OUT,
        };

        // mutation
        parse_err!(DqAcbCreate(
            self.dqe,
            handle,
            device as u32,
            subsystem,
            &mut bcb
        ))?;
        Ok(bcb)
    }
}

impl Drop for DqEngine {
    fn drop(&mut self) {
        match parse_err!(DqStopDQEngine(self.dqe)) {
            Err(err) => {
                eprintln!("DqStopDQEngine failed. Error: {:?}", err);
            }
            Ok(_) => {}
        };
        unsafe {
            DqCleanUpDAQLib();
        }
    }
}

unsafe impl Send for DqEngine {}
unsafe impl Sync for DqEngine {} // TODO validate this one's ok
