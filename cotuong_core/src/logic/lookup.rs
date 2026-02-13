use std::sync::OnceLock;

// Max dimension is 10 (rows 0..9).
// Occupancy mask is up to 10 bits. 1 << 10 = 1024.
const MAX_DIM: usize = 10;
const OCC_SIZE: usize = 1024;

pub struct AttackTables {
    pub rook_table: [[u16; OCC_SIZE]; MAX_DIM],
    pub cannon_table: [[u16; OCC_SIZE]; MAX_DIM],
    // Lookup tables for non-sliding pieces
    // Horse: (target_sq, leg_sq)
    pub horse_moves: [Vec<(usize, usize)>; 90],
    // Elephant: (target_sq, eye_sq)
    pub elephant_moves: [Vec<(usize, usize)>; 90],
    // Advisor: target_sq
    pub advisor_moves: [Vec<usize>; 90],
    // General: target_sq
    pub general_moves: [Vec<usize>; 90],
    // Soldier: [color_idx][sq] -> target_sq
    pub soldier_moves: [[Vec<usize>; 90]; 2],
}

impl AttackTables {
    fn new() -> Self {
        let mut rook_table = [[0; OCC_SIZE]; MAX_DIM];
        let mut cannon_table = [[0; OCC_SIZE]; MAX_DIM];

        for idx in 0..MAX_DIM {
            for occ in 0..OCC_SIZE {
                rook_table[idx][occ] = compute_rook_attacks(idx, occ, MAX_DIM);
                cannon_table[idx][occ] = compute_cannon_attacks(idx, occ, MAX_DIM);
            }
        }

        let mut horse_moves: [Vec<(usize, usize)>; 90] = core::array::from_fn(|_| Vec::new());
        let mut elephant_moves: [Vec<(usize, usize)>; 90] = core::array::from_fn(|_| Vec::new());
        let mut advisor_moves: [Vec<usize>; 90] = core::array::from_fn(|_| Vec::new());
        let mut general_moves: [Vec<usize>; 90] = core::array::from_fn(|_| Vec::new());
        let mut soldier_moves: [[Vec<usize>; 90]; 2] = [
            core::array::from_fn(|_| Vec::new()),
            core::array::from_fn(|_| Vec::new()),
        ];

        for i in 0..90 {
            horse_moves[i] = compute_horse_moves(i);
            elephant_moves[i] = compute_elephant_moves(i);
            advisor_moves[i] = compute_advisor_moves(i);
            general_moves[i] = compute_general_moves(i);
            soldier_moves[0][i] = compute_soldier_moves(i, 0); // Red
            soldier_moves[1][i] = compute_soldier_moves(i, 1); // Black
        }

        Self {
            rook_table,
            cannon_table,
            horse_moves,
            elephant_moves,
            advisor_moves,
            general_moves,
            soldier_moves,
        }
    }

    pub fn get() -> &'static Self {
        static INSTANCE: OnceLock<AttackTables> = OnceLock::new();
        INSTANCE.get_or_init(AttackTables::new)
    }

    pub fn get_rook_attacks(&self, idx: usize, occ: u16, len: usize) -> u16 {
        // Mask occupancy to length
        let mask = (1 << len) - 1;
        let effective_occ = (occ & mask) as usize;
        self.rook_table[idx][effective_occ] & mask
    }

    pub fn get_cannon_attacks(&self, idx: usize, occ: u16, len: usize) -> u16 {
        let mask = (1 << len) - 1;
        let effective_occ = (occ & mask) as usize;
        self.cannon_table[idx][effective_occ] & mask
    }
}

fn compute_rook_attacks(idx: usize, occ: usize, len: usize) -> u16 {
    let mut attacks = 0;
    // Right
    for i in (idx + 1)..len {
        attacks |= 1 << i;
        if (occ & (1 << i)) != 0 {
            break;
        }
    }
    // Left
    for i in (0..idx).rev() {
        attacks |= 1 << i;
        if (occ & (1 << i)) != 0 {
            break;
        }
    }
    attacks
}

fn compute_cannon_attacks(idx: usize, occ: usize, len: usize) -> u16 {
    let mut attacks = 0;
    // Right
    let mut jumped = false;
    for i in (idx + 1)..len {
        if (occ & (1 << i)) != 0 {
            if !jumped {
                jumped = true;
            } else {
                attacks |= 1 << i; // Capture
                break;
            }
        } else if !jumped {
            attacks |= 1 << i; // Move
        }
    }
    // Left
    jumped = false;
    for i in (0..idx).rev() {
        if (occ & (1 << i)) != 0 {
            if !jumped {
                jumped = true;
            } else {
                attacks |= 1 << i; // Capture
                break;
            }
        } else if !jumped {
            attacks |= 1 << i; // Move
        }
    }
    attacks
}

// Helpers for precomputing moves
fn to_coord(sq: usize) -> (isize, isize) {
    ((sq / 9) as isize, (sq % 9) as isize)
}

fn to_sq(r: isize, c: isize) -> Option<usize> {
    if (0..10).contains(&r) && (0..9).contains(&c) {
        Some((r * 9 + c) as usize)
    } else {
        None
    }
}

fn compute_horse_moves(sq: usize) -> Vec<(usize, usize)> {
    let (r, c) = to_coord(sq);
    let offsets = [
        (-2, -1, -1, 0),
        (-2, 1, -1, 0),
        (2, -1, 1, 0),
        (2, 1, 1, 0),
        (-1, -2, 0, -1),
        (-1, 2, 0, 1),
        (1, -2, 0, -1),
        (1, 2, 0, 1),
    ];
    let mut moves = Vec::new();
    for (dr, dc, lr, lc) in offsets {
        if let Some(target) = to_sq(r + dr, c + dc) {
            if let Some(leg) = to_sq(r + lr, c + lc) {
                moves.push((target, leg));
            }
        }
    }
    moves
}

fn compute_elephant_moves(sq: usize) -> Vec<(usize, usize)> {
    let (r, c) = to_coord(sq);
    let offsets = [
        (-2, -2, -1, -1),
        (-2, 2, -1, 1),
        (2, -2, 1, -1),
        (2, 2, 1, 1),
    ];
    let mut moves = Vec::new();
    for (dr, dc, er, ec) in offsets {
        if let Some(target) = to_sq(r + dr, c + dc) {
            // River check handled safely: just check if target row is on correct side?
            // Red (0-4), Black (5-9).
            let target_r = r + dr;
            let valid = if r <= 4 { target_r <= 4 } else { target_r >= 5 };

            if valid {
                if let Some(eye) = to_sq(r + er, c + ec) {
                    moves.push((target, eye));
                }
            }
        }
    }
    moves
}

fn compute_advisor_moves(sq: usize) -> Vec<usize> {
    let (r, c) = to_coord(sq);
    let offsets = [(-1, -1), (-1, 1), (1, -1), (1, 1)];
    let mut moves = Vec::new();
    for (dr, dc) in offsets {
        if let Some(target) = to_sq(r + dr, c + dc) {
            let tr = r + dr;
            let tc = c + dc;
            let in_palace = (3..=5).contains(&tc) && ((0..=2).contains(&tr) || (7..=9).contains(&tr));
            if in_palace {
                moves.push(target);
            }
        }
    }
    moves
}

fn compute_general_moves(sq: usize) -> Vec<usize> {
    let (r, c) = to_coord(sq);
    let offsets = [(0, 1), (0, -1), (1, 0), (-1, 0)];
    let mut moves = Vec::new();
    for (dr, dc) in offsets {
        if let Some(target) = to_sq(r + dr, c + dc) {
            let tr = r + dr;
            let tc = c + dc;
            let in_palace = (3..=5).contains(&tc) && ((0..=2).contains(&tr) || (7..=9).contains(&tr));
            if in_palace {
                moves.push(target);
            }
        }
    }
    moves
}

fn compute_soldier_moves(sq: usize, color_idx: usize) -> Vec<usize> {
    let (r, c) = to_coord(sq);
    let forward = if color_idx == 0 { 1 } else { -1 };
    let mut moves = Vec::new();

    // Forward
    if let Some(target) = to_sq(r + forward, c) {
        moves.push(target);
    }

    // Horizontal (if crossed river)
    let crossed = if color_idx == 0 { r > 4 } else { r < 5 };
    if crossed {
        if let Some(target) = to_sq(r, c - 1) {
            moves.push(target);
        }
        if let Some(target) = to_sq(r, c + 1) {
            moves.push(target);
        }
    }
    moves
}
