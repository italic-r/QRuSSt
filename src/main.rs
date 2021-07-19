//! QRuSSt is an application to listen to weak radio signals in the audio domain.
//!
//! QRuSSt takes audio from a radio receiver and visualizes the audio spectrum in an image. Using
//! the FFT algorithm, a user may see a signal otherwise inaudible over the air.

#![allow(non_snake_case)]


#[macro_use]
mod macros;
mod gui;
mod settings;
mod windows;
mod logging;

#[macro_use]
extern crate slog;

// std
use std::sync::{Arc, Mutex, Condvar};
// use std::rc::Rc;
use std::thread;
use std::sync::mpsc;
// use std::time::Duration;

// Audio
use cpal;
use cpal::traits::*;

// use shellexpand as se;

// Data processing
// use gnuplot::*;
use dasp::{Sample};  // for window functions
use rustfft::{
    Fft,
    FftPlanner,
    algorithm::Radix4,
    num_complex::Complex,
    num_traits::Zero
};


fn send_samples<T: cpal::Sample>(s: &[T], tx: &mpsc::Sender<Vec<T>>) {
    tx.send(Vec::from(s));
}

fn main() {
    // Set up logger
    let logger = Arc::new(logging::set_logger());

    // Read settings
    let opts = settings::clap_args();
    let mut set = Arc::new(Mutex::new(settings::Settings::default()));
    if let Some(c) = opts.value_of("config") {
        let mut set = set.lock().unwrap();
        set.config = c.into();
    }
    {
        let mut set = set.lock().unwrap();

        if set.read_config().is_err() {
            error!(logger, "Error reading config");
        }
        if set.arg_override(&opts).is_err() {
            error!(logger, "Error overriding config");
        }
        if opts.is_present("save_prefs") {
            if set.write_config().is_err() {
                error!(logger, "Error writing config");
            }
        }
    }

    gui::build_gtk(&mut set, &logger);

    // audio data channel to FFT process thread
    let (tx, rx) = mpsc::channel();

    // opts->audio cvar
    let cvar_ui_to_stream_src = Arc::new((Mutex::new(false), Condvar::new()));
    let cvar_ui_to_stream_dest = cvar_ui_to_stream_src.clone();

    // FFT signaling to image thread
    let cvar_fft_to_img_src = Arc::new((Mutex::new(false), Condvar::new()));
    let cvar_fft_to_img_dest = cvar_fft_to_img_src.clone();

    let thread_audio = thread::Builder::new()
        .name("audio_capture".to_string())
        .spawn(mclone!(logger, set => move || {
            let logger = logger.new(o!("thread" => format!("{}", thread::current().name().unwrap())));

            loop {
                let tx = tx.clone();
                let (lock, cvar) = &*cvar_ui_to_stream_dest;

                let set = set.lock().unwrap();
                let dev_name = &set.audio.device;
                // TODO: hardcoded channel count - only good for SSB audio (not IQ)
                let channels: cpal::ChannelCount = 1;
                let cfg = cpal::StreamConfig {
                    channels,
                    sample_rate: cpal::SampleRate(set.audio.rate),
                    buffer_size: cpal::BufferSize::Default,
                };

                let host = cpal::default_host();

                // TODO: Error handling
                if let Ok(in_devices) = host.input_devices() {
                    let devs: Vec<cpal::Device> = in_devices
                        .filter(|d| d.name().unwrap() == *dev_name)
                        .collect();
                    if let Some(dev) = devs.get(0) {
                        info!(logger, "Device: {}", dev.name().unwrap());
                        let log_inner = logger.new(o!("thread" => format!("{}", thread::current().name().unwrap())));
                        if let Ok(stream) = dev.build_input_stream(
                            &cfg,
                            move |data, _cb| {
                                send_samples::<f32>(data, &tx);
                            },
                            move |error| {
                                debug!(log_inner, "{:?}", error);
                                // How to handle stream error: error popup, stop stream, exit?
                            },
                        ) {
                            match stream.play() {
                                Ok(_) => {
                                    // Thread sleep must be in same block as `stream.play()`
                                    // to keep `stream` from going out of scope and closing
                                    let mut restart = lock.lock().unwrap();
                                    *restart = false;
                                    while !*restart {
                                        debug!(logger, "Restart condition");
                                        restart = cvar.wait(restart).unwrap();
                                    }
                                },
                                Err(e) => {
                                    error!(logger, "{:?}", e);
                                    continue;
                                },
                            }
                        }  // Ok(stream)
                    }  // Some(dev)
                }  // Ok(in_devices)
            }  // loop
        }));

    let thread_fft = thread::Builder::new()
        .name("fft_process".to_string())
        .spawn(mclone!(logger => move || {
            // constantly receiving data, notify image gen thread upon new processed data
            let logger = logger.new(o!("thread" => format!("{}", thread::current().name().unwrap())));
            debug!(logger, "fft thread");

            'outer: loop {
                // get settings
                let img_x: usize = 1280;                                    // x in pixels
                let img_y: usize = 720;                                     // y in pixels

                let frame_min: usize = 2;
                let frame_sec: usize = frame_min * 60;

                let sample_rate: usize = 48000;                             // device sample rate per second
                let samples_per_frame = sample_rate * frame_sec;            // samples per large time frame

                let samples_per_pixel: usize = samples_per_frame / img_x;   // number of samples without overlap

                let overlap_percent: f32 = 0.33;

                let overlap_samples = (samples_per_pixel as f32 * overlap_percent).round() as usize;
                let window_size = samples_per_pixel + (overlap_samples * 2);
                let shift_size = window_size - overlap_samples;

                let mut large_buffer: Vec<Vec<f32>> = Vec::new();           // buffer for whole time slot
                let mut buffer: Vec<f32> = Vec::with_capacity(window_size); // buffer for each pixel

                debug!(logger, "Samples per pixel: {}", samples_per_pixel);
                debug!(logger, "Window size: {}", window_size);

                'rx: for d in &rx {
                    // sample processing

                    // when buffer full, notify image processor
                    if buffer.len() >= window_size {
                        let (lock, cvar) = &*cvar_fft_to_img_src;
                        let mut start = lock.lock().unwrap();
                        *start = true;
                        cvar.notify_one();
                    }
                    // if cond_changes {
                    //   continue 'outer
                    // }
                }
            }


            // 'outer loop {
            //   time_frame, sample_rate, window_size, storage_vec, overlap_len
            //   'rx loop {
            //     storage_vec.push()
            //     if storage_vec >= window_size
            //       process_fft(storage_vec)
            //       notify_img_thread()
            //       shift(storage_vec, window_size - overlap_len)
            //       continue 'rx
            //     if time_frame, sample_rate, etc change size:
            //       restart 'outer
            //   }
            // }
    }));

    let thread_image = thread::Builder::new()
        .name("image".to_string())
        .spawn(mclone!(logger => move || {
            // wait until data to process is available, send render update to gui(or another place?)
            let logger = logger.new(o!("thread" => format!("{}", thread::current().name().unwrap())));
            debug!(logger, "image thread");

            let (lock, cvar) = &*cvar_fft_to_img_dest;
            let mut start = lock.lock().unwrap();
            while !*start {
                start = cvar.wait(start).unwrap();
            }
    }));

    let mut threads: Vec<_> = Vec::new();
    threads.push(thread_audio);
    threads.push(thread_fft);
    threads.push(thread_image);

    // Finalize program settings
    // Set up threads
    // Set up GTK widgets with settings
    // Run GTK
    // clean up threads on GTK exit

    // tx, rx
    //      tx -> audio capture thread
    //      rx -> fft process thread
    // fft process thread writes processed data to Arc<Mutex<Vec<fft_data>>>
    // image rendering thread processes Arc<Mutex<Vec<fft_data>>> into image (redraw whole image with
    //     current-time cursor and time marker ticks)
    // who controls timeframe: render thread, main thread?
    // always render to window size, save image files at chosen resolution

    gtk::main();

    for t in threads {
        if let Ok(thr) = t {
            thr.join().unwrap();
        }
    }
    debug!(logger, "Quit success");
}
