use powerdna_sys::{
    pDQE,
    DqInitDAQLib,
    DqStartDQEngine,
    DqStopDQEngine,
    DqCleanUpDAQLib,
    pDQBCB,
    DqAcbCreate,
    DQ_SS0IN,
};
use std::ptr;
use crate::DaqError;
use crate::results::PowerDnaError;

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

        Ok(
            DqEngine {
                dqe,
            }
        )
    }

    pub(crate) fn create_acb(&self, handle: i32, device: u8) -> Result<pDQBCB, PowerDnaError> {
        let mut bcb: pDQBCB = ptr::null_mut();
        // mutation
        parse_err!(DqAcbCreate(self.dqe, handle, device as u32, DQ_SS0IN, &mut bcb))?;
        Ok(bcb)
    }
}

impl Drop for DqEngine {
    fn drop(&mut self) {
        match parse_err!(DqStopDQEngine(self.dqe)) {
            Err(err) => {
                eprintln!("DqStopDQEngine failed. Error: {:?}", err);
            },
            Ok(_) => {},
        };
        unsafe {
            DqCleanUpDAQLib();
        }
    }
}

unsafe impl Send for DqEngine {}
unsafe impl Sync for DqEngine {}  // TODO validate this one's ok
