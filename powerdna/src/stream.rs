use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use crate::boards::Ai201;
use powerdna_sys::{pDQBCB, DqeEnable};
use crate::daq::Daq;
use crate::DaqError;
use crate::config::BoardConfig;
use tokio::sync::mpsc::UnboundedSender;
use std::sync::mpsc::{Receiver, channel};


fn pass_through(topic: String, input: Receiver<Vec<f64>>, out: UnboundedSender<(String, Vec<f64>)>) {
    loop {
        let buffer = match input.recv() {
            Ok(buf) => buf,
            Err(_) => {
                // channel closed -- time to shut down
                break;
            },
        };
        match out.send((topic.clone(), buffer)) {
            Ok(_) => (),
            Err(err) => eprintln!("Failed to push buffer to channel. Error: {}", err),
        };
    }
}


fn merge(topic: String, num_channels: usize, frames: usize, inputs: Vec<Receiver<Vec<f64>>>, out: UnboundedSender<(String, Vec<f64>)>) {
    // danger here! memcpy-style pointer arithmetic!
    loop {
        let mut combined: Vec<f64> = vec![0.0; num_channels * frames];

        let buffers: Vec<Vec<f64>> = match inputs.iter().map(|input| input.recv()).collect() {
            Ok(bufs) => bufs,
            Err(_) => {
                // one of the channels has been closed -- time to shut down
                break;
            },
        };

        for i in 0..frames {
            let mut dst_start = i * num_channels;
            for buf in &buffers {
                let src_start = i * 12;  // TODO num channels
                let src_end = src_start + 12;  // TODO  num channels
                let dst_end = dst_start + 12;  // TODO num channels
                &mut combined[dst_start..dst_end].copy_from_slice(&buf[src_start..src_end]);
                dst_start += 12;  // TODO num channels
            }
        }
        match out.send((topic.clone(), combined)) {
            Ok(_) => (),
            Err(err) => eprintln!("Failed to push merged buffer to channel. Error: {}", err),
        };
    }
}


pub struct Sampler {
    stop: Arc<AtomicBool>,
    muxer_thread: Option<thread::JoinHandle<()>>,
    boards: Vec<Arc<Ai201>>,
    board_threads: Option<Vec<thread::JoinHandle<()>>>,
}

impl Sampler {
    pub fn new(daq: Arc<Daq>, freq: u32, frame_size: u32, board_configs: &Vec<BoardConfig>, out: UnboundedSender<(String, Vec<f64>)>, topic: String) -> Result<Sampler, DaqError> {
        let stop = Arc::new(AtomicBool::new(false));
        let mut boards = Vec::new();
        let mut board_threads = Vec::new();
        let mut receivers = Vec::new();
        let mut total_channels = 0;

        for config in board_configs {
            let (tx, rx) = channel();
            let board = Arc::new(Ai201::new(Arc::clone(&daq), freq, frame_size, config, tx)?);

            let cloned_stop = Arc::clone(&stop);
            let cloned_board = Arc::clone(&board);
            let thread = thread::spawn(move || cloned_board.sample(cloned_stop));

            boards.push(board);
            board_threads.push(thread);
            receivers.push(rx);
            total_channels += config.channels.len();
        }

        let muxer_thread = Some(if receivers.len() > 1 {
            thread::spawn(move || {
                merge(topic, total_channels, frame_size as usize, receivers, out)
            })
        } else {
            let receiver = match receivers.pop() {
                Some(rx) => rx,
                None => return Err(DaqError::ChannelConfigError),
            };
            thread::spawn(move || {
                pass_through(topic, receiver, out)
            })
        });

        let bcbs: Vec<pDQBCB> = boards.iter().map(|board| board.bcb()).collect();
        parse_err!(DqeEnable(1, bcbs.as_ptr(), bcbs.len() as i32, 1))?;

        Ok(Sampler {
            stop,
            muxer_thread,
            boards,
            board_threads: Some(board_threads),
        })
    }
}

impl Drop for Sampler {
    fn drop(&mut self) {
        let bcbs: Vec<pDQBCB> = self.boards.iter().map(|board| board.bcb()).collect();
        match parse_err!(DqeEnable(0, bcbs.as_ptr(), bcbs.len() as i32, 1)) {
            Ok(_) => (),
            Err(err) => eprintln!("DqeEnable -> false failed. Error: {:?}", err),
        };
        self.stop.store(true, Ordering::SeqCst);
        match self.board_threads.take() {
            Some(threads) => {
                for thread in threads {
                    match thread.join() {
                        Ok(_) => (),
                        Err(_) => eprintln!("Failed to join board thread."),
                    };
                }
            },
            None => eprintln!("No board threads to join!!"),
        }
        self.boards.clear();
        match self.muxer_thread.take() {
            Some(handle) => match handle.join() {
                Ok(_) => (),
                Err(_) => eprintln!("Failed to join sampling thread."),
            },
            None => eprintln!("No muxer thread to join!!"),
        }
    }
}
