use std::sync::mpsc;
use std::thread;
use std::time::SystemTime;

use chrono::{DateTime, Utc};
use opencv::{
    core::Size_, core::BORDER_DEFAULT, imgproc::cvt_color, imgproc::gaussian_blur,
    imgproc::COLOR_BGR2GRAY, prelude::*, Result,
};

pub struct Frame {
    pub img: Mat,
    pub time: DateTime<Utc>,
    pub height: u32,
    pub width: u32,
}

pub struct VideoFrame {
    pub frame: Frame,
    pub is_start: bool,
    pub is_end: bool,
}

impl Frame {
    pub fn get_img(&self) -> &Mat {
        &self.img
    }

    pub fn blur(&self) -> Result<Frame> {
        let mut blurred = Mat::default();
        gaussian_blur(
            &self.img,
            &mut blurred,
            Size_::new(21, 21),
            0.0,
            0.0,
            BORDER_DEFAULT,
        )?;
        Ok(Frame {
            img: blurred,
            ..*self
        })
    }

    pub fn grayscale(&self) -> Result<Frame> {
        let mut gray = Mat::default();
        cvt_color(&self.img, &mut gray, COLOR_BGR2GRAY, 0)?;
        Ok(Frame { img: gray, ..*self })
    }

    pub fn downsample(&self) -> Result<Frame> {
        self.grayscale()?.blur()
    }
}

impl Clone for Frame {
    fn clone(&self) -> Frame {
        Frame {
            img: self.img.clone(),
            time: self.time,
            height: self.height,
            width: self.width,
        }
    }
}
