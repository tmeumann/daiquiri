use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use crate::boards::Ai201;
use crate::results::PowerDnaError;
use powerdna_sys::{
    DQ_ePacketLost,
    DQ_eBufferError,
    DQ_ePacketOOB,
    DQ_eFrameDone,
};
use crate::daq::Daq;
use crate::DaqError;
use crate::config::BoardConfig;
use std::sync::mpsc::Sender;

fn sampler(board: Arc<Ai201>, stop: Arc<AtomicBool>, mut raw_buffer: Vec<u16>, buffer_size: usize, tx: Sender<(String, Vec<u8>)>, topic: String) {
    loop {
        let events = match board.wait_for_event() {
            Err(PowerDnaError::TimeoutError) => {
                match stop.load(Ordering::SeqCst) {
                    true => break,
                    false => continue,
                };
            },
            Err(err) => {
                eprintln!("DqeWaitForEvent failed. Error: {:?}", err);
                break;
            },
            Ok(val) => val,
        };

        if events & DQ_ePacketLost != 0 {
            eprintln!("AI:DQ_ePacketLost");
        }
        if events & DQ_eBufferError != 0 {
            eprintln!("AI:DQ_eBufferError");
        }
        if events & DQ_ePacketOOB != 0 {
            eprintln!("AI:DQ_ePacketOOB");
        }

        if events & DQ_eFrameDone == 0 {
            continue;
        }

        let scaled_data = match board.get_scaled_data(&mut raw_buffer, buffer_size) {
            Ok(val) => val,
            Err(_) => {
                eprintln!("Failed to get scaled data. Skipping frame!");
                continue;
            },
        };

        match tx.send((topic.clone(), scaled_data)) {
            Ok(_) => {},
            Err(_) => break,  // TODO log me
        };
    }
}


pub struct Sampler {
    board: Arc<Ai201>,
    stop: Arc<AtomicBool>,
    join: Option<thread::JoinHandle<()>>,
}

impl Sampler {
    pub fn new(daq: Arc<Daq>, freq: u32, board_config: &BoardConfig, tx: Sender<(String, Vec<u8>)>, topic: String) -> Result<Sampler, DaqError> {
        let board = Ai201::new(daq, freq, board_config)?;

        let buffer_size = board.buffer_size()?;
        let raw_buffer = vec![0; buffer_size];

        let board = Arc::new(board);
        let cloned_board = Arc::clone(&board);
        let stop = Arc::new(AtomicBool::new(false));
        let cloned_stop = stop.clone();

        let join = Some(thread::spawn(move || {
            sampler(cloned_board, cloned_stop, raw_buffer, buffer_size, tx, topic)
        }));

        board.enable()?;

        Ok(Sampler {
            board,
            stop,
            join,
        })
    }
}

impl Drop for Sampler {
    fn drop(&mut self) {
        match self.board.disable() {
            Err(err) => eprintln!("DqeEnable -> false failed. Error: {:?}", err),
            Ok(_) => (),
        };
        self.stop.store(true, Ordering::SeqCst);
        match self.join.take() {
            Some(handle) => match handle.join() {
                Ok(_) => (),
                Err(_) => eprintln!("Failed to join sampling thread."),
            },
            None => (),
        }
    }
}
