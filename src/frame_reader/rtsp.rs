extern crate ffmpeg_next as ffmpeg;
use super::FrameReader;
use anyhow::Result;
use ffmpeg::format::{input, Pixel};
use ffmpeg::media::Type;
use ffmpeg::software::scaling::{context::Context, flag::Flags};
use ffmpeg::util::frame::video::Video;
use ffmpeg_next::codec::packet::packet::Packet;
use log::{debug, error, trace, warn};
use opencv::core::Mat_AUTO_STEP;
use opencv::core::CV_8UC3;
use opencv::prelude::*;
use std::error::Error;
use std::sync::{mpsc::channel, mpsc::Receiver, mpsc::Sender, Arc};
use std::time::SystemTime;
use tokio::sync::mpsc::Sender as AsyncSender;

use crate::frame::{Colorspace, Frame};

use opencv::core::Vector;
use opencv::imgcodecs::imwrite;
use std::fs::File;
use std::io::prelude::*;
use std::thread;
use std::thread::JoinHandle;

pub struct RTSPFrameReader {}

struct DecoderThread {
    packet_rx: Receiver<Packet>,
    decoder: ffmpeg::decoder::Video,
    senders: Vec<Sender<Arc<Frame>>>,
    web_tx: Option<AsyncSender<Arc<Frame>>>,
    scaler: Context,
}

impl DecoderThread {
    pub fn new(
        packet_rx: Receiver<Packet>,
        decoder: ffmpeg::decoder::Video,
        senders: Vec<Sender<Arc<Frame>>>,
        web_tx: Option<AsyncSender<Arc<Frame>>>,
    ) -> Self {
        let mut scaler = Context::get(
            decoder.format(),
            decoder.width(),
            decoder.height(),
            Pixel::BGR24,
            decoder.width(),
            decoder.height(),
            Flags::BILINEAR,
        )
        .unwrap();

        Self {
            packet_rx,
            decoder,
            senders,
            web_tx,
            scaler,
        }
    }

    pub fn start(&mut self) {
        debug!(
            "Original dimensions are {} x {}",
            self.decoder.width(),
            self.decoder.height()
        );

        loop {
            let packet = match self.packet_rx.recv() {
                Ok(packet) => packet,
                Err(error) => {
                    error!("Failed to receive packet: {:?}", error);
                    continue;
                }
            };
            match self.decoder.send_packet(&packet) {
                Ok(_) => (),
                Err(e) => {
                    warn!("Error decoding packet: {} -- dropping", e);
                    continue;
                }
            }

            self.receive_and_process_decoded_frames();
        }

        panic!("Decoder thread exited");
    }

    fn receive_and_process_decoded_frames(&mut self) -> Result<()> {
        let mut decoded = Video::empty();

        while self.decoder.receive_frame(&mut decoded).is_ok() {
            let mut rgb_frame = Video::empty();

            self.scaler.run(&decoded, &mut rgb_frame)?;

            let rgb_frame = rgb_frame.data(0);
            let img = unsafe {
                Mat::new_rows_cols_with_data(
                    self.decoder.height() as _,
                    self.decoder.width() as _,
                    CV_8UC3,
                    // Note: this data is not copied:
                    rgb_frame.as_ptr() as *mut std::os::raw::c_void,
                    Mat_AUTO_STEP,
                )?
            };
            let frame = Frame::new(img.clone(), Colorspace::BGR, Some(SystemTime::now().into()));
            let a = Arc::new(frame);
            for s in &self.senders {
                s.send(Arc::clone(&a))?;
            }
            if let Some(s) = &self.web_tx {
                s.blocking_send(Arc::clone(&a))?;
            }
        }
        Ok(())
    }
}

impl FrameReader for RTSPFrameReader {
    fn read_frames(
        &self,
        senders: Vec<Sender<Arc<Frame>>>,
        web_tx: Option<AsyncSender<Arc<Frame>>>,
        source: Option<&str>,
    ) {
        // AVFormatContext
        let mut ictx = input(&source.unwrap()).unwrap();
        // Stream (Context -> AVFormatContext)
        let input = ictx.streams().best(Type::Video).unwrap();
        let video_stream_index = input.index();
        let mut ff_decoder = input
            // AVCodecContext
            .codec()
            // Docoder(AVCodecContext)
            .decoder()
            .video()
            .unwrap();

        let (packet_tx, packet_rx) = channel();
        let decoder_thread = thread::spawn(move || -> () {
            let mut dec = DecoderThread::new(packet_rx, ff_decoder, senders, web_tx);
            dec.start();
        });

        loop {
            for (stream, packet) in ictx.packets() {
                if stream.index() == video_stream_index {
                    if let Err(e) = packet_tx.send(packet) {
                        error!("Packet send failed: {}", e);
                        continue;
                    }
                }
            }
            warn!("Input packet iterator returned None -- restarting");
        }

        panic!("RTSPFrameReader.read_frame exiting");
    }
}

fn save_file(frame: &Video, index: usize) -> std::result::Result<(), std::io::Error> {
    let mut file = File::create(format!("frame{}.ppm", index))?;
    file.write_all(format!("P6\n{} {}\n255\n", frame.width(), frame.height()).as_bytes())?;
    file.write_all(frame.data(0))?;
    Ok(())
}
