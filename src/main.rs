//! QRuSSt listens to weak radio signals in the audio domain.
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
use std::sync::{mpsc, Arc, Mutex, Condvar};
use std::thread;

// Audio
use cpal;
use cpal::traits::*;

// Data processing
use rustfft::{
    FftPlanner,
    num_complex::Complex,
};


// remain generic to use any available sample format from cpal
fn send_samples<T: cpal::Sample>(s: &[T], tx: &mpsc::Sender<Vec<T>>) {
    tx.send(Vec::from(s));
}

fn main() {
    // Set up logger
    let logger = Arc::new(logging::set_logger());

    // Read settings
    let opts = settings::clap_args();
    let set = Arc::new(Mutex::new(settings::Settings::default()));
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

    // audio data channel to FFT process thread
    let (tx, rx) = mpsc::channel();

    // opts->audio cvar
    let cvar_ui_stream_src = Arc::new((Mutex::new(false), Condvar::new()));
    let cvar_ui_stream_dest = cvar_ui_stream_src.clone();

    // FFT signaling to image thread
    let cvar_fft_img_src = Arc::new((Mutex::new(false), Condvar::new()));
    let cvar_fft_img_dest = cvar_fft_img_src.clone();

    let quit_condition: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
    gui::build_gtk(Arc::clone(&set), &logger, cvar_ui_stream_src, Arc::clone(&quit_condition));

    let thread_audio = thread::Builder::new()
        .name("audio_capture".to_string())
        .spawn(mclone!(logger, set, quit_condition => move || {
            let logger = logger.new(o!("thread" => format!("{}", thread::current().name().unwrap())));

            'restart_loop: loop {
                let tx = tx.clone();
                let (lock, cvar) = &*cvar_ui_stream_dest;

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
                                // TODO: How to handle stream error: error popup, stop stream, exit?
                            },
                        ) {
                            match stream.play() {
                                Ok(_) => {
                                    // Thread sleep must be in same block as `stream.play()`
                                    // to keep `stream` from going out of scope and closing
                                    let mut restart = lock.lock().unwrap();
                                    *restart = false;
                                    while !*restart {
                                        restart = cvar.wait(restart).unwrap();
                                    }
                                },
                                Err(e) => {
                                    error!(logger, "{:?}", e);
                                    // TODO: How to handle stream error: error popup, stop stream, exit?
                                },
                            }
                        }  // Ok(stream)
                    }  // Some(dev)
                }  // Ok(in_devices)
                if *quit_condition.lock().unwrap() {
                    debug!(logger, "breaking stream thread");
                    break 'restart_loop
                }
            }  // loop
        }));

    let thread_fft = thread::Builder::new()
        .name("fft_process".to_string())
        .spawn(mclone!(logger, set => move || {
            // constantly receiving data, notify image gen thread upon new processed data
            let logger = logger.new(o!("thread" => format!("{}", thread::current().name().unwrap())));

            'outer: loop {
                // get settings
                let img_x: usize = 1280;                                  // x in pixels
                let img_y: usize = 720;                                   // y in pixels

                let freq_min: usize = 200;
                let freq_max: usize = 2800;

                let frame_min: usize = 2;
                let frame_sec: usize = frame_min * 60;

                let sample_rate: usize = 48000;                           // device sample rate per second
                let samples_per_frame = sample_rate * frame_sec;          // samples per large time frame

                let samples_per_pixel: usize = samples_per_frame / img_x; // number of samples without overlap

                let overlap_percent: f32 = 0.33;
                let overlap_samples = (samples_per_pixel as f32 * overlap_percent).round() as usize;

                let window_size = samples_per_pixel + (overlap_samples * 2);
                let shift_size = window_size - overlap_samples;

                let nearest_pow_2: u32 = ((window_size as f32).ln() / 2_f32.ln()).ceil() as u32;
                let fft_size = 2_usize.pow(nearest_pow_2);

                // sample frequency ranges
                // most likely in drawing thread
                let freq_per_fft_samp = (sample_rate / 2) / (fft_size / 2 - 1);
                let sample_first = freq_per_fft_samp * freq_min;
                let sample_last = freq_per_fft_samp * freq_max;
                let samples_per_pixel = (sample_last - sample_first) / img_y;

                let mut buffer_proc_lrg: Vec<Vec<f32>> = Vec::new();                           // buffer for whole time slot
                let mut buffer_proc: Vec<Complex<f32>> = Vec::with_capacity(fft_size);         // buffer for windowed and FFT processed samples
                let mut buffer_raw:  Vec<f32> = Vec::with_capacity(window_size);               // buffer for unwindowed, unprocessed samples
                let mut fft_scratch: Vec<Complex<f32>> = vec![Complex::new(0., 0.); fft_size]; // scratch for fft processor

                // TODO: window will come from settings
                let window = settings::FftWindow::new(window_size, settings::FftWindowType::Hann);

                let mut planner = FftPlanner::new();
                let fft = planner.plan_fft_forward(fft_size);

                'rx: for d in &rx {
                    // sample processing
                    for s in d {
                        buffer_raw.push(s);
                        if buffer_raw.len() >= window_size {
                            buffer_proc.clear();
                            buffer_proc.append(
                                &mut buffer_raw.iter()
                                    .zip(&window.window)
                                    .map(|x| Complex::from(*x.0 * x.1))
                                    .collect());

                            buffer_proc.extend(vec![Complex::new(0., 0.); fft_size - window_size]);

                            fft.process_with_scratch(&mut buffer_proc, &mut fft_scratch);

                            // discard all frequencies after Nyquist (powers of 2 always even):
                            // N/2 for even number of input points (exactly Nyquist freq)
                            // (N-1)/2 for odd (last positive point)
                            buffer_proc.truncate(fft_size/2);

                            buffer_proc_lrg.push(
                                buffer_proc.iter().map(|x| x.norm() / (fft_size as f32).sqrt()
                            ).collect());
                            buffer_raw.rotate_left(shift_size);
                            buffer_raw.truncate(overlap_samples);

                            // notify image processor
                            let (lock, cvar) = &*cvar_fft_img_src;
                            let mut start = lock.lock().unwrap();
                            *start = true;
                            cvar.notify_one();
                        }
                    }

                    // rebuild FFT chain when audio settings change
                    // if settings_change {
                    //   continue 'outer
                    // }
                }
                // accessible when rx.iter() returns None, which only happens when Sender is dropped
                // otherwise 'outer is explicitly restarted
                debug!(logger, "breaking fft thread");
                break 'outer;
            }
    }));

    let thread_image = thread::Builder::new()
        .name("image".to_string())
        .spawn(mclone!(logger, quit_condition => move || {
            // wait until data to process is available, send render update to gui(or another place?)
            let logger = logger.new(o!("thread" => format!("{}", thread::current().name().unwrap())));
            debug!(logger, "image thread");

            loop {
                let (lock, cvar) = &*cvar_fft_img_dest;
                let mut start = lock.lock().unwrap();
                while !*start {
                    start = cvar.wait(start).unwrap();
                }
                if *quit_condition.lock().unwrap() {
                    debug!(logger, "breaking img thread");
                    break;
                }
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
