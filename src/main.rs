extern crate clap;
extern crate termion;

use clap::{App, Arg};
use std::cmp::min;
use std::ffi::OsStr;
use std::fs;
use std::io::{stdin, stdout, Write};
use std::path;
use termion::clear;
use termion::cursor;
use termion::event::{Event, Key};
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::screen::AlternateScreen;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
// カーソルの位置　0-indexed
struct Cursor {
    row: usize,
    column: usize,
}

// エディタの内部状態
struct Kiro {
    // テキスト本体
    buffer: Vec<Vec<char>>,
    // 現在のカーソルの位置
    cursor: Cursor,
}

impl Default for Kiro {
    fn default() -> Self {
        Self {
            buffer: vec![Vec::new()],
            cursor: Cursor { row: 0, column: 0 },
        }
    }
}

impl Kiro {
    // ファイルを読み込む
    fn open(&mut self, path: &path::Path) {
        self.buffer = fs::read_to_string(path)
            .ok()
            .map(|s| {
                let buffer: Vec<Vec<char>> = s
                    .lines()
                    .map(|line| line.trim_right().chars().collect())
                    .collect();
                if buffer.is_empty() {
                    vec![Vec::new()]
                } else {
                    buffer
                }
            })
            .unwrap_or(vec![Vec::new()]);

        self.cursor = Cursor { row: 0, column: 0 };
    }
    // 描画処理
    fn draw<T: Write>(&self, out: &mut T) {
        write!(out, "{}", clear::All);
        write!(out, "{}", cursor::Goto(1, 1));

        for line in &self.buffer {
            for &c in line {
                write!(out, "{}", c);
            }
            write!(out, "\r\n");
        }

        write!(
            out,
            "{}",
            cursor::Goto(self.cursor.column as u16 + 1, self.cursor.row as u16 + 1)
        );
        out.flush().unwrap();
    }
    fn cursor_up(&mut self) {
        if self.cursor.row > 0 {
            self.cursor.row -= 1;
            self.cursor.column = min(self.buffer[self.cursor.row].len(), self.cursor.column);
        }
    }
    fn cursor_down(&mut self) {
        if self.cursor.row + 1 < self.buffer.len() {
            self.cursor.row += 1;
            self.cursor.column = min(self.cursor.column, self.buffer[self.cursor.row].len());
        }
    }
    fn cursor_left(&mut self) {
        if self.cursor.column > 1 {
            self.cursor.column -= 1;
        }
    }
    fn cursor_right(&mut self) {
        self.cursor.column = min(self.cursor.column + 1, self.buffer[self.cursor.row].len());
    }
}

fn main() {
    // Clap
    let matches = App::new("kiro")
        .about("A text editor")
        .bin_name("kiro")
        .arg(Arg::with_name("file"))
        .get_matches();

    let file_path: Option<&OsStr> = matches.value_of_os("file");

    let mut state = Kiro::default();

    if let Some(file_path) = file_path {
        state.open(path::Path::new(file_path));
    }

    let stdin = stdin();
    let mut stdout = AlternateScreen::from(stdout().into_raw_mode().unwrap());

    state.draw(&mut stdout);

    for evt in stdin.events() {
        match evt.unwrap() {
            Event::Key(Key::Ctrl('c')) => {
                return;
            }
            Event::Key(Key::Up) => {
                state.cursor_up();
            }
            Event::Key(Key::Down) => {
                state.cursor_down();
            }
            Event::Key(Key::Left) => {
                state.cursor_left();
            }
            Event::Key(Key::Right) => {
                state.cursor_right();
            }
            _ => {}
        }
        state.draw(&mut stdout);
    }
}
