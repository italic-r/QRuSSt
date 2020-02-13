#![allow(non_snake_case)]
#![allow(unused_imports)]

#[macro_use]
mod macros;
mod settings;

// external crate imports
use std::thread;
use std::sync::mpsc;
use std::path::PathBuf;
use std::f32::consts::PI;

use cpal;
use cpal::traits::*;

use shellexpand as se;

use gnuplot::*;
use sample::{
    signal,
    Signal
};
use rustfft::{
    FFT,
    FFTplanner,
    algorithm::Radix4,
    num_complex::Complex,
    num_traits::Zero
};

use gtk::{AboutDialog, ApplicationWindow, Builder, Button};
use gtk::prelude::*;
use gio::prelude::*;


fn _gtk_main() {
    if gtk::init().is_err() {
        println!("GTK+ init failure.");
        return;
    }

    // Read in UI template
    let builder = Builder::new_from_file("assets/QRuSSt.glade");

    // Windows
    let window_main: ApplicationWindow = builder.get_object("window_main").unwrap();
    let window_about: AboutDialog = builder.get_object("window_about").unwrap();

    // Extract widgets
    let button_about: Button = builder.get_object("button_about").unwrap();

    // Connect signals
    button_about.connect_clicked(clone!(window_about => move |_| {
        window_about.run();
        window_about.hide();
    }));

    // Finalize GTK+, show window, run program
    window_main.show_all();
    gtk::main();
}

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

fn _clap_main() {
    let opts = settings::clap_args(); // validate args
    println!("{:#?}", opts);
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
    let _hann = hann_window(inp.len());
    for i in 0..inp.len() {
        inp[i] *= _hann[i];
    }

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

    // GNUPLOT
    // requires simple data, no Complex<T>
    let mut fg = gnuplot::Figure::new();
    fg.axes2d()
        .set_title("fft", &[])
        .lines(
            0..f_high,
            &normalized[0..f_high],
            &[]);
    fg.show().unwrap();
    println!("{:?}", normalized);
}

fn hann_window(window_length: usize) -> Vec<f32> {
    (0..window_length).map(|n|
        (0.5 - (0.5 * (PI * n as f32 * 2. / (window_length as f32 - 1.)).sin())) * 2.
    ).collect()
}

fn main() {
    _clap_main();
}
