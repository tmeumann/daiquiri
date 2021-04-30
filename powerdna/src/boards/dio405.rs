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
use std::convert::TryFrom;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;
use tokio::time::sleep;

pub struct Dio405 {
    device: u8,
    daq: Arc<Daq>,
    topic: String,
    out: UnboundedSender<(String, u32)>,
}

impl Dio405 {
    pub fn new(
        daq: Arc<Daq>,
        topic: String,
        board_config: &OutputConfig,
        out: UnboundedSender<(String, u32)>,
    ) -> Result<Self, PowerDnaError> {
        let OutputConfig { device } = board_config;

        daq.setup_edge_events(*device)?;

        Ok(Dio405 {
            device: *device,
            topic,
            daq,
            out,
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
        let mut timestamp: u32;
        let mut pos: u64;
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
                timestamp = match u32::try_from(self.daq.to_host_repr((*header_ptr).tstamp as u64))
                {
                    Ok(val) => val,
                    Err(_) => {
                        eprintln!("Integer overflow when reading timestamp.");
                        break;
                    }
                };
                let data_size: usize = (*header_ptr).size as usize / size_of::<u32>();
                data_slice = (*header_ptr).data.as_slice(data_size);
            }
            pos = self.daq.to_host_repr(data_slice[0] as u64);

            // only push buzzer event if positive edge detected
            if pos != 0 {
                match self.out.send((self.topic.clone(), timestamp)) {
                    Ok(_) => (),
                    Err(err) => {
                        eprintln!("Failed to send edge detection timestamp. Error: {}", err);
                        break;
                    }
                };
            }
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
