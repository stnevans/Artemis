
use chess::{Color, Board, Piece, BoardStatus, NUM_PIECES, BitBoard};

const NAIVE_PIECE_VAL : [i32; chess::NUM_PIECES] =  [100, 280, 320, 500, 900, i32::MAX];
const ALL_PIECES : [Piece; chess::NUM_PIECES] = [Piece::Pawn, Piece::Knight, Piece::Bishop, Piece::Rook, Piece::Queen, Piece::King];
const ALL_PIECES_NO_KING : [Piece; chess::NUM_PIECES-1] = [Piece::Pawn, Piece::Knight, Piece::Bishop, Piece::Rook, Piece::Queen];


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

// const ALL_COLORS : [Color; chess::NUM_COLORS] = [Color::White, Color::Black];
pub fn eval_is_mate(eval : i32) -> bool {
    eval < i32::MIN + 1200 || eval > -(i32::MIN+1200) 
}

pub fn eval_distance_to_mate(eval : i32) -> i32 {
    if eval < i32::MIN+1200 {
		return (-eval+(i32::MIN+1000))/2;
	}else if eval > -(i32::MIN+1200 ){
		return (-eval-(i32::MIN+1000)+1)/2;
	}
    0
}


pub fn eval_no_ply(board : &Board) -> i32 {
    eval(board, 0)
}

fn centralization_midgame(bit_board : BitBoard, piece : Piece) -> i32{
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

pub fn centralization_eval(board : &Board, total_material : i32) -> i32 {
    let white_bb = board.color_combined(Color::White);
    let black_bb = board.color_combined(Color::Black);

    let mut eval = 0;
    if total_material < 1600 {
        for piece in ALL_PIECES {
            let white_piece_bb = board.pieces(piece) & white_bb;
            let black_piece_bb = board.pieces(piece) & black_bb;
            eval += centralization_midgame(white_piece_bb, piece) - centralization_midgame(black_piece_bb, piece);
        }        
    } else {
        for piece in ALL_PIECES {
            let white_piece_bb = board.pieces(piece) & white_bb;
            let black_piece_bb = board.pieces(piece) & black_bb;
            eval += centralization_midgame(white_piece_bb, piece) - centralization_midgame(black_piece_bb, piece);
        }
    }
    eval
}

pub fn total_material_eval(board : &Board) -> i32 {
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

pub fn eval(board : &Board, ply : u32) -> i32{
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

    eval += centralization_eval(board, total_material);
    

    

    
    match board.side_to_move() {
        Color::Black => eval = eval * -1,
        Color::White => (),
    }
    eval
}