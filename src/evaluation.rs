use chess::{Color, Board, Piece, BoardStatus, NUM_PIECES, BitBoard, File, EMPTY, ALL_FILES};

use crate::bb_utils::BitBoardUtils;

pub const NAIVE_PIECE_VAL : [i32; chess::NUM_PIECES] =  [100, 290, 310, 500, 900, i32::MAX];
const ALL_PIECES : [Piece; chess::NUM_PIECES] = [Piece::Pawn, Piece::Knight, Piece::Bishop, Piece::Rook, Piece::Queen, Piece::King];
const ALL_PIECES_NO_KING : [Piece; chess::NUM_PIECES-1] = [Piece::Pawn, Piece::Knight, Piece::Bishop, Piece::Rook, Piece::Queen];
const DOUBLE_PAWN_PENALTY : i32 = 20;
const ISOLATED_PAWN_PENALTY : i32 = 15;
const PASSED_PAWN_BONUS : i32 = 25;
const TEMPO_VALUE : i32 = 5;
const PAWN_PROTECTOR_BONUS : i32 = 6;
const PAWN_SECOND_PROTECTOR_BONUS : i32 = 4;
const KING_ON_SEMI_OPEN : i32 = 7;



const PAWN_CENTRALIZATION_MIDGAME : [i32; 64] = 
    [0,0,0,0,0,0,0,0,
    0,2,2,2,2,2,2,0,
    0,2,4,4,4,4,2,0,
    0,2,5,12,12,5,2,0,
    0,2,5,12,12,5,2,0,
    0,2,4,4,4,4,2,0,
    0,2,2,2,2,2,2,0,
    0,0,0,0,0,0,0,0];
const KNIGHT_CENTRALIZATION_MIDGAME : [i32; 64] = 
    [0,0,0,0,0,0,0,0,
    0,2,2,2,2,2,2,0,
    0,2,4,4,4,4,2,0,
    0,2,5,12,12,5,2,0,
    0,2,5,12,12,5,2,0,
    0,2,4,4,4,4,2,0,
    0,2,2,2,2,2,2,0,
    0,0,0,0,0,0,0,0];

const SLIDER_CENTRALIZATION_MIDGAME : [i32; 64] = 
    [0,0,0,0,0,0,0,0,
     0,0,0,0,0,0,0,0,
     0,0,4,4,4,4,0,0,
     0,0,4,4,4,4,0,0,
     0,0,4,4,4,4,0,0,
     0,0,4,4,4,4,0,0,
     0,0,0,0,0,0,0,0,
     0,0,0,0,0,0,0,0,
    ];
const KING_CENTRALIZATION_MIDGAME : [i32; 64] = 
    [0,  0,  0,  0,  0,  0,  0,  0,  
     0,  0,  0,  0,  0,  0,  0,  0,  
     0,  0, -8,-10,-10, -8,  0,  0,  
     0,  0, -10,-16,-16,-10, 0,  0,  
     0,  0, -10,-16,-16,-10, 0,  0,  
     0,  0, -8, -10,-10, -8, 0,  0,  
     0,  0,  0,  0,  0,  0,  0,  0,  
     0,  0,  0,  0,  0,  0,  0,  0,  
    ];


const ALL_CENTRALIZATION_MIDGAME : [[i32 ; 64]; NUM_PIECES] =
    [PAWN_CENTRALIZATION_MIDGAME, KNIGHT_CENTRALIZATION_MIDGAME, SLIDER_CENTRALIZATION_MIDGAME, SLIDER_CENTRALIZATION_MIDGAME, SLIDER_CENTRALIZATION_MIDGAME, KING_CENTRALIZATION_MIDGAME];
const SLIDER_CENTRALIZATION_ENDGAME : [i32; 64] = SLIDER_CENTRALIZATION_MIDGAME;
const KING_CENTRALIZATION_ENDGAME : [i32; 64] = 
    [0,  0,  0,  0,  0,  0,  0,  0,  
     0,  0,  0,  0,  0,  0,  0,  0,  
     0,  0,  8, 10, 10,  8,  0,  0,  
     0,  0,  10, 16, 16, 10, 0,  0,  
     0,  0,  10, 16, 16, 10, 0,  0,  
     0,  0,  8,  10, 10,  8, 0,  0,  
     0,  0,  0,  0,  0,  0,  0,  0,  
     0,  0,  0,  0,  0,  0,  0,  0,  
    ];

const WHITE_PAWN_POSITION_ENDGAME : [i32; 64] = 
    [0,0,0,0,0,0,0,0,
    0,0,0,0,0,0,0,0,
    0,0,0,20,20,0,0,0,
    5,5,10,25,25,10,5,5,
    10,10,20,30,30,20,10,10,
    15,15,20,20,20,20,15,25,
    50,50,50,50,50,50,50,50,
    0,0,0,0,0,0,0,0];
const BLACK_PAWN_POSITION_ENDGAME : [i32; 64] =
    [0,0,0,0,0,0,0,0,
    50,50,50,50,50,50,50,50,
    15,15,20,20,20,20,15,25,
    10,10,20,30,30,20,10,10,
    5,5,10,25,25,10,5,5,
    0,0,0,20,20,0,0,0,
    0,0,0,0,0,0,0,0,
    0,0,0,0,0,0,0,0];

const ALL_WHITE_POSITION_ENDGAME : [[i32 ; 64]; NUM_PIECES] =
    [WHITE_PAWN_POSITION_ENDGAME, KNIGHT_CENTRALIZATION_MIDGAME, SLIDER_CENTRALIZATION_ENDGAME, SLIDER_CENTRALIZATION_ENDGAME, SLIDER_CENTRALIZATION_ENDGAME, KING_CENTRALIZATION_ENDGAME];
const ALL_BLACK_POSITION_ENDGAME : [[i32 ; 64]; NUM_PIECES] =
    [BLACK_PAWN_POSITION_ENDGAME, KNIGHT_CENTRALIZATION_MIDGAME, SLIDER_CENTRALIZATION_ENDGAME, SLIDER_CENTRALIZATION_ENDGAME, SLIDER_CENTRALIZATION_ENDGAME, KING_CENTRALIZATION_ENDGAME];

const MIDDLE_FILES : [File; 6] = [File::B, File::C, File::D, File::E, File::F, File::G];

pub fn eval_is_mate(eval : i32) -> bool {
    eval < i32::MIN + 1200 || eval > -(i32::MIN+1200) 
}

pub struct Evaluator {
    bb_utils : BitBoardUtils,

}
impl Evaluator {
    pub fn new() -> Evaluator {
        Evaluator {  
            bb_utils : BitBoardUtils::new(),
        }
    }

    // const ALL_COLORS : [Color; chess::NUM_COLORS] = [Color::White, Color::Black];
    pub fn eval_is_mate(&self, eval : i32) -> bool {
        eval < i32::MIN + 1200 || eval > -(i32::MIN+1200) 
    }

    pub fn eval_distance_to_mate(&self, eval : i32) -> i32 {
        if eval < i32::MIN+1200 {
            return (-eval+(i32::MIN+1000))/2;
        }else if eval > -(i32::MIN+1200 ){
            return (-eval-(i32::MIN+1000)+1)/2;
        }
        0
    }

    fn centralization_midgame(&self, bit_board : BitBoard, piece : Piece, _color : Color) -> i32 {
        let mut eval = 0;
        let mut bb = bit_board;
        while bb.0 != 0 {
            let loc = bb.0.trailing_zeros();
            eval += ALL_CENTRALIZATION_MIDGAME[piece.to_index()][loc as usize];
            let mut pos : u64 = 1;
            pos = pos << loc;
            bb ^= BitBoard(pos);
        }
        eval
    }

    fn position_endgame(&self, bit_board : BitBoard, piece : Piece, color : Color) -> i32 {
        let mut eval = 0;
        let mut bb = bit_board;
        let endgame_arr;
        match color {
            Color::White => endgame_arr = ALL_WHITE_POSITION_ENDGAME,
            Color::Black => endgame_arr = ALL_BLACK_POSITION_ENDGAME,
        }

        while bb.0 != 0 {
            let loc = bb.0.trailing_zeros();
            eval += endgame_arr[piece.to_index()][loc as usize];
            let mut pos : u64 = 1;
            pos = pos << loc;
            bb ^= BitBoard(pos);
        }
        eval
    }

    pub fn centralization_eval(&self, board : &Board, total_material : i32) -> i32 {
        let white_bb = board.color_combined(Color::White);
        let black_bb = board.color_combined(Color::Black);

        let mut eval = 0;
        if total_material > 1600 {
            for piece in ALL_PIECES {
                let white_piece_bb = board.pieces(piece) & white_bb;
                let black_piece_bb = board.pieces(piece) & black_bb;
                eval += self.centralization_midgame(white_piece_bb, piece, Color::White) - self.centralization_midgame(black_piece_bb, piece, Color::Black);
            }        
        } else {
            for piece in ALL_PIECES {
                let white_piece_bb = board.pieces(piece) & white_bb;
                let black_piece_bb = board.pieces(piece) & black_bb;
                eval += self.position_endgame(white_piece_bb, piece, Color::White) - self.position_endgame(black_piece_bb, piece, Color::Black);
            }
        }
        eval
    }

    pub fn total_material_eval(&self, board : &Board) -> i32 {
        let white_bb = board.color_combined(Color::White);
        let black_bb = board.color_combined(Color::Black);

        let mut material = 0;
        for piece in ALL_PIECES_NO_KING {
            let white_piece_bb = board.pieces(piece) & white_bb;
            let black_piece_bb = board.pieces(piece) & black_bb;
            material += NAIVE_PIECE_VAL[piece.to_index()] * (white_piece_bb.popcnt() as i32 + black_piece_bb.popcnt() as i32);
        }
        material
    }

    fn get_doubled_pawn_eval(&self, board : &Board) -> i32 {
        let white_pawns = board.color_combined(Color::White) & board.pieces(Piece::Pawn);
        let black_pawns = board.color_combined(Color::Black) & board.pieces(Piece::Pawn);
        let mut eval = 0;

        for file in ALL_FILES {
            let white_pawns_in_file = white_pawns & self.bb_utils.file_mask[file.to_index()];
            let black_pawns_in_file: BitBoard = black_pawns & self.bb_utils.file_mask[file.to_index()];
            
            let white_num_pawns_in_file = white_pawns_in_file.popcnt() as i32;
            let black_num_pawns_in_file = black_pawns_in_file.popcnt() as i32;
            if white_num_pawns_in_file != 0 {
                eval -= DOUBLE_PAWN_PENALTY * (white_num_pawns_in_file - 1);
            }
            if black_num_pawns_in_file != 0 {
                eval += DOUBLE_PAWN_PENALTY * (black_num_pawns_in_file - 1);
            }
        }
        eval
    }

    fn get_isolated_pawn_eval(&self, board : &Board) -> i32 {
        let mut eval = 0;
        let white_pawns = board.color_combined(Color::White) & board.pieces(Piece::Pawn);
        let black_pawns = board.color_combined(Color::Black) & board.pieces(Piece::Pawn);


        for file in MIDDLE_FILES {
            let white_pawns_in_file = white_pawns & self.bb_utils.file_mask[file.to_index()];
            let black_pawns_in_file: BitBoard = black_pawns & self.bb_utils.file_mask[file.to_index()];
            
            
            let white_num_pawns_in_file = white_pawns_in_file.popcnt() as i32;
            let black_num_pawns_in_file = black_pawns_in_file.popcnt() as i32;
            if white_num_pawns_in_file != 0 && (white_pawns & self.bb_utils.file_mask[file.left().to_index()] != EMPTY) 
                && (white_pawns & self.bb_utils.file_mask[file.right().to_index()] != EMPTY){
                eval -= ISOLATED_PAWN_PENALTY * white_num_pawns_in_file;
            }
            if black_num_pawns_in_file != 0 && (black_pawns & self.bb_utils.file_mask[file.left().to_index()] != EMPTY) 
                && (black_pawns & self.bb_utils.file_mask[file.right().to_index()] != EMPTY){
                eval += DOUBLE_PAWN_PENALTY * black_num_pawns_in_file;
            }
        }

        // TODO a pawn and h pawn can be isolated too

        eval
    }

    fn get_passed_pawn_eval(&self, board : &Board) -> i32 {
        let mut eval = 0;
        let white_pawns = board.color_combined(Color::White) & board.pieces(Piece::Pawn);
        let black_pawns = board.color_combined(Color::Black) & board.pieces(Piece::Pawn);


        for file in MIDDLE_FILES {
            let white_pawns_in_file = white_pawns & self.bb_utils.file_mask[file.to_index()];
            let black_pawns_in_file: BitBoard = black_pawns & self.bb_utils.file_mask[file.to_index()];
            
            
            let white_num_pawns_in_file = white_pawns_in_file.popcnt() as i32;
            let black_num_pawns_in_file = black_pawns_in_file.popcnt() as i32;
            if white_num_pawns_in_file != 0 && (black_pawns & self.bb_utils.file_mask[file.left().to_index()] != EMPTY) 
                && (black_pawns & self.bb_utils.file_mask[file.right().to_index()] != EMPTY) && black_num_pawns_in_file == 0{
                eval += PASSED_PAWN_BONUS;
            }
            if black_num_pawns_in_file != 0 && (white_pawns & self.bb_utils.file_mask[file.left().to_index()] != EMPTY) 
                && (white_pawns & self.bb_utils.file_mask[file.right().to_index()] != EMPTY) && white_num_pawns_in_file == 0{
                eval -= PASSED_PAWN_BONUS;
            }
        }

        // TODO a pawn and h pawn can be passed too

        eval
    }


