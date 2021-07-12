/// Build and init GTK GUI


use std::str::FromStr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use super::settings;

// GTK+
use glib::clone;
use gtk::{
    AboutDialog,
    ApplicationWindow,
    Builder,
    Button,
    CheckButton,
    ComboBox,
    Entry,
    FileChooserButton,
    ListStore,
    Popover,
    SpinButton,
};
use gtk::prelude::*;
// use gio::prelude::*;

// Logging
use slog;

// Audio
use cpal;
use cpal::traits::*;

pub (crate) fn build_gtk(set: &mut Arc<Mutex<settings::Settings>>, logger: &slog::Logger) {
    if gtk::init().is_err() {
        crit!(logger, "GTK+ init failure.");
        return;
    }

    // Read in UI template
    // TODO: ensure asset found during distribution - may need include_str!()
    let builder: Builder = Builder::new_from_file("assets/QRuSSt.glade");

    // Windows
    let window_main:     ApplicationWindow = builder.get_object("window_main").unwrap();
    let window_about:    AboutDialog       = builder.get_object("window_about").unwrap();
    let window_settings: Popover           = builder.get_object("window_settings").unwrap();

    // Extract Widgets
    let button_about:    Button            = builder.get_object("button_about").unwrap();
    let button_help:     Button            = builder.get_object("button_help").unwrap();
    let _button_options: Button            = builder.get_object("button_options").unwrap();

    // Extract Settings
    let _combo_devices:  ComboBox          = builder.get_object("combo_devices").unwrap();
    let list_devices:    ListStore         = builder.get_object("list_dev").unwrap();
    let entry_dev:       Entry             = builder.get_object("entry_dev").unwrap();

    let _combo_rate:     ComboBox          = builder.get_object("combo_rate").unwrap();
    let list_rate:       ListStore         = builder.get_object("list_rate").unwrap();
    let entry_rate:      Entry             = builder.get_object("entry_rate").unwrap();

    let _combo_format:   ComboBox          = builder.get_object("combo_format").unwrap();
    let list_format:     ListStore         = builder.get_object("list_format").unwrap();
    // let entry_format:    Entry             = builder.get_object("entry_format").unwrap();

    let spin_freq_min:   SpinButton        = builder.get_object("spin_freq_min").unwrap();
    let spin_freq_max:   SpinButton        = builder.get_object("spin_freq_max").unwrap();

    let spin_brightness: SpinButton        = builder.get_object("spin_brightness").unwrap();
    let spin_contrast:   SpinButton        = builder.get_object("spin_contrast").unwrap();

    let check_win_xy:    CheckButton       = builder.get_object("check_use_window_dim").unwrap();
    let spin_width:      SpinButton        = builder.get_object("image_width").unwrap();
    let spin_height:     SpinButton        = builder.get_object("image_height").unwrap();

    let check_export:    CheckButton       = builder.get_object("export_images").unwrap();

    let check_single:    CheckButton       = builder.get_object("check_single").unwrap();
    let check_average:   CheckButton       = builder.get_object("check_average").unwrap();
    let check_peak:      CheckButton       = builder.get_object("check_peak").unwrap();
    let check_hour:      CheckButton       = builder.get_object("check_hour").unwrap();
    let check_day:       CheckButton       = builder.get_object("check_day").unwrap();

    let entry_single:    Entry             = builder.get_object("input_single").unwrap();
    let entry_average:   Entry             = builder.get_object("input_average").unwrap();
    let entry_peak:      Entry             = builder.get_object("input_peak").unwrap();
    let entry_hour:      Entry             = builder.get_object("input_hour").unwrap();
    let entry_day:       Entry             = builder.get_object("input_day").unwrap();

    let file_chooser:    FileChooserButton = builder.get_object("settings_filechooser").unwrap();

    for e in &["16000", "32000", "44100", "48000", "96000", "19200"] {
        list_rate.insert_with_values(None, &[0], &[e]);
    }

    for e in &["i16", "u16", "f32"] {
        list_format.insert_with_values(None, &[0], &[e]);
    }

    // Load settings into UI
    {
        let set = set.lock().unwrap();
        entry_dev      .set_text(&set.audio.device);
        entry_rate     .set_text(&format!("{}", set.audio.rate));
        // entry_format   .set_text(match &set.audio.format {
        //     settings::AudioFormat::i16 => "i16",
        //     settings::AudioFormat::u16 => "u16",
        //     settings::AudioFormat::f32 => "f32",
        // });
        spin_freq_min  .set_value(set.audio.freq_range.0 as f64);
        spin_freq_max  .set_value(set.audio.freq_range.1 as f64);
        spin_brightness.set_value(set.image.brightness as f64);
        spin_contrast  .set_value(set.image.contrast as f64);
        check_win_xy   .set_active(set.image.use_window_xy);
        spin_width     .set_value(set.image.dimensions.0 as f64);
        spin_height    .set_value(set.image.dimensions.1 as f64);
        check_export   .set_active(set.export.export_enable);
        check_single   .set_active(set.export.single);
        check_average  .set_active(set.export.average);
        check_peak     .set_active(set.export.peak);
        check_hour     .set_active(set.export.hour);
        check_day      .set_active(set.export.day);
        entry_single   .set_text(&set.names.single);
        entry_average  .set_text(&set.names.average);
        entry_peak     .set_text(&set.names.peak);
        entry_hour     .set_text(&set.names.hour);
        entry_day      .set_text(&set.names.day);
        file_chooser   .set_uri(&set.export.path.to_str().unwrap());
    }

    // Connect signals
    button_about.connect_clicked(clone!(@strong logger, @strong window_about
            => move |_| {
        // TODO: will the rest of the program still run during this closure?
        debug!(logger, "About window opened");
        window_about.run();
        window_about.hide();
    }));

    button_help.connect_clicked(clone!(@strong logger => move |_| {
        debug!(logger, "Help clicked");
    }));

    entry_dev.connect_changed(clone!(@strong logger,
            @strong entry_dev
            => move |_| {
        let name = entry_dev.get_text().unwrap();
        debug!(logger, "Selected entry: {:?}", name.as_str());
        // TODO: save device object
    }));

    entry_rate.connect_changed(clone!(@strong logger, @strong set,
            @strong entry_rate
            => move |_| {
        // Parsing cannot fail due to hardcoded available values
        let _rate = entry_rate.get_text().unwrap();
        let rate: u32 = _rate.parse().unwrap();
        let mut set = set.lock().unwrap();
        set.audio.rate = rate;
        debug!(logger, "Selected rate: {}", set.audio.rate);
    }));

    /*
    entry_format.connect_changed(clone!(@strong logger, @strong set,
            @strong entry_format
            => move |_| {
        let _format = entry_format.get_text().unwrap();
        let format = match _format.as_str() {
            "i16" => settings::AudioFormat::i16,
            "u16" => settings::AudioFormat::u16,
            "f32" => settings::AudioFormat::f32,
            _ => unreachable!(),
        };
        let mut set = set.lock().unwrap();
        set.audio.format = format;
        debug!(logger, "Selected format: {:?}", set.audio.format);
    }));
    */

    check_export.connect_toggled(clone!(@strong logger, @strong set,
            @strong check_export
            => move |_| {
        let mut set = set.lock().unwrap();
        set.export.export_enable = check_export.get_active();
        debug!(logger, "Export enabled: {:?}", set.export.export_enable);
    }));

    // EXPORT
    check_single.connect_toggled(clone!(@strong logger, @strong set,
            @strong check_single
            => move |_| {
        let mut set = set.lock().unwrap();
        set.export.single = check_single.get_active();
        debug!(logger, "Export single: {:?}", set.export.single);
    }));

    check_average.connect_toggled(clone!(@strong logger, @strong set,
            @strong check_average
            => move |_| {
        let mut set = set.lock().unwrap();
        set.export.average = check_average.get_active();
        debug!(logger, "Export average: {:?}", set.export.average);
    }));

    check_peak.connect_toggled(clone!(@strong logger, @strong set,
            @strong check_peak
            => move |_| {
        let mut set = set.lock().unwrap();
        set.export.peak = check_peak.get_active();
        debug!(logger, "Export peak: {:?}", set.export.peak);
    }));

    check_hour.connect_toggled(clone!(@strong logger, @strong set,
            @strong check_hour
            => move |_| {
        let mut set = set.lock().unwrap();
        set.export.hour = check_hour.get_active();
        debug!(logger, "Export hour: {:?}", set.export.hour);
    }));

    check_day.connect_toggled(clone!(@strong logger, @strong set,
            @strong check_day
            => move |_| {
        let mut set = set.lock().unwrap();
        set.export.day = check_day.get_active();
        debug!(logger, "Export day: {:?}", set.export.day);
    }));

    // NAMES
    entry_single.connect_changed(clone!(@strong logger, @strong set,
            @strong entry_single
            => move |_| {
        let mut set = set.lock().unwrap();
        set.names.single = entry_single.get_text().unwrap().to_string();
        debug!(logger, "Single name: {:?}", set.names.single);
    }));

    entry_average.connect_changed(clone!(@strong logger, @strong set,
            @strong entry_average
            => move |_| {
        let mut set = set.lock().unwrap();
        set.names.average = entry_average.get_text().unwrap().to_string();
        debug!(logger, "Average name: {:?}", set.names.average);
    }));

    entry_peak.connect_changed(clone!(@strong logger, @strong set,
            @strong entry_peak
            => move |_| {
        let mut set = set.lock().unwrap();
        set.names.peak = entry_peak.get_text().unwrap().to_string();
        debug!(logger, "Peak name: {:?}", set.names.peak);
    }));

    entry_hour.connect_changed(clone!(@strong logger, @strong set,
            @strong entry_hour
            => move |_| {
        let mut set = set.lock().unwrap();
        set.names.hour = entry_hour.get_text().unwrap().to_string();
        debug!(logger, "Hour name: {:?}", set.names.hour);
    }));

    entry_day.connect_changed(clone!(@strong logger, @strong set,
            @strong entry_day
            => move |_| {
        let mut set = set.lock().unwrap();
        set.names.day = entry_day.get_text().unwrap().to_string();
        debug!(logger, "Day name: {:?}", set.names.day);
    }));

    // IMAGE
    spin_width.connect_value_changed(clone!(@strong logger, @strong set,
            @strong spin_width
            => move |_| {
        let mut set = set.lock().unwrap();
        set.image.dimensions = (spin_width.get_value() as u16, set.image.dimensions.1);
        debug!(logger, "Width: {:?}", set.image.dimensions);
    }));

    spin_height.connect_value_changed(clone!(@strong logger, @strong set,
            @strong spin_height
            => move |_| {
        let mut set = set.lock().unwrap();
        set.image.dimensions = (set.image.dimensions.0, spin_height.get_value() as u16);
        debug!(logger, "Width: {:?}", set.image.dimensions);
    }));

    spin_freq_min.connect_value_changed(clone!(@strong logger, @strong set,
            @strong spin_freq_min
            => move |_| {
        let mut set = set.lock().unwrap();
        set.audio.freq_range = (
            spin_freq_min.get_value()  as u16,
            set.audio.freq_range.1 as u16);
        debug!(logger, "Set frequency range: {:?}", set.audio.freq_range);
    }));

    spin_freq_max.connect_value_changed(clone!(@strong logger, @strong set,
            @strong spin_freq_max
            => move |_| {
        let mut set = set.lock().unwrap();
        set.audio.freq_range = (
            set.audio.freq_range.0 as u16,
            spin_freq_max.get_value()  as u16);
        debug!(logger, "Set frequency range: {:?}", set.audio.freq_range);
    }));

    spin_brightness.connect_value_changed(clone!(@strong logger, @strong set,
            @strong spin_brightness
            => move |_| {
        let mut set = set.lock().unwrap();
        set.image.brightness = spin_brightness.get_value() as u8;
        debug!(logger, "Brightness: {}", set.image.brightness);
    }));

    spin_contrast.connect_value_changed(clone!(@strong logger, @strong set,
            @strong spin_contrast
            => move |_| {
        let mut set = set.lock().unwrap();
        set.image.contrast = spin_contrast.get_value() as u8;
        debug!(logger, "Contrast: {}", set.image.contrast);
    }));

    check_win_xy.connect_toggled(clone!(@strong logger, @strong set,
            @strong check_win_xy
            => move |_| {
        let mut set = set.lock().unwrap();
        set.image.use_window_xy = check_win_xy.get_active();
        debug!(logger, "Use window dimensions: {}", set.image.use_window_xy);
    }));

    file_chooser.connect_file_set(clone!(
            @strong logger, @strong set,
            @strong file_chooser
            => move |_| {
        let mut set = set.lock().unwrap();
        // TODO: unwrap()
        set.export.path = PathBuf::from_str(&file_chooser.get_uri().unwrap()).unwrap();
        debug!(logger, "File save path: {:?}", set.export.path)
    }));

    window_settings.connect_show(clone!(@strong logger,
            @strong list_devices
            => move |_| {
        debug!(logger, "Settings opened");
        // TODO: latency when opening window is too high. move to program init.
        list_devices.clear();
        let host = cpal::default_host();
        let c_devices: Vec<cpal::Device> = host.devices().unwrap().collect();
        for dev in c_devices {
            let name = &dev.name().unwrap();
            debug!(logger, "{}", name);
            list_devices.insert_with_values(None, &[0], &[name]);
        }
    }));

    // save prefs at popover close
    window_settings.connect_closed(clone!(@strong logger, @strong set
            => move |_| {
        debug!(logger, "Prefs closed");
    }));

    // quit application when window closed
    window_main.connect_delete_event(clone!(@strong logger => move |_, _| {
        debug!(logger, "Quitting...");
        gtk::main_quit();
        Inhibit(false)
    }));

    // Finalize GTK+, show window
    window_main.show_all();
}
