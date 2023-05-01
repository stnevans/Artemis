use vampirc_uci::{UciMessage};
use std::io::{self, BufRead};
use std::str::FromStr;
use chess::Board;
use crate::search::Search;
use crate::transpo;

const ARTEMIS_VERSION : &str = "1.0";

pub fn uci_loop () {
    let mut board = Board::default();
    let mut tt = transpo::TranspoTable::new();
    println!("Artemis {ARTEMIS_VERSION}");
    'outer: loop {
        for line in io::stdin().lock().lines() {
            let msg: UciMessage = vampirc_uci::parse_one(&line.unwrap());

            match msg {
                UciMessage::UciNewGame => {
                    board = Board::default();
                },
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
                        search.set_time_controls(control);
                    }
                    let result = search.get_best_move(&board, &mut tt);

                    println!("bestmove {result}");

                },
                UciMessage::IsReady => {
                    println!("readyok");
                },
                UciMessage::Uci => {
                    println!("id name Artemis Release {ARTEMIS_VERSION}");
                    println!("id author Stuart Nevans Locke");
                    // println!("option name Hash type spin default 32 min 1 max 1048576");
                    println!("uciok");
                },
                UciMessage::UciNewGame => {
                    board = Board::default();
                },

                _ => (),
            }
        }      
    }
}