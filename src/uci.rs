use vampirc_uci::parse;
use vampirc_uci::{UciMessage, MessageList, UciTimeControl, Serializable};
use std::io::{self, BufRead};
use std::str::FromStr;
use chess::Board;
use crate::search;
use crate::transpo;

pub fn uci_loop () {
    let mut board = Board::default();

    'outer: loop {
        for line in io::stdin().lock().lines() {
            let msg: UciMessage = vampirc_uci::parse_one(&line.unwrap());
            println!("Received message: {}", msg);

            let mut tt = transpo::TranspoTable::new();
            match msg {
                UciMessage::Quit => break 'outer,
                UciMessage::Position { startpos, fen, moves } => {
                    if startpos {
                        board = Board::default();
                    }
                    if fen.is_some() {
                        board = Board::from_str(fen.unwrap().as_str()).unwrap(); 
                    }

                    for chess_move in moves {
                        board = board.make_move_new(chess_move);
                    }

                },
                UciMessage::UciNewGame => {
                    board = Board::default();
                },
                UciMessage::Go { time_control, search_control } => {
                    if search_control.is_some() {
                        let control = search_control.unwrap();
                        if control.depth.is_some() {
                            let cfg = search::get_default_cfg(control.depth.unwrap() as u32);
                            let result = search::get_best_move(&board, &cfg, &mut tt);
                            println!("bestmove {result}");
                        }
                    }
                },

                _ => (),
            }
        }      
    }
}