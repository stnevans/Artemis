
use chess::MoveGen;
use chess::EMPTY;
use chess::{Square, Board, ChessMove, Piece};


mod uci;
mod evaluation;
mod search;

use crate::search::get_best_move;

fn main() {
    println!("Hello, world!");
    tester();
    // demo_eval();
    // demo_search();
    uci::uci_loop();
}



fn tester() {
    // create a board with the initial position
    let board = Board::default();

    // create an iterable
    let mut iterable = MoveGen::new_legal(&board);

    // make sure .len() works.
    assert_eq!(iterable.len(), 20); // the .len() function does *not* consume the iterator

    // lets iterate over targets.
    let targets = board.color_combined(!board.side_to_move());
    iterable.set_iterator_mask(*targets);

    // count the number of targets
    let mut count = 0;
    for _ in &mut iterable {
        count += 1;
        // This move captures one of my opponents pieces (with the exception of en passant)
    }

    assert_eq!(count, 0);

    // now, iterate over the rest of the moves
    iterable.set_iterator_mask(!EMPTY);
    for _ in &mut iterable {
        count += 1;
        // This move does not capture anything
    }

    // make sure it works
    assert_eq!(count, 20);
    get_best_move(&board, &search::get_default_cfg(2));
}





#[allow(deprecated)]
fn board_from_fen(fen : &str) -> Board {
    Board::from_fen(fen.to_string()).expect("Bad fen")
}

fn demo_eval () {
    let board = board_from_fen("1k6/6p1/8/8/8/3P4/8/1K6 w - - 0 1");
    println!("Eval(P vs P)={}", crate::evaluation::eval_no_ply(&board));
    let board = board_from_fen("1k6/6p1/8/8/8/3PP3/8/1K6 w - - 0 1");
    println!("Eval(PP vs P)={}", crate::evaluation::eval_no_ply(&board));
    let board = board_from_fen("1k6/5n2/8/8/8/3P4/8/1K6 w - - 0 1");
    println!("Eval(P vs N)={}", crate::evaluation::eval_no_ply(&board));
    let board = board_from_fen("1k6/1b3n2/5K2/8/8/3P4/2R5/8 w - - 0 1");
    println!("Eval(P R vs N B)={}", crate::evaluation::eval_no_ply(&board));
}

fn  demo_search () {
    let board: Board = board_from_fen("8/4P3/8/8/8/8/8/1k1K4 w - - 0 1"); // Next move queen
    
    let best_move = get_best_move(&board, &search::get_default_cfg(1));
    println!("Best Move should be queen: {best_move}");
    assert_eq!(best_move, ChessMove{source: Square::E7, dest: Square::E8, promotion: Some(Piece::Queen)});


    let board = board_from_fen("8/8/4P3/8/8/8/8/1k1K4 w - - 0 1");
    for depth in 1..5 {
        let best_move = get_best_move(&board, &search::get_default_cfg(depth));
        println!("Best Move should be queen: {best_move}");
    }

}