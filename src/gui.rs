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
    let combo_devices:   ComboBox          = builder.get_object("combo_devices").unwrap();
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

    // Extract internal lists
    let list_devices:    ListStore         = builder.get_object("dev_list").unwrap();
    let entry_dev:      Entry             = builder.get_object("dev_list_entry").unwrap();

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

    combo_devices.connect_changed(clone!(@strong logger,
            @strong combo_devices
            => move |_| {
        debug!(logger, "ComboBox index: {:?}", combo_devices.get_active());
    }));

    entry_dev.connect_changed(clone!(@strong logger,
            @strong entry_dev
            => move |_| {
        debug!(logger, "Selected entry: {:?}", entry_dev.get_text().unwrap().as_str());
    }));

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
