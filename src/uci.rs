use vampirc_uci::{UciMessage};
use std::io::{self, BufRead};
use std::str::FromStr;
use std::time::SystemTime;
use chess::{Board, MoveGen};
use crate::evaluation::Evaluator;
use crate::search::{Search, DUMMY_MOVE};
use crate::transpo::{self, TranspoTable};
use crate::move_ordering::{MoveOrdering, MoveOrderer};

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
                UciMessage::CopyProtection ( _ ) => {
                    perft(&board, 3);
                },


                _ => (),
            }
        }      
    }
}
const MIN_ALPHA : i32 = i32::MIN + 500;
const MAX_BETA : i32 = i32::MAX - 500;


fn perft(board : &Board, depth : i32) {
    println!("perft");
    // create an iterable
    let iterable = MoveGen::new_legal(&board);
    let mut total = 0;
    let start = SystemTime::now();
    for chess_move in iterable {
        let board_copy = board.make_move_new(chess_move);
        let mut temp = Temp::new();
        let num = -temp.alphabeta(&board_copy, depth - 1, MIN_ALPHA, MAX_BETA);
        println!("{chess_move} {}  = {num}", temp.nodes);
        total += temp.nodes;
    }
    let duration = SystemTime::now().duration_since(start).expect("Error");

    println!("-----------------");
    println!("Nodes: {total}");
    println!("Took {}s. nps {}", duration.as_millis() as f64 / 1000.0, (total as f64 / (duration.as_millis() as f64 / 1000.0)));
}

fn perft_internal(board : &Board, depth : i32) -> i32 {
    if depth == 0 {
        Evaluator::new().eval(board,0);
        return 1
    }
    let iterable = MoveGen::new_legal(&board);
    let mut total = 0;
    for chess_move in iterable {
        let board_copy = board.make_move_new(chess_move);
        let num = perft_internal(&board_copy, depth - 1);
        total += num;
    }
    total
}
struct Temp {
    nodes : u32,
    move_orderer : MoveOrderer,
    tt: TranspoTable,
}
impl Temp {
    fn new() -> Temp {
        Temp {
            nodes : 0,
            move_orderer : MoveOrderer::new(),
            tt : TranspoTable::new(),
        }
    }

    fn alphabeta(&mut self, board : &Board, depth : i32, mut alpha : i32, beta : i32) -> i32 {
        // Depth 0, quiesce
        if depth <= 0 {
            self.nodes += 1;
            let eval = self.quiesce(board, -beta, -alpha);
            // let eval = Evaluator::new().eval(board,0);;
            return eval;
        }

        // TODO Gen psuedo moves, check len(pseudo)
        let moves = MoveGen::new_legal(&board);

        // If there were no moves, that means it's draw or mate
        if moves.len() == 0 {
            return Evaluator::new().eval(board,0);
        }

        // Go through each move, internal alphabeta
        for chess_move in moves {


            // test a move
            let new_board: Board = board.make_move_new(chess_move);
            let inner_result = self.alphabeta(&new_board, depth - 1, -beta, -alpha,);
            let score = -inner_result;


            // Score >= beta means refutation was found (i.e we know we worst case eval is -200. this move gives eval of > that)
            if score >= beta {
                return beta;
            }

            // Score > alpha means we have a new best move
            if score > alpha {
                
                alpha = score;
            }
        }
        

        alpha
    }


    fn quiesce(&mut self, board : &Board, mut alpha : i32, beta : i32) -> i32 {

        // Do our initial eval and check cutoffs
        let eval = Evaluator::new().eval(board, 0);
        if eval >= beta {
            return beta
        }

        if eval > alpha {        
            alpha = eval;
        }

        // Do our captures and keep searching
        // TODO BUG NO ENPASSANT IN THIS
        let mut moves = MoveGen::new_legal(&board);
        let targets = board.color_combined(!board.side_to_move());

        moves.set_iterator_mask(*targets);

        let mut move_ordering = MoveOrdering::from_moves(&mut moves);

        for i in 0..move_ordering.len() {
            let capture = move_ordering.get_next_best_move(i, board, 0, &self.tt, &self.move_orderer, DUMMY_MOVE);
            // println!("Considering {capture} on {board}");


            let new_board: Board = board.make_move_new(capture);
            let score = -self.quiesce(&new_board, -beta, -alpha);

            if score >= beta {
                return beta;
            }

            if score > alpha {
                alpha = score;
            }
        }

        alpha
    }
}