extern crate termion;

use std::io::{stdin, stdout, Write};
use std::sync::mpsc::channel;
use std::thread;
use std::time::Duration;
use termion::event::{Event, Key};
use termion::input::{MouseTerminal, TermRead};
use termion::raw::IntoRawMode;
use termion::screen::AlternateScreen;

fn main() {
    let stdin = stdin();
    let mut stdout = MouseTerminal::from(AlternateScreen::from(stdout()).into_raw_mode().unwrap());
    let (tx, rx) = channel();

    thread::spawn(move || {
        for c in stdin.events() {
            if let Ok(evt) = c {
                tx.send(evt).unwrap();
            }
        }
    });

    loop {
        if let Ok(evt) = rx.recv_timeout(Duration::from_millis(16)) {
            if evt == Event::Key(Key::Ctrl('c')) {
                return;
            }
        }
        stdout.flush().unwrap();
    }
}
