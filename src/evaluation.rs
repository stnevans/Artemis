
use chess::{Color, Board, Piece, BoardStatus};

const NAIVE_PIECE_VAL : [i32; chess::NUM_PIECES] =  [100, 280, 320, 500, 900, i32::MAX];
const ALL_PIECES : [Piece; chess::NUM_PIECES] = [Piece::Pawn, Piece::Knight, Piece::Bishop, Piece::Rook, Piece::Queen, Piece::King];
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
    

    // Do material eval
    for piece in ALL_PIECES {
        let white_piece_bb = board.pieces(piece) & white_bb;
        let black_piece_bb = board.pieces(piece) & black_bb;
        eval += NAIVE_PIECE_VAL[piece.to_index()] * (white_piece_bb.popcnt() as i32 - black_piece_bb.popcnt() as i32);
    }

    

    
    match board.side_to_move() {
        Color::Black => eval = eval * -1,
        Color::White => (),
    }
    eval
}