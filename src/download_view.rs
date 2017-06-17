/*
 * Copyright (c) 2016 Boucher, Antoni <bouanto@zoho.com>
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy of
 * this software and associated documentation files (the "Software"), to deal in
 * the Software without restriction, including without limitation the rights to
 * use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of
 * the Software, and to permit persons to whom the Software is furnished to do so,
 * subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS
 * FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR
 * COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER
 * IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
 * CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 */

/*
 * TODO: show errors.
 * TODO: add a command to download the current page.
 * TODO: add commands to cancel, delete (on disk), open, retry, remove from list, clear all
 * the list
 */

use std::ops::Deref;
use std::time::SystemTime;

use gtk;
use gtk::WidgetExt;
use number_prefix::{Prefixed, Standalone, binary_prefix};
use relm::{Relm, Widget};
use relm_attributes::widget;
use webkit2gtk::Download;

use urls;

use self::Msg::*;

pub struct Model {
    download: Download,
    id: u32,
    last_update: SystemTime,
    progress: f64,
    relm: Relm<DownloadView>,
    text: String,
    was_shown: bool,
}

#[derive(Msg)]
pub enum Msg {
    Finish,
    Update,
}

#[widget]
impl Widget for DownloadView {
    /// Show the data of a finished download.
    fn handle_finished(&mut self) {
        let filename = get_filename(&self.model.download);
        let percent = 100;
        self.model.progress = 1.0;
        let (_, total_size) = get_data_sizes(&self.model.download);
        let total_size = total_size.map(|size| format!(" [{}]", size)).unwrap_or_default();
        self.model.text = format!("{} {}%{}", filename, percent, total_size);
    }

    fn init_view(&mut self) {
        self.model.relm.stream().emit(Update);
        connect!(self.model.download, connect_received_data(download, _), self.model.relm, Update);
        connect!(self.model.download, connect_finished(download), self.model.relm, Finish);
    }

    fn model(relm: &Relm<DownloadView>, (id, download): (u32, Download)) -> Model {
        Model {
            download,
            id,
            last_update: SystemTime::now(),
            progress: 1.0,
            relm: relm.clone(),
            text: String::new(),
            was_shown: false,
        }
    }

    fn update(&mut self, msg: Msg) {
        match msg {
            Finish => self.handle_finished(),
            Update => self.update_progress_bar(),
        }
    }

    /// Update the progress and the text of the progress bar.
    fn update_progress_bar(&mut self) {
        let filename = get_filename(&self.model.download);
        self.model.progress = self.model.download.get_estimated_progress();
        let percent = (self.model.progress * 100.0) as i32;
        let (downloaded_size, total_size) = get_data_sizes(&self.model.download);
        // TODO: show the speed (downloaded data over the last 5 seconds).
        let mut updated = false;
        if percent == 100 {
            let total_size = total_size.map(|size| format!(" [{}]", size)).unwrap_or_default();
            self.model.text = format!("{} {}%{}", filename, percent, total_size);
        }
        else if let Ok(duration) = self.model.last_update.elapsed() {
            // Update the text once per second.
            if duration.as_secs() >= 1 || !self.model.was_shown {
                updated = true;
                let time_remaining = get_remaining_time(&self.model.download)
                    .map(|time| format!(", {}", time))
                    .unwrap_or_default();
                let total_size = total_size.map(|size| format!("/{}", size)).unwrap_or_default();
                self.model.text = format!("{} {}%{} [{}{}]", filename, percent, time_remaining, downloaded_size,
                    total_size);
                self.model.was_shown = true;
            }
        }
        if updated {
            self.model.last_update = SystemTime::now();
        }
    }

    view! {
        gtk::ProgressBar {
            fraction: self.model.progress,
            show_text: true,
            text: self.model.text.as_ref(),
        }
    }
}

impl DownloadView {
}

/// Get the destination filename of the download.
/// Return the suggested filename if it does not exist.
fn get_filename(download: &Download) -> String {
    let suggested_filename =
        download.get_request()
            .and_then(|request| request.get_uri())
            .and_then(|url| urls::get_filename(&url));
    download.get_destination()
        .and_then(|url| urls::get_filename(&url))
        .unwrap_or(suggested_filename.clone().unwrap_or_default())
}

/// Add the byte suffix with the right prefix.
/// For instance, convert 10 to "10B" and 5234 to "5.2KiB".
fn add_byte_suffix(number: f64) -> String {
    match binary_prefix(number) {
        Prefixed(suffix, number) => format!("{:.1}{}B", number, suffix),
        Standalone(bytes) => format!("{}B", bytes),
    }
}

/// Get the sizes bytes received and total bytes.
fn get_data_sizes(download: &Download) -> (String, Option<String>) {
    let progress = download.get_estimated_progress();
    if progress == 0.0 {
        (add_byte_suffix(progress), None)
    }
    else {
        let current = download.get_received_data_length() as f64;
        let total = current / progress;
        (add_byte_suffix(current), Some(add_byte_suffix(total)))
    }
}

/// Get the estimated remaining time.
fn get_remaining_time(download: &Download) -> Option<String> {
    let progress = download.get_estimated_progress();
    if progress == 0.0 {
        None
    }
    else {
        let elapsed_seconds = download.get_elapsed_time();
        let total_seconds = elapsed_seconds / progress;
        let seconds = total_seconds - elapsed_seconds;
        let minutes = (seconds / 60.0) as i32;
        let seconds = (seconds % 60.0) as i32;
        Some(format!("{}:{:02}", minutes, seconds))
    }
}
