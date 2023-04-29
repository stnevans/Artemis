// Our transposition table
use std::{mem, collections::btree_map::Entry};
use chess::{ChessMove, Square};
const DUMMY_MOVE : ChessMove = ChessMove {
    source: Square::A1,
    dest: Square::A1,
    promotion: None
};

#[derive(Clone, Copy)]
enum EntryFlags {
    Exact,
    Alpha,
    Beta,
}
#[derive(Clone)]
pub struct TableEntry {
    pub hash : u64,
    pub eval : i32, 
    pub depth : u8,
    flags : EntryFlags,
    pub best_move : ChessMove,
}

pub struct TranspoTable {
    entrys : Vec<TableEntry>
}

impl TranspoTable {
    pub fn set_size(&mut self, size : u64) {
        let default_entry = TableEntry {
            hash : 0,
            eval : 0,
            depth : 0,
            flags : EntryFlags::Exact,
            best_move : DUMMY_MOVE
        };
        let calc_entrys = u64::max((size / mem::size_of::<TableEntry>() as u64) - 1, 0);
        self.entrys = vec![default_entry; calc_entrys as usize]
    }

    pub fn probe(&self, key : u64) -> &TableEntry {
        return &self.entrys[(key % self.entrys.len() as u64) as usize];
    }
}