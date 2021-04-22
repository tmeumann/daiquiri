use crate::boards::ai201::Ai201;
use crate::boards::dio405::Dio405;
use crate::boards::Bcb;
use crate::config::{BoardConfig, OutputConfig};
use crate::daq::Daq;
use crate::DaqError;
use itertools::Itertools;
use powerdna_sys::{pDQBCB, DqeEnable};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, RecvError};
use std::sync::Arc;
use std::thread;
use tokio::sync::mpsc::UnboundedSender;

pub struct Sampler {
    stop: Arc<AtomicBool>,
    muxer_thread: Option<thread::JoinHandle<()>>,
    boards: Vec<Arc<Ai201>>,
    board_threads: Option<Vec<thread::JoinHandle<()>>>,
    outputs: Vec<Arc<Dio405>>,
}

impl Sampler {
    pub fn new(
        daq: Arc<Daq>,
        freq: u32,
        frame_size: u32,
        board_configs: &Vec<BoardConfig>,
        output_configs: &Vec<OutputConfig>,
        out: UnboundedSender<(String, Vec<f64>, Vec<u32>)>,
        topic: String,
    ) -> Result<Sampler, DaqError> {
        let stop = Arc::new(AtomicBool::new(false));
        let mut boards = Vec::new();
        let mut board_threads = Vec::new();
        let mut receivers = Vec::new();

        for config in board_configs {
            let (tx, rx) = channel();
            let board = Arc::new(Ai201::new(Arc::clone(&daq), freq, frame_size, config, tx)?);

            let cloned_stop = Arc::clone(&stop);
            let cloned_board = Arc::clone(&board);
            let thread = thread::spawn(move || cloned_board.sample(cloned_stop));

            boards.push(board);
            board_threads.push(thread);
            receivers.push((rx, config.channels.len()));
        }

        let muxer_thread = Some(if receivers.len() > 1 {
            thread::spawn(move || merge(topic, frame_size as usize, receivers, out))
        } else {
            let (rx, _) = match receivers.pop() {
                Some(item) => item,
                None => return Err(DaqError::ChannelConfigError),
            };
            thread::spawn(move || pass_through(topic, rx, out))
        });

        let mut outputs: Vec<Arc<Dio405>> = Vec::new();

        for config in output_configs {
            let output_board = Arc::new(Dio405::new(Arc::clone(&daq), config)?);
            let cloned_stop = Arc::clone(&stop);
            let cloned_board = Arc::clone(&output_board);
            outputs.push(output_board);
            board_threads.push(thread::spawn(move || cloned_board.sample(cloned_stop)));
        }

        let bcbs: Vec<pDQBCB> = boards.iter().map(|board| board.bcb()).collect();
        parse_err!(DqeEnable(1, bcbs.as_ptr(), bcbs.len() as i32, 1))?;

        Ok(Sampler {
            stop,
            muxer_thread,
            boards,
            board_threads: Some(board_threads),
            outputs,
        })
    }

    pub async fn trigger(&mut self) -> Result<(), DaqError> {
        for output in self.outputs.as_slice() {
            output.trigger().await?;
        }
        Ok(())
    }
}

fn pass_through(
    topic: String,
    input: Receiver<(Vec<f64>, Vec<u32>)>,
    out: UnboundedSender<(String, Vec<f64>, Vec<u32>)>,
) {
    loop {
        let (data, timestamps) = match input.recv() {
            Ok(buf) => buf,
            Err(_) => {
                // channel closed -- time to shut down
                break;
            }
        };
        if data.len() != timestamps.len() {
            eprintln!("Buffers differ in length.");
            break;
        }
        match out.send((topic.clone(), data, timestamps)) {
            Ok(_) => (),
            Err(err) => eprintln!("Failed to push buffer to channel. Error: {}", err),
        };
    }
}

fn merge(
    topic: String,
    frames: usize,
    inputs: Vec<(Receiver<(Vec<f64>, Vec<u32>)>, usize)>,
    out: UnboundedSender<(String, Vec<f64>, Vec<u32>)>,
) {
    let total_channels = inputs.iter().fold(0, |total, (_, chans)| total + chans);
    loop {
        let mut combined: Vec<f64> = vec![0.0; total_channels * frames];

        let messages: Vec<((Vec<f64>, Vec<u32>), &usize)> = match inputs
            .iter()
            .map(|(input, chans)| {
                Ok::<((Vec<f64>, Vec<u32>), &usize), RecvError>((input.recv()?, chans))
            })
            .collect()
        {
            Ok(bufs) => bufs,
            Err(_) => {
                // one of the channels has been closed -- time to shut down
                break;
            }
        };

        let (data_buffers, timestamp_buffers): (Vec<(Vec<f64>, &usize)>, Vec<Vec<u32>>) = messages
            .into_iter()
            .map(
                |((data, timestamps), chans)| -> ((Vec<f64>, &usize), Vec<u32>) {
                    ((data, chans), timestamps)
                },
            )
            .unzip();

        if !data_buffers
            .iter()
            .map(|(v, chans)| v.len() / (*chans + 2)) // 2 extra channels for timestamp
            .chain(timestamp_buffers.iter().map(|v| v.len()))
            .all_equal()
        {
            eprintln!("Buffers differ in length.");
            break;
        }

        // TODO reinstate this once we've got a synchronised clock or start trigger
        // if !(timestamp_buffers.iter().map(|v| v[0]).all_equal()
        //     && timestamp_buffers.iter().map(|v| v[v.len() - 1]).all_equal())
        // {
        //     eprintln!("Timestamp mismatch.");
        //     break;
        // }

        let timestamps = match timestamp_buffers.into_iter().nth(0) {
            Some(buf) => buf,
            None => {
                break;
            }
        };

        // danger here! memcpy-style pointer arithmetic!
        for i in 0..frames {
            let mut dst_start = i * total_channels;
            for (buf, &chans) in &data_buffers {
                let src_start = i * (chans + 2); // 2 extra values (timestamps)
                let src_end = src_start + chans;
                let dst_end = dst_start + chans;
                &mut combined[dst_start..dst_end].copy_from_slice(&buf[src_start..src_end]);
                dst_start += chans;
            }
        }
        match out.send((topic.clone(), combined, timestamps)) {
            Ok(_) => (),
            Err(err) => eprintln!("Failed to push merged buffer to channel. Error: {}", err),
        };
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
            }
            None => eprintln!("No board threads to join."),
        }
        self.boards.clear();
        self.outputs.clear();
        match self.muxer_thread.take() {
            Some(handle) => match handle.join() {
                Ok(_) => (),
                Err(_) => eprintln!("Failed to join sampling thread."),
            },
            None => eprintln!("No muxer thread to join!!"),
        }
    }
}
