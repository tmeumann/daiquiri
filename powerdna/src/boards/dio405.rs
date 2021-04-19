use crate::config::OutputConfig;
use crate::daq::Daq;
use crate::results::PowerDnaError;
use core::marker::{Send, Sync};
use core::mem::size_of;
use core::ptr;
use core::result::Result;
use core::result::Result::{Err, Ok};
use core::sync::atomic::{AtomicBool, Ordering};
use core::time::Duration;
use powerdna_sys::{event401_t_EV401_DI_CHANGE, pDQEVENT, EV401_ID};
use std::sync::Arc;
use tokio::time::sleep;

pub struct Dio405 {
    device: u8,
    daq: Arc<Daq>,
}

impl Dio405 {
    pub fn new(daq: Arc<Daq>, board_config: &OutputConfig) -> Result<Self, PowerDnaError> {
        let OutputConfig { device } = board_config;

        daq.enter_config_mode(*device)?;
        daq.setup_edge_events(*device)?; // TODO rationalise with above

        Ok(Dio405 {
            device: *device,
            daq,
        })
    }

    pub async fn trigger(&self) -> Result<(), PowerDnaError> {
        self.daq.write(self.device, 0xffffffff)?;
        sleep(Duration::from_millis(100)).await;
        self.daq.write(self.device, 0x0)?;
        Ok(())
    }

    pub fn sample(&self, stop: Arc<AtomicBool>) {
        let mut p_event: pDQEVENT = ptr::null_mut();
        let mut event: u32;
        let mut timestamp: u64;
        let mut pos: u64;
        let mut neg: u64;
        let mut data_slice;
        loop {
            match self.daq.receive_event(&mut p_event) {
                Err(PowerDnaError::TimeoutError) => {
                    match stop.load(Ordering::SeqCst) {
                        true => break,
                        false => {
                            continue;
                        }
                    };
                }
                Err(err) => {
                    eprintln!("DqCmdReceiveEvent failed. Error: {:?}", err);
                    break;
                }
                Ok(size) => size,
            };
            event = unsafe { (*p_event).event };
            if event != event401_t_EV401_DI_CHANGE {
                eprintln!("Unexpected event: {}", event);
                break;
            }
            unsafe {
                let data_ptr: *const u8 = (*p_event).data.as_ptr();
                let header_ptr: *const EV401_ID = data_ptr as *const _;
                timestamp = self.daq.to_host_repr((*header_ptr).tstamp as u64);
                let data_size: usize = (*header_ptr).size as usize / size_of::<u32>();
                data_slice = (*header_ptr).data.as_slice(data_size);
            }
            pos = self.daq.to_host_repr(data_slice[0] as u64);
            neg = self.daq.to_host_repr(data_slice[1] as u64);
            println!("ts: {}, pos: {}, neg: {}", timestamp, pos, neg);
            // match self.out.send(timestamp) {
            //     Ok(_) => (),
            //     Err(err) => {
            //         eprintln!("Failed to send edge detection timestamp. Error: {}", err);
            //         break;
            //     }
            // };
        }
    }
}

impl Drop for Dio405 {
    fn drop(&mut self) {
        match self.daq.teardown_edge_events(self.device) {
            Err(err) => eprintln!("Failed to teardown edge events. Error: {}", err),
            Ok(_) => (),
        };
    }
}

unsafe impl Send for Dio405 {}

unsafe impl Sync for Dio405 {}
