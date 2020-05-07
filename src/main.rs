//! QRuSSt is an application to listen to weak radio signals in the audio domain.
//!
//! QRuSSt takes audio from a radio receiver and visualizes the audio spectrum in an image. Using
//! the FFT algorithm, a user may see a signal otherwise inaudible over the air.


mod gui;
mod settings;
mod logging;

#[macro_use]
extern crate slog;

// std
use std::sync::{Arc, Mutex};
use std::rc::Rc;
use std::thread;
use std::sync::mpsc;

// Audio
use cpal;
use cpal::traits::*;

// use shellexpand as se;

// Data processing
/*
use gnuplot::*;
*/
// use sample::{signal, Signal};
use rustfft::{
    FFT,
    FFTplanner,
    algorithm::Radix4,
    num_complex::Complex,
    num_traits::Zero
};


/*
fn _cpal_main() {
    let (tx, rx) = mpsc::channel();

    let host = cpal::default_host();
    let event_loop = host.event_loop();
    let c_devices: Vec<cpal::Device> = host.devices().unwrap()
        .filter(|x| x.name().unwrap() == "SIG_AUTO").collect();
    let format = cpal::Format{
        channels:    1,
        sample_rate: cpal::SampleRate(48000),
        data_type:   cpal::SampleFormat::I16};
    let _s = event_loop.build_input_stream(&c_devices[0], &format).unwrap();

    let _thread = thread::spawn(move || {
        event_loop.run(move |_stream_id, stream_result| {
            let stream_data = match stream_result {
                Ok(data) => data,
                Err(err) => {println!("{}", err); return;},
            };
            if let cpal::StreamData::Input {
                buffer: cpal::UnknownTypeInputBuffer::I16(buffer)
            } = stream_data {
                tx.send(buffer.iter().map(|e| *e).collect::<Vec<i16>>()).unwrap();
            }
        });
    });

    for pack in rx {
        println!("packet length: {}", pack.len());
        println!("{:?}", pack);
    }
}

fn _fft_main() {
    let fft_inverse = false;
    let fft_size = 32768;
    let samp_rate = 32768.;
    let samp_len = fft_size;
    let _ex_len = 2048;

    // generate waveform
    let _noise: Vec<f32> = signal::noise(12)                             .take(samp_len).map(|val| val[0] as f32).collect();
    let _sig1:  Vec<f32> = signal::rate(samp_rate).const_hz( 600.).sine().take(samp_len).map(|val| val[0] as f32).collect();
    let _sig2:  Vec<f32> = signal::rate(samp_rate).const_hz(1500.).sine().take(samp_len).map(|val| val[0] as f32).collect();

    println!("_sig1 len: {}", _sig1.len());

    let _join1: Vec<f32> = _sig1
        .iter()
        .zip(_sig2.iter())
        .map(|tup| tup.0 + tup.1)
        .collect();
    let _join2: Vec<f32> = _join1
        .iter()
        .zip(_noise.iter())
        .map(|tup| tup.0 + tup.1)
        .collect();

    // make complex data for fft
    let mut inp: Vec<Complex<f32>> = _join2
        .iter()
        .map(|val| Complex::from(val))
        .collect();
    let mut output = vec![Zero::zero(); samp_len];

    // window filter on input data before FFT
    // let _hann = hann_window(inp.len());
    // for i in 0..inp.len() {
    //     inp[i] *= _hann[i];
    // }

    // automatic fft
    let mut planner = FFTplanner::new(fft_inverse);
    let fft = planner.plan_fft(samp_len);
    fft.process_multi(&mut inp, &mut output);

    let out_graph = output
        .iter()
        .map(|val| val.norm() / fft.len() as f32);

    // GNUPLOT
    // requires simple data, no Complex<T>
    let mut fg = gnuplot::Figure::new();
    fg.axes2d()
        .set_title("fft", &[])
        .lines(
            1.._ex_len,
            out_graph,
            &[]);
    fg.show().unwrap();
}
*/

fn _cpal_fft() {
    let fft_inverse = false;
    let fft_size = 65536;
    let sample_rate = 48000;
    let f_low = 200;
    let f_high = 3000;

    let mut buffer = Vec::new();

    let (tx, rx) = mpsc::channel();

    let host = cpal::default_host();
    let event_loop = host.event_loop();
    let c_devices: Vec<cpal::Device> = host.devices().unwrap()
        .filter(|x| x.name().unwrap() == "SIG_AUTO").collect();
    let format = cpal::Format{
        channels:    1,
        sample_rate: cpal::SampleRate(sample_rate),
        data_type:   cpal::SampleFormat::I16};
    let _s = event_loop.build_input_stream(&c_devices[0], &format).unwrap();

    let _thread = thread::spawn(move || {
        event_loop.run(move |_stream_id, stream_result| {
            let stream_data = match stream_result {
                Ok(data) => data,
                Err(err) => {println!("{}", err); return;},
            };
            if let cpal::StreamData::Input {
                buffer: cpal::UnknownTypeInputBuffer::I16(buffer)
            } = stream_data {
                tx.send(buffer.iter().map(|e| *e).collect::<Vec<i16>>()).unwrap();
            }
        });
    });

    for mut pack in rx {
        buffer.append(&mut pack);
        if buffer.len() > fft_size {
            buffer.truncate(fft_size);
            break;
        }
    }

    let mut input: Vec<Complex<f32>> = buffer
        .iter()
        .map(|&val| Complex::from(val as f32))
        .collect();
    let mut output = vec![Zero::zero(); fft_size];

    let mut planner = FFTplanner::new(fft_inverse);
    let fft = planner.plan_fft(fft_size);
    fft.process_multi(&mut input, &mut output);

    let mut normalized: Vec<f32> = output.iter().map(|val| val.norm() / (fft.len() as f32)).collect();
    normalized.truncate(f_high);
    println!("{:?}", normalized);

    // GNUPLOT
    // requires simple data, no Complex<T>
    // let mut fg = gnuplot::Figure::new();
    // fg.axes2d()
    //     .set_title("fft", &[])
    //     .lines(
    //         0..f_high,
    //         &normalized[0..f_high],
    //         &[]);
    // fg.show().unwrap();
}

fn main() {
    // Set up logger
    let logger = Rc::new(logging::set_logger());

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

    // Set up GTK widgets with settings
    // Set up threads
    // Run GTK
    gui::build_gtk(&mut set, &logger);

    // tx, rx, send tx to audio capture thread
    // capture thread sends data to fft process thread
    // fft process thread writes processed data to Mutex<Vec<fft_data>>
    // image rendering thread processes Mutex<Vec<fft_data>> into image (redraw whole image with
    //     current-time cursor and time marker ticks)

    gtk::main();

    debug!(logger, "Quit success");
}
