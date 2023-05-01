
use chess::{BitBoard, EMPTY};

pub struct BitBoardUtils {
    pub file_mask : [BitBoard; 8],
    pub rank_mask : [BitBoard; 8],
    pub square_mask : [BitBoard; 64],
}

impl BitBoardUtils {
    fn get_file(&self, loc : usize) -> usize {
        loc % 8
    }

    fn get_rank(&self, loc : usize) -> usize {
        loc / 8
    }

    pub fn new() -> BitBoardUtils {
        let mut ret = BitBoardUtils {
            file_mask :  [EMPTY; 8],
            rank_mask :  [EMPTY; 8],
            square_mask : [EMPTY; 64],
        };
        for i in 0..64 {
            let mask : u64 = 1 << i;
            ret.square_mask[i] = BitBoard(mask);
            ret.file_mask[ret.get_file(i)] |= ret.square_mask[i];
            ret.rank_mask[ret.get_rank(i)] |= ret.square_mask[i];
        }



        ret
        
    }

}