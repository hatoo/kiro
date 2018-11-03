extern crate clap;
extern crate termion;

use clap::{App, Arg};
use std::cmp::{max, min};
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
    // 画面の一番上はバッファの何行目か
    row_offset: usize,
}

impl Default for Kiro {
    fn default() -> Self {
        Self {
            buffer: vec![Vec::new()],
            cursor: Cursor { row: 0, column: 0 },
            row_offset: 0,
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
        self.row_offset = 0;
    }
    fn terminal_size() -> (usize, usize) {
        let (cols, rows) = termion::terminal_size().unwrap();
        (rows as usize, cols as usize)
    }
    // 描画処理
    fn draw<T: Write>(&self, out: &mut T) {
        // 画面サイズ(文字数)
        let (rows, cols) = Self::terminal_size();

        write!(out, "{}", clear::All);
        write!(out, "{}", cursor::Goto(1, 1));

        // 画面上の行、列
        let mut row = 0;
        let mut col = 0;

        let mut display_cursor: Option<(usize, usize)> = None;

        'outer: for i in self.row_offset..self.buffer.len() {
            for j in 0..=self.buffer[i].len() {
                if self.cursor == (Cursor { row: i, column: j }) {
                    // 画面上のカーソルの位置がわかった
                    display_cursor = Some((row, col));
                }

                if let Some(c) = self.buffer[i].get(j) {
                    write!(out, "{}", c);
                    col += 1;
                    if col >= cols {
                        row += 1;
                        col = 0;
                        if row >= rows {
                            break 'outer;
                        } else {
                            // 最後の行の最後では改行すると1行ずれてしまうのでこのようなコードになっている
                            write!(out, "\r\n");
                        }
                    }
                }
            }
            row += 1;
            col = 0;
            if row >= rows {
                break;
            } else {
                // 最後の行の最後では改行すると1行ずれてしまうのでこのようなコードになっている
                write!(out, "\r\n");
            }
        }

        if let Some((r, c)) = display_cursor {
            write!(out, "{}", cursor::Goto(c as u16 + 1, r as u16 + 1));
        }

        out.flush().unwrap();
    }
    // カーソルが画面に映るようにする
    fn scroll(&mut self) {
        let (rows, _) = Self::terminal_size();
        self.row_offset = min(self.row_offset, self.cursor.row);
        if self.cursor.row + 1 >= rows {
            self.row_offset = max(self.row_offset, self.cursor.row + 1 - rows);
        }
    }
    fn cursor_up(&mut self) {
        if self.cursor.row > 0 {
            self.cursor.row -= 1;
            self.cursor.column = min(self.buffer[self.cursor.row].len(), self.cursor.column);
        }
        self.scroll();
    }
    fn cursor_down(&mut self) {
        if self.cursor.row + 1 < self.buffer.len() {
            self.cursor.row += 1;
            self.cursor.column = min(self.cursor.column, self.buffer[self.cursor.row].len());
        }
        self.scroll();
    }
    fn cursor_left(&mut self) {
        if self.cursor.column > 0 {
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
