// Our transposition table
use std::{mem, collections::btree_map::Entry};
use chess::{ChessMove, Square};
use crate::evaluation;

const DUMMY_MOVE : ChessMove = ChessMove {
    source: Square::A1,
    dest: Square::A1,
    promotion: None
};

const DEFAULT_TABLE_ENTRY : TableEntry = TableEntry {
    hash : 0,
    eval : 0,
    depth : 0,
    flags : EntryFlags::Exact,
    best_move : DUMMY_MOVE
};

const DEFAULT_TT_SIZE : u64 = 1048576 * 10;
#[derive(Clone, Copy)]
pub enum EntryFlags {
    Exact,
    Alpha,
    Beta,
}
#[derive(Clone)]
pub struct TableEntry {
    pub hash : u64,
    pub eval : i32, 
    pub depth : u8,
    pub flags : EntryFlags,
    pub best_move : ChessMove,
}

pub struct TranspoTable {
    entrys : Vec<TableEntry>
}


impl TranspoTable {
    pub fn new() -> TranspoTable {
        let mut tt = TranspoTable { entrys: Vec::new()};
        tt.set_size(DEFAULT_TT_SIZE);
        tt
    }

    fn set_size(&mut self, size : u64) {
        
        let calc_entrys = u64::max((size / mem::size_of::<TableEntry>() as u64) - 1, 0);
        self.entrys = vec![DEFAULT_TABLE_ENTRY; calc_entrys as usize];
    }

    pub fn probe(&self, key : u64) -> &TableEntry {
        return &self.entrys[(key % self.entrys.len() as u64) as usize];
    }

    pub fn save(&mut self, key : u64, eval : i32, flags : EntryFlags, best_move : ChessMove, depth : u8, ply : u8) {
        let len = self.entrys.len();
        let mut entry = &mut self.entrys[(key % len as u64) as usize];
        if depth > entry.depth {
            entry.hash = key;
            if evaluation::eval_is_mate(eval) {
                if eval < 0 {
                    entry.eval = eval - ply as i32;
                } else {
                    entry.eval = eval + ply as i32;
                }
            } else {
                entry.eval = eval;
            }
            entry.depth = depth;
            entry.best_move = best_move;
            entry.flags = flags;
        }
    }
}