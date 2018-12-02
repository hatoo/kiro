extern crate clap;
extern crate termion;
extern crate unicode_width;

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
use unicode_width::UnicodeWidthChar;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// カーソルの位置　0-indexed
struct Cursor {
    row: usize,
    column: usize,
}

// エディタの内部状態
struct Kiro {
    /// テキスト本体
    /// buffer[i][j]はi行目のj列目の文字
    buffer: Vec<Vec<char>>,
    /// 現在のカーソルの位置
    /// self.cursor.row < self.buffer.len()
    /// self.cursor.column <= self.buffer[self.cursor.row].len()
    /// を常に保証する
    cursor: Cursor,
    /// 画面の一番上はバッファの何行目か
    /// スクロール処理に使う
    row_offset: usize,
    path: Option<path::PathBuf>,
}

impl Default for Kiro {
    fn default() -> Self {
        Self {
            buffer: vec![Vec::new()],
            cursor: Cursor { row: 0, column: 0 },
            row_offset: 0,
            path: None,
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

        self.path = Some(path.into());
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

        write!(out, "{}", clear::All).unwrap();
        write!(out, "{}", cursor::Goto(1, 1)).unwrap();

        // 画面上の行、列
        let mut row = 0;
        let mut col = 0;

        let mut display_cursor: Option<(usize, usize)> = None;

        'outer: for i in self.row_offset..self.buffer.len() {
            for j in 0..=self.buffer[i].len() {
                if self.cursor == (Cursor { row: i, column: j }) {
                    display_cursor = Some((row, col));
                }

                if let Some(c) = self.buffer[i].get(j) {
                    let width = c.width().unwrap_or(0);
                    if col + width >= cols {
                        row += 1;
                        col = 0;
                        if row >= rows {
                            break 'outer;
                        } else {
                            write!(out, "\r\n").unwrap();
                        }
                    }
                    write!(out, "{}", c).unwrap();
                    col += width;
                }
            }
            row += 1;
            col = 0;
            if row >= rows {
                break;
            } else {
                write!(out, "\r\n").unwrap();
            }
        }

        if let Some((r, c)) = display_cursor {
            write!(out, "{}", cursor::Goto(c as u16 + 1, r as u16 + 1)).unwrap();
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
    fn insert(&mut self, c: char) {
        if c == '\n' {
            // 改行
            let rest: Vec<char> = self.buffer[self.cursor.row]
                .drain(self.cursor.column..)
                .collect();
            self.buffer.insert(self.cursor.row + 1, rest);
            self.cursor.row += 1;
            self.cursor.column = 0;
            self.scroll();
        } else if !c.is_control() {
            self.buffer[self.cursor.row].insert(self.cursor.column, c);
            self.cursor_right();
        }
    }
    fn back_space(&mut self) {
        if self.cursor == (Cursor { row: 0, column: 0 }) {
            // 一番始めの位置の場合何もしない
            return;
        }

        if self.cursor.column == 0 {
            // 行の先頭
            let line = self.buffer.remove(self.cursor.row);
            self.cursor.row -= 1;
            self.cursor.column = self.buffer[self.cursor.row].len();
            self.buffer[self.cursor.row].extend(line.into_iter());
        } else {
            self.cursor_left();
            self.buffer[self.cursor.row].remove(self.cursor.column);
        }
    }
    fn delete(&mut self) {
        if self.cursor.row == self.buffer.len() - 1
            && self.cursor.column == self.buffer[self.cursor.row].len()
        {
            return;
        }

        if self.cursor.column == self.buffer[self.cursor.row].len() {
            // 行末
            let line = self.buffer.remove(self.cursor.row + 1);
            self.buffer[self.cursor.row].extend(line.into_iter());
        } else {
            self.buffer[self.cursor.row].remove(self.cursor.column);
        }
    }
    fn save(&self) {
        if let Some(path) = self.path.as_ref() {
            if let Ok(mut file) = fs::File::create(path) {
                for line in &self.buffer {
                    for &c in line {
                        write!(file, "{}", c).unwrap();
                    }
                    writeln!(file).unwrap();
                }
            }
        }
    }
}

fn main() {
    // Clap
    let matches = App::new("kiro")
        .about("A text editor")
        .bin_name("kiro")
        .arg(Arg::with_name("file").required(true))
        .get_matches();

    let file_path: &OsStr = matches.value_of_os("file").unwrap();

    let mut state = Kiro::default();

    state.open(path::Path::new(file_path));

    let stdin = stdin();
    let mut stdout = AlternateScreen::from(stdout().into_raw_mode().unwrap());

    state.draw(&mut stdout);

    for evt in stdin.events() {
        match evt.unwrap() {
            Event::Key(Key::Ctrl('c')) => {
                return;
            }
            Event::Key(Key::Ctrl('s')) => {
                state.save();
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
            Event::Key(Key::Char(c)) => {
                state.insert(c);
            }
            Event::Key(Key::Backspace) => {
                state.back_space();
            }
            Event::Key(Key::Delete) => {
                state.delete();
            }
            _ => {}
        }
        state.draw(&mut stdout);
    }
}
