use crate::frame::Frame;
use std::sync::mpsc::{Receiver, Sender};

pub fn start(receiver: Receiver<Frame>, out1: Sender<Frame>, out2: Sender<Frame>) -> () {
    loop {
        let frame = receiver.recv().unwrap();
        out1.send(frame.clone()).unwrap();
        out2.send(frame).unwrap();
    }
}
