use std::time::SystemTime;

use chrono::{DateTime, Utc};
use opencv::{
    core::Size_, core::BORDER_DEFAULT, imgproc::cvt_color, imgproc::gaussian_blur,
    imgproc::COLOR_BGR2GRAY, prelude::*, Result,
};

pub struct Frame {
    img: Mat,
    time: DateTime<Utc>,
    height: u32,
    width: u32,
}

pub struct VideoFrame {
    pub frame: Frame,
    pub is_start: bool,
    pub is_end: bool,
}

impl Frame {
    pub fn new(img: Mat, time: Option<DateTime<Utc>>) -> Self {
        let _time = if let Some(t) = time {
            t
        } else {
            let now: DateTime<Utc> = SystemTime::now().into();
            now
        };

        Self {
            time: _time,
            width: img.size().unwrap().width as u32,
            height: img.size().unwrap().height as u32,
            img: img,
        }
    }

    pub fn img(&self) -> &Mat {
        &self.img
    }

    pub fn img_mut(&mut self) -> &mut Mat {
        &mut self.img
    }

    pub fn time(&self) -> DateTime<Utc> {
        self.time
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn width(&self) -> u32 {
        self.width
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