    fn get_king_safety(&self, board : &Board) -> i32 {
        let mut eval = 0;
        let white_bb = board.color_combined(Color::White);
        let black_bb = board.color_combined(Color::Black);
        let white_kings = white_bb & board.pieces(Piece::King);
        let black_kings = black_bb & board.pieces(Piece::King);
        let white_pawns = white_bb & board.pieces(Piece::Pawn);
        let black_pawns = black_bb & board.pieces(Piece::Pawn);

        let white_protectors = white_pawns & (BitBoard(white_kings.0 << 7) | BitBoard(white_kings.0 << 8) | BitBoard(white_kings.0 << 9));
        let black_protectors = black_pawns & (BitBoard(black_kings.0 >> 7) | BitBoard(black_kings.0 >> 8) | BitBoard(black_kings.0 >> 9));
        let white_second_protectors = white_pawns & (BitBoard(white_kings.0 << 15) | BitBoard(white_kings.0 << 16) | BitBoard(white_kings.0 << 9));
        let black_second_protectors = black_pawns & (BitBoard(black_kings.0 >> 15) | BitBoard(black_kings.0 >> 16) | BitBoard(black_kings.0 >> 9));
        eval += PAWN_PROTECTOR_BONUS * white_protectors.popcnt() as i32;
        eval -= PAWN_PROTECTOR_BONUS * black_protectors.popcnt() as i32;
        eval += PAWN_SECOND_PROTECTOR_BONUS * white_second_protectors.popcnt() as i32;
        eval -= PAWN_SECOND_PROTECTOR_BONUS * black_second_protectors.popcnt() as i32;

        for file in ALL_FILES {
            if (white_kings & self.bb_utils.file_mask(file)) != EMPTY {
                if (white_pawns & self.bb_utils.file_mask(file)) != EMPTY {
                    eval += KING_ON_SEMI_OPEN;
                }
            }
            if (black_kings & self.bb_utils.file_mask(file)) != EMPTY {
                if (black_pawns & self.bb_utils.file_mask(file)) != EMPTY {
                    eval -= KING_ON_SEMI_OPEN;
                }
            }
        }
        eval
    }

    pub fn eval(&self, board : &Board, ply : u32) -> i32 {
        let mut eval : i32 = 0;

        // TODO fixup
        match board.status() {
            BoardStatus::Checkmate => return i32::MIN + 1000 + ply as i32,
            BoardStatus::Stalemate => return 0, 
            BoardStatus::Ongoing => (),
        }


        let white_bb = board.color_combined(Color::White);
        let black_bb = board.color_combined(Color::Black);

        // let pawns = [white_bb & board.pieces(Piece::Pawn), black_bb & board.pieces(Piece::Pawn)];
        // let rooks = [white_bb & board.pieces(Piece::Rook), black_bb & board.pieces(Piece::Rook)];
        
        let mut total_material = 0;
        // Do material eval
        for piece in ALL_PIECES_NO_KING {
            let white_piece_bb = board.pieces(piece) & white_bb;
            let black_piece_bb = board.pieces(piece) & black_bb;
            eval += NAIVE_PIECE_VAL[piece.to_index()] * (white_piece_bb.popcnt() as i32 - black_piece_bb.popcnt() as i32);
            total_material += NAIVE_PIECE_VAL[piece.to_index()] * (white_piece_bb.popcnt() as i32 + black_piece_bb.popcnt() as i32);
        }

        eval += self.centralization_eval(board, total_material);
        eval += self.get_doubled_pawn_eval(board);
        eval += self.get_isolated_pawn_eval(board);
        eval += self.get_passed_pawn_eval(board);

        if total_material > 1600 {
            eval += self.get_king_safety(board);
        }

        

        eval += TEMPO_VALUE;

        match board.side_to_move() {
            Color::Black => eval = eval * -1,
            Color::White => (),
        }
        eval
    }
}