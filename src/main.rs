mod uci;
mod evaluation;
mod search;
mod transpo;
mod move_ordering;
mod bb_utils;


fn main() {
    println!("Hello, world!");
    // demo_eval();
    // demo_search();
    // test_board(&board_from_fen("rnbqkbnr/pppp1ppp/8/4p3/5PP1/8/PPPPP2P/RNBQKBNR b KQkq - 0 1"));
    uci::uci_loop();
}

