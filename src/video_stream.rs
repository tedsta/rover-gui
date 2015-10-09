use std::fs::File;
use std::io::{
    BufWriter,
    Write,
};
use std::mem;
use std::path::Path;
use std::ptr;
use std::slice;
use std::sync::{Arc, Mutex};
use std::thread;

use ffmpeg;
use ffmpeg::codec;
use ffmpeg::format;
use ffmpeg::media;
use ffmpeg::frame;
use ffmpeg::software::scaling;
use ffmpeg::util::format::pixel::Pixel;
use image::RgbaImage;

use opengl_graphics::Texture;

pub fn start_video_stream(path: &str, out_path: &str) -> (Texture, Arc<Mutex<RgbaImage>>) {
    let rgba_img = RgbaImage::new(512, 512);
    let video_texture = Texture::from_image(&rgba_img);
    let rgba_img = Arc::new(Mutex::new(rgba_img));

    let path = path.to_string();
    let out_path = out_path.to_string();
    
    let thread_rgba_img = rgba_img.clone();
    thread::Builder::new()
        .name("video_packet_in".to_string())
        .spawn(move || {
            /////////////////////////////////////////////////////
            // Open input stream
            
            let fps: u8 = 10;

            let mut format_context = format::input(&path).unwrap();
            //format::dump(&format_context, 0, Some(path.as_str()));

            let stream_codec =
                format_context.streams()
                              .filter(|stream| stream.codec().medium() == media::Type::Video)
                              .map(|stream| stream.codec())
                              .next().expect("No video streams in stream");
            let video_codec = codec::decoder::find(stream_codec.id()).unwrap();
            
            let codec_context = stream_codec.clone();

            let mut decoder = codec_context.decoder().video().unwrap();
            let mut sws_context = scaling::Context::get(decoder.format(), decoder.width(), decoder.height(),
                                                    Pixel::RGBA, 512, 512,
                                                    scaling::flag::BILINEAR).unwrap();
            
            let mut input_frame = frame::Video::new(decoder.format(), decoder.width(), decoder.height());
            let mut output_frame = frame::Video::new(Pixel::RGBA, 512, 512);

            /////////////////////////////////////////////////////
            // Open recording stream

            let mut rec_format = ffmpeg::format::output(&format!("{}.h264", out_path)).unwrap();

            let mut rec_video = {
                    let mut stream = rec_format.add_stream(stream_codec.id()).unwrap();
                    let mut codec  = stream.codec().encoder().video().unwrap();

                    codec.set_width(decoder.width());
                    codec.set_height(decoder.height());
                    //codec.set_format(ffmpeg::format::Pixel::YUV420P);
                    codec.set_format(decoder.format());
                    codec.set_time_base((1, fps as i32));
                    codec.set_flags(ffmpeg::codec::flag::GLOBAL_HEADER);

                    stream.set_time_base((1, 1_000));
                    stream.set_rate((fps as i32, 1));

                    //codec.open_as(stream_codec.id()).unwrap()
                    //codec.open().unwrap()
                    codec.encoder()
            };

            let mut rec_converter =
                ffmpeg::software::converter((decoder.width(), decoder.height()),
                                            decoder.format(),
                                            ffmpeg::format::Pixel::YUV420P).unwrap();

            rec_format.write_header().unwrap();

            let mut rec_packet = ffmpeg::Packet::empty();
            let mut rec_frame  = ffmpeg::frame::Video::empty();

            //let start = ffmpeg::time::relative() as i64;
            //let sleep = 1;

            /////////////////////////////////////////////////////
            // Process streams
            
            for (stream, packet) in format_context.packets() {
                decoder.decode(&packet, &mut input_frame).unwrap();
                
                if let Err(e) = sws_context.run(&input_frame, &mut output_frame) {
                    println!("WARNING: video software scaling error: {}", e);
                }
                
                //let mut buf: Vec<u8> = Vec::with_capacity(1048576);
                for line in output_frame.data().iter() {
                    let mut rgba_img = thread_rgba_img.lock().unwrap();
                
                    //buf.reserve(line.len());
                    unsafe {
                        //let buf_len = buf.len();
                        //buf.set_len(buf_len + line.len());
                        let src: *const u8 = mem::transmute(line.get(0));
                        //let dst: *mut u8 = std::mem::transmute(buf.get_mut(buf_len));
                        let dst = rgba_img.as_mut_ptr();
                        ptr::copy(src, dst, line.len());
                    }
                }

                println!("converting...");

                // Now encode the recording packets
                /*if let Err(e) = rec_converter.run(&input_frame, &mut rec_frame) {
                    println!("WARNING: video software converter error: {}", e);
                }*/
                //rec_frame.set_pts(Some((now - start) / sleep));

                println!("encoding...");
                if rec_video.encode(&input_frame, &mut rec_packet).unwrap() {
                    println!("Encoded!!!");
                    rec_packet.set_stream(0);
                    //rec_packet.rescale_ts((1, fps as i32), (1, 1_000))
                    rec_packet.write_interleaved(&mut rec_format).unwrap();
                }
            }

            // Write the recording packets
            /*while let Ok(true) = rec_video.flush(&mut rec_packet) {
                rec_packet.set_stream(0);
                //rec_packet.rescale_ts((1, fps as i32), (1, 1_000))
                rec_packet.write_interleaved(&mut rec_format).unwrap();
            }*/

            rec_format.write_trailer().unwrap();
        }).unwrap();
    
    (video_texture, rgba_img)
}

pub fn init_ffmpeg() {
    ffmpeg::init().unwrap();
    ffmpeg::format::network::init();
}
