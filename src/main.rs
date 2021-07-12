//! QRuSSt is an application to listen to weak radio signals in the audio domain.
//!
//! QRuSSt takes audio from a radio receiver and visualizes the audio spectrum in an image. Using
//! the FFT algorithm, a user may see a signal otherwise inaudible over the air.

#![allow(non_snake_case)]


#[macro_use]
mod macros;
mod gui;
mod settings;
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
use dasp::{Sample};
use rustfft::{
    FFT,
    FFTplanner,
    algorithm::Radix4,
    num_complex::Complex,
    num_traits::Zero
};


fn convert_samples<T: cpal::Sample>(s: &[T], tx: &mpsc::Sender<Vec<f32>>) {
    println!("converting samples");
    tx.send(s.clone().iter().map(|x| x.to_f32()).collect());
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

    let mut threads: Vec<_> = Vec::new();

    // channel
    let (tx, rx) = mpsc::channel::<Vec<f32>>();
    // opts->audio cvar
    let cvar_audio_ui = Arc::new((Mutex::new(false), Condvar::new()));
    let cvar_audio_ui2 = cvar_audio_ui.clone();

    let thread_audio = thread::Builder::new()
        .name("audio_capture".to_string())
        .spawn(mclone!(logger, set => move || {
            let logger = logger.new(o!("thread" => format!("{}", thread::current().name().unwrap())));

            loop {
                let tx = tx.clone();
                let (lock, cvar) = &*cvar_audio_ui2;
                let mut restart = lock.lock().unwrap();

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

                if let Ok(in_devices) = host.input_devices() {
                    let devs: Vec<cpal::Device> = in_devices
                        .filter(|d| d.name().unwrap() == *dev_name)
                        .collect();
                    if let Some(dev) = devs.get(0) {
                        info!(logger, "Device: {}", dev.name().unwrap());
                        if let Ok(stream) = dev.build_input_stream(
                            &cfg,
                            move |data, _cb| {
                                match data[0].FORMAT {
                                    cpal::SampleFormat::I16 => {tx.send(Vec::from(data));},
                                    cpal::SampleFormat::U16 => {tx.send(Vec::from(data));},
                                    cpal::SampleFormat::F32 => {tx.send(Vec::from(data));},
                                }
                            },
                            |error| {
                                //
                            },
                        ) {
                            stream.play();
                        }
                    }
                }
                while !*restart {
                    restart = cvar.wait(restart).unwrap();
                }
            }
        }));

    let thread_fft = thread::Builder::new()
        .name("fft_process".to_string())
        .spawn(mclone!(logger => move || {
            // constantly receiving data, notify image gen thread upon new processed data
            let logger = logger.new(o!("thread" => format!("{}", thread::current().name().unwrap())));
            debug!(logger, "fft thread");

            for d in rx {
                info!(logger, "{:?}", d);
            }

            // let (lock, cvar) = &*p_var_tx;
            // let mut start = lock.lock().unwrap();
            // *start = true;
            // cvar.notify_one();
    }));

    let thread_image = thread::Builder::new()
        .name("image".to_string())
        .spawn(mclone!(logger => move || {
            // wait until data to process is available, send render update to gui(or another place?)
            let logger = logger.new(o!("thread" => format!("{}", thread::current().name().unwrap())));
            debug!(logger, "image thread");

            // let (lock, cvar) = &*p_var_rx;
            // let mut start = lock.lock().unwrap();
            // while !*start {
            //     start = cvar.wait(start).unwrap();
            // }
    }));

    threads.push(thread_audio);
    threads.push(thread_fft);
    threads.push(thread_image);

    // Finalize program settings
    // Set up threads
    // Set up GTK widgets with settings
    // Run GTK
    // clean up threads on GTK exit
    gui::build_gtk(&mut set, &logger);

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
