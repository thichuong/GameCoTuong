use std::sync::OnceLock;

// Max dimension is 10 (rows 0..9).
// Occupancy mask is up to 10 bits. 1 << 10 = 1024.
const MAX_DIM: usize = 10;
const OCC_SIZE: usize = 1024;

pub struct AttackTables {
    pub rook_table: [[u16; OCC_SIZE]; MAX_DIM],
    pub cannon_table: [[u16; OCC_SIZE]; MAX_DIM],
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

        Self {
            rook_table,
            cannon_table,
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
