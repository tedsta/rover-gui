use std::cell::RefCell;
use std::fs;
use std::io::{Read, Write};
use std::mem;
use std::net::UdpSocket;
use std::path::Path;
use std::rc::Rc;
use std::sync::mpsc::channel;
use std::thread;

extern crate time;
extern crate piston_window;
extern crate graphics;
extern crate image;
#[macro_use] extern crate conrod;
#[macro_use] extern crate ffmpeg;

use conrod::Theme;
use piston_window::{EventLoop, Glyphs, PistonWindow, WindowSettings};

use conrod_config::Ui;
use stereo_ui::StereoUi;
use video_stream::{init_ffmpeg, start_video_stream, VideoMsg};

mod conrod_config;
mod stereo_ui;
mod video_stream;
mod imu;

fn main() {
    init_ffmpeg();

    let ref mut window: PistonWindow = WindowSettings::new("PISCES Navigation".to_string(),
                                                           [1280, 700]).exit_on_esc(true)
                                                                       .build().unwrap();

    let mut ui = {
        let font_path = Path::new("./assets/fonts/NotoSans-Regular.ttf");
        let theme = Theme::default();
        let glyph_cache = Glyphs::new(&font_path, window.factory.clone());
        Ui::new(glyph_cache.unwrap(), theme)
    };
    
    // Create a UDP socket to talk to the rover
    let client = UdpSocket::bind("0.0.0.0:30002").unwrap();
    client.send_to(b"connect me plz", ("10.10.153.8", 30001));
    
    let client_in = client.try_clone().unwrap();
    let (packet_t, packet_r) = channel();

    /*let mut client = TcpStream::connect("10.10.153.8:30001").unwrap();
    client.write(b"connect me plz");
    
    let mut client_in = client.try_clone().unwrap();
    let (packet_t, packet_r) = channel();*/
    
    thread::Builder::new()
        .name("packet_in".to_string())
        .spawn(move || {
            let mut buf = [0u8; 512];
            loop {
                let (bytes_read, _) = client_in.recv_from(&mut buf).unwrap();
                //let bytes_read = client_in.read(&mut buf).unwrap();
                if let Ok(msg) = String::from_utf8(buf[0..bytes_read].iter().cloned().collect()) {
                    packet_t.send(msg).unwrap();
                }
            }
        }).unwrap();

    ////////////////////////////////////////////////////////////////////////////////////////
    
    let (video0_texture, video0_image) =
        //start_video_stream(window, None, "rtsp://10.10.153.9/axis-media/media.amp");
        //start_video_stream(window, None, "/dev/video1", 480);
        start_video_stream(window, None, "rtsp://10.10.153.8/stereo0", 1944);
    let (video1_texture, video1_image) =
        start_video_stream(window, None, "rtsp://10.10.153.8/stereo1", 1944);

    ///////////////////////////////////////////////////////////////////////////////////////
    
    let mut stereo_ui = StereoUi::new(client);
    stereo_ui.send_pan();
    stereo_ui.send_tilt();

    ////////////////////////////////////////////////////////////////////////////////////////

    let mut vid_textures = [video0_texture, video1_texture];

    let mut mouse_x = 0.0;
    let mut mouse_y = 0.0;
    
    ///////////////////////////////////////////////////////////////////////////////////////

    window.set_ups(10);
    window.set_max_fps(60);

    while let Some(e) = window.next() {
        use piston_window::{Button, PressEvent, ReleaseEvent, UpdateEvent, MouseCursorEvent};

        ui.handle_event(&e);

        e.mouse_cursor(|x, y| {
            mouse_x = x;
            mouse_y = y;
        });
        
        e.press(|button| {
            match button {
                Button::Keyboard(key) => stereo_ui.on_key_pressed(key), 
                _ => { },
            }
        });
        
        e.release(|button| {
            match button {
                Button::Keyboard(key) => stereo_ui.on_key_released(key), 
                _ => { },
            }
        });
        
        // Update
        e.update(|u_args| {
            stereo_ui.update(u_args.dt);

            while let Ok(packet) = packet_r.try_recv() {
                stereo_ui.handle_packet(packet);
            }
            
            let video0_image = video0_image.lock().unwrap();
            vid_textures[0].update(&mut window.encoder, &video0_image.as_rgba8().unwrap());
            
            let video1_image = video1_image.lock().unwrap();
            vid_textures[1].update(&mut window.encoder, &video1_image.as_rgba8().unwrap());
        });

        // Render GUI
        window.draw_2d(&e, |c, g| {
            use graphics::*;

            ui.set_widgets(|ref mut ui| {
                stereo_ui.set_widgets(ui);
            });

            stereo_ui.draw_ui(c, g, &mut ui);

            Rectangle::new([0.0, 0.0, 0.4, 1.0])
                .draw([5.0, 80.0, 630.0, 480.0],
                      &c.draw_state, c.transform,
                      g);
            image(&vid_textures[0],
                  c.trans(5.0, 80.0).scale(630.0/1944.0, 480.0/1944.0).transform, g);
            
            Rectangle::new([0.0, 0.0, 0.4, 1.0])
                .draw([1280.0 - 630.0 - 5.0, 80.0, 630.0, 480.0],
                      &c.draw_state, c.transform,
                      g);
            image(&vid_textures[1],
                  c.trans(1280.0 - 630.0 - 5.0, 80.0).scale(630.0/1944.0, 480.0/1944.0).transform, g);
        });
    }
}
