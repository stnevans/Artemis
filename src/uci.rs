use vampirc_uci::parse;
use vampirc_uci::{UciMessage, MessageList, UciTimeControl, Serializable};
use std::io::{self, BufRead};
use std::str::FromStr;
use chess::Board;
use chess::ChessMove;
use crate::search::Search;
use crate::transpo;
use std::mem;
pub fn uci_loop () {
    let mut board = Board::default();
    let mut tt = transpo::TranspoTable::new();
    'outer: loop {
        for line in io::stdin().lock().lines() {
            let msg: UciMessage = vampirc_uci::parse_one(&line.unwrap());
            println!("Received message: {}", msg);

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
                    let mut search = Search::new();

                    if search_control.is_some() {
                        let control = search_control.unwrap();
                        if control.depth.is_some() {
                            search.set_cfg_depth(control.depth.unwrap() as u32);
                        }
                    }
                    if time_control.is_some() {
                        let control = time_control.unwrap();

                    }
                    let result = search.get_best_move(&board, &mut tt);

                    println!("bestmove {result}");

                },

                _ => println!("{board}"),
            }
        }      
    }
}