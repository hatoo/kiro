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

fn main() {
    // Clap
    let matches = App::new("kiro")
        .about("A text editor")
        .bin_name("kiro")
        .arg(Arg::with_name("file"))
        .get_matches();

    // ファイルパスはUTF-8でない可能性があるのでOsStrを使います
    let file_path: Option<&OsStr> = matches.value_of_os("file");

    // テキストを読み込む
    // 改行コードに関してはlinesに一任している
    let buffer: Vec<Vec<char>> = file_path
        .and_then(|file_path| {
            // エラー処理は適当
            fs::read_to_string(path::Path::new(file_path))
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
        })
        .unwrap_or(vec![Vec::new()]);

    let mut cursor = Cursor { row: 0, column: 0 };

    let stdin = stdin();
    // Rawモードに移行
    // into_raw_modeはIntoRawModeトレイトに定義されている
    // めんどくさいので失敗時は終了(unwrap)
    // stdout変数がDropするときにrawモードから元の状態にもどる
    let mut stdout = AlternateScreen::from(stdout().into_raw_mode().unwrap());

    // 画面全体をクリアする
    write!(stdout, "{}", clear::All);
    // カーソルを左上に設定する(1-indexed)
    write!(stdout, "{}", cursor::Goto(1, 1));

    // bufferの内容を出力する
    for line in &buffer {
        for &c in line {
            write!(stdout, "{}", c);
        }
        // Rawモードでは改行は\r\nで行う
        write!(stdout, "\r\n");
    }

    write!(stdout, "{}", cursor::Goto(1, 1));
    // フラッシュする
    stdout.flush().unwrap();

    // eventsはTermReadトレイトに定義されている
    for evt in stdin.events() {
        match evt.unwrap() {
            // Ctrl-cでプログラム終了
            // Rawモードなので自前で終了方法を書いてかないと終了する方法がなくなってしまう！
            Event::Key(Key::Ctrl('c')) => {
                return;
            }

            // 方向キーの処理
            Event::Key(Key::Up) => {
                if cursor.row > 0 {
                    cursor.row -= 1;
                    cursor.column = min(buffer[cursor.row].len(), cursor.column);
                }
            }
            Event::Key(Key::Down) => {
                if cursor.row + 1 < buffer.len() {
                    cursor.row += 1;
                    cursor.column = min(cursor.column, buffer[cursor.row].len());
                }
            }
            Event::Key(Key::Left) => {
                if cursor.column > 0 {
                    cursor.column -= 1;
                }
            }
            Event::Key(Key::Right) => {
                cursor.column = min(cursor.column + 1, buffer[cursor.row].len());
            }

            _ => {}
        }

        // カーソルの移動
        write!(
            stdout,
            "{}",
            cursor::Goto(cursor.column as u16 + 1, cursor.row as u16 + 1)
        );

        stdout.flush().unwrap();
    }
}
