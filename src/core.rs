use std::sync::mpsc;
use std::thread;
use std::time::{SystemTime};

use chrono::{DateTime, Utc};
use opencv::{
  prelude::*,
  Result,
  core::Size_,
  core::BORDER_DEFAULT,
  imgproc::gaussian_blur,
  imgproc::COLOR_BGR2GRAY,
  imgproc::cvt_color,
};
use ffmpeg_sys_next as ffs;
use ffs::{
  avpicture_fill,
  AVCodecID::AV_CODEC_ID_RAWVIDEO
};
use ffmpeg_next::{
  codec::encoder,
  format::Pixel,
  Picture
};


pub struct Frame {
  pub img: Mat,
  pub time: DateTime<Utc>,
  pub height: i32,
  pub width: i32

}


impl Frame {

    pub fn get_img(&self) -> &Mat {
      &self.img
    }

    pub fn blur(&self) -> Result<Frame> {
      let mut blurred = Mat::default();
      gaussian_blur(&self.img, &mut blurred, Size_::new(21, 21), 0.0, 0.0, BORDER_DEFAULT)?;
      Ok(Frame { img: blurred, ..*self })
    }

    pub fn grayscale(&self) -> Result<Frame> {
      let mut gray = Mat::default();
      cvt_color(&self.img, &mut gray, COLOR_BGR2GRAY, 0)?;
      Ok(Frame { img: gray, ..*self })
    }

    pub fn downsample(&self) -> Result<Frame> {
      self
        .grayscale()?
        .blur()
    }

}


impl Clone for Frame {

    fn clone(&self) -> Frame {
      Frame {
        img: self.img.clone(),
        time: self.time,
        height: self.height,
        width: self.width
      }
    }

    fn toAVFrame(&self) -> {


        let mut dst = Picture::new(Pixel::YUYV422, self.width, self.height).unwrap();

        codec::Codec encoder = codec::encoder::find(AV_CODEC_ID_RAWVIDEO);

        // AVFormatContext* outContainer = avformat_alloc_context();
        // how get output_file to be pipe instead:
        let mut octx = format::output(&output_file).unwrap();

        // AVStream *outStream = avformat_new_stream(outContainer, encoder);
        let mut outStream = octx.new_stream(encoder);

        // avcodec_get_context_defaults3(outStream->codec, encoder);
        // outStream->codec->pix_fmt = AV_PIX_FMT_BGR24;
        // outStream->codec->width = frame->cols;
        // outStream->codec->height = frame->rows;

        encoder.set_width(self.width);
        encoder.set_height(self.height);
        encoder.set_format(Pixel::YUYV422);

        // avpicture_fill((AVPicture*)&dst, frame->data, AV_PIX_FMT_BGR24, outStream->codec->width, outStream->codec->height);
        unsafe {
          avpicture_fill(&dst, &frame.img, Pixel::YUYV422, self.width as c_int, self.height as c_int);
        }

        return dst;
    }

}


