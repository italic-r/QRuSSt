#![allow(non_snake_case)]
#![allow(unused_imports)]

#[macro_use]
mod macros;
mod settings;

// external crate imports
use std::thread;
use std::sync::mpsc;
use std::path::PathBuf;

use rodio;
use cpal;
use cpal::traits::*;
use structopt::StructOpt;

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
    let button_about:Button = builder.get_object("button_about").unwrap();

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

fn _structopt_main() {
    let mut set = settings::Settings::default();
    let opts = settings::Opts::from_args(); // TODO: validate args?
    if let Some(c) = opts.clone().config {
        set.config_path = c;
    }
    if let Some(e) = set.read_config().err() {
        // Warning dialog box (with file chooser?)
        println!("Read Error: {:?}", e);
    }
    if !opts.is_default() {
        if let Some(e) = set.set_override(opts.clone()).err() {
            println!("Override Error: {:?}", e);
        };
    } else {
        println!("No overrides. Continuing.");
    }

    println!("Settings: {:#?}", set);

    if opts.save_prefs {
        println!("Writing prefs...");
        if let Some(e) = set.write_config().err() {
            println!("Settings write error: {:?}", e);
        }
    }

    // _cpal_main();
}

fn _clap_main() {
    let opts = settings::clap_args();
    println!("{:#?}", opts);
}

fn main() {
    _clap_main()
}
