mod uci;
mod evaluation;
mod search;
mod transpo;
mod move_ordering;
mod bb_utils;


fn main() {
    uci::uci_loop();
}

