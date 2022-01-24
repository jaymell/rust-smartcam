use chrono::{DateTime, Utc};
use log::debug;
use opencv::core::Vec3b;
use opencv::{
    core::Size_, core::BORDER_DEFAULT, imgproc::cvt_color, imgproc::gaussian_blur,
    imgproc::COLOR_BGR2GRAY, prelude::*, Result,
};
use std::convert::TryInto;
use std::error::Error;
use std::fmt;
use std::iter::Flatten;
use std::ops::Drop;
use std::str::FromStr;
use std::sync::Arc;
use std::time::SystemTime;
use strum::ParseError;
use strum_macros::{Display, EnumString};

#[derive(Display, EnumString, Copy, Clone, Debug)]
pub enum Colorspace {
    RGB,
    BGR,
    YUYV,
}

impl Colorspace {
    pub fn str(input: &str) -> Result<Colorspace, ParseError> {
        Colorspace::from_str(input)
    }

    pub fn convert_buf(&self, buf: Vec<u8>, target: Colorspace) -> Vec<u8> {
        match self {
            Self::RGB => match target {
                Colorspace::RGB => buf,
                Colorspace::BGR => {
                    let mut c = buf.clone();
                    rgb_bgr_swap(&mut c);
                    c
                }
                Colorspace::YUYV => panic!("{} to {} conversion not supported", self, target),
            },
            Self::BGR => match target {
                Colorspace::BGR => buf,
                Colorspace::RGB => {
                    let mut c = buf.clone();
                    rgb_bgr_swap(&mut c);
                    c
                }
                Colorspace::YUYV => panic!("{} to {} conversion not supported", self, target),
            },
            Self::YUYV => match target {
                Colorspace::YUYV => buf,
                Colorspace::BGR => {
                    debug!("buf len is {}", buf.len());
                    yuyv_to_bgr(&buf)
                }
                Colorspace::RGB => panic!("{} to {} conversion not supported", self, target),
            },
        }
    }
}

/// swap red and blue
pub fn rgb_bgr_swap(buf: &mut [u8]) {
    let mut i = 0;

    while i < buf.len() {
        let temp = buf[i];
        buf[i] = buf[i + 2];
        buf[i + 2] = temp;

        i = i + 3;
    }
}

pub fn yuyv_to_bgr(buf: &[u8]) -> Vec<u8> {
    /*
    Cr aka V aka red
    Cb aka U aka blue
    R = Y + 1.402 (Cr-128.0)
    G = Y - 0.34414 (Cb-128.0) - 0.71414 (Cr-128.0)
    B = Y + 1.772 (Cb-128.0)
    */

    let mut mat_buf = Vec::new();
    let mut i = 0;
    while i < buf.len() {
        let y1 = buf[i] as f64;
        let u = buf[i + 1] as f64;
        let y2 = buf[i + 2] as f64;
        let v = buf[i + 3] as f64;

        let p1_b = (y1 + (1.772 * (u - 128.0))) as u8;
        let p1_g = (y1 - (0.34414 * (u - 128.0)) - (0.71414 * (v - 128.0))) as u8;
        let p1_r = (y1 + 1.402 * (v - 128.0)) as u8;

        let p2_b = (y2 + (1.772 * (u - 128.0))) as u8;
        let p2_g = (y2 - (0.34414 * (u - 128.0)) - (0.71414 * (v - 128.0))) as u8;
        let p2_r = (y2 + 1.402 * (v - 128.0)) as u8;

        mat_buf.push(p1_b);
        mat_buf.push(p1_g);
        mat_buf.push(p1_r);
        mat_buf.push(p2_b);
        mat_buf.push(p2_g);
        mat_buf.push(p2_r);

        i = i + 4;
    }
    mat_buf
}

unsafe impl Send for Frame {}
unsafe impl Sync for Frame {}

#[derive(Debug)]
pub struct Frame {
    img: Mat,
    time: DateTime<Utc>,
    height: u32,
    width: u32,
    colorspace: Colorspace,
}

pub struct VideoFrame {
    pub frame: Arc<Frame>,
    pub is_start: bool,
    pub is_end: bool,
}

impl Frame {
    pub fn new(img: Mat, colorspace: Colorspace, time: Option<DateTime<Utc>>) -> Self {
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
            colorspace: colorspace,
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

    pub fn colorspace(&self) -> Colorspace {
        self.colorspace
    }

    pub fn buf(&self) -> Result<Vec<u8>, Box<dyn Error>> {
        unsafe {
            /* I have no idea why this doesn't display properly:

            let s = self.img.data_typed_unchecked::<Vec3b>()?;
            let mut v = Vec::new();
            for px in s {
                v.push(px[0]);
                v.push(px[1]);
                v.push(px[2]);
            }
            let l = v.len();
            Ok(v)
            */

            let sl = std::slice::from_raw_parts(self.img.data()?, self.img.total()? * 3).to_vec();
            Ok(sl)
        }
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
            colorspace: self.colorspace,
        }
    }
}

// impl Drop for Frame {
//     fn drop(&mut self) {
//         debug!("Dropping frame with time {}", self.time);
//     }
// }
