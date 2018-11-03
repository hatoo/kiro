extern crate termion;

use std::io::{stdin, stdout, Write};
use termion::clear;
use termion::cursor;
use termion::event::{Event, Key};
use termion::input::TermRead;
use termion::raw::IntoRawMode;

fn main() {
    let stdin = stdin();
    // Rawモードに移行
    // into_raw_modeはIntoRawModeトレイトに定義されている
    // めんどくさいので失敗時は終了(unwrap)
    // stdout変数がDropするときにrawモードから元の状態にもどる
    let mut stdout = stdout().into_raw_mode().unwrap();

    // 画面全体をクリアする
    write!(stdout, "{}", clear::All);
    // カーソルを左上に設定する(1-indexed)
    write!(stdout, "{}", cursor::Goto(1, 1));
    // Hello World!
    write!(stdout, "Hello World!");
    // 最後にフラッシュする
    stdout.flush().unwrap();

    // eventsはTermReadトレイトに定義されている
    for evt in stdin.events() {
        // Ctrl-cでプログラム終了
        // Rawモードなので自前で終了方法を書いてかないと終了する方法がなくなってしまう！
        if evt.unwrap() == Event::Key(Key::Ctrl('c')) {
            return;
        }
    }
}
