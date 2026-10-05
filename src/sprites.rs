//! Pixel sprites, one `pixel::pal` char per pixel.

/// Lapla the banker: top hat, monocle, bow tie, white gloves. 22 x 31.
const LAPLA: [&str; 31] = [
    ".......kk....kk.......",
    "......kwwk..kwwk......",
    "......kwpk..kpwk......",
    "......kwpk..kpwk......",
    "......kwpk..kpwk......",
    "......kwpk..kpwk......",
    ".....kkkkkkkkkkkk.....",
    ".....kdGddddddddk.....",
    ".....kdGddddddddk.....",
    ".....kdGddddddddk.....",
    ".....krrrrrrrrrrk.....",
    ".....kddddddddddk.....",
    "...kkkkkkkkkkkkkkkk...",
    "....kwwwwwwwwwwwwk....",
    "...kwwwwwwwwyyyywwk...",
    "...kwwkkwwwwykkywwk...",
    "...kwwkkwwwwykkywwk...",
    "...kppwwwwwwyyyyppk...",
    "...kwwwwwwPPwwwwywk...",
    "....kwwwwkwwkwwwyk....",
    ".....kwwwwkkwwwwyk....",
    ".....knnrrbbrrnnk.....",
    "....knnnnwwwwnnnnk....",
    "...knnnnnwywwnnnnnk...",
    "..kknnnnnwwwwnnnnnkk..",
    "..kwknnnnwywwnnnnkwk..",
    "..kwwknnnnnnnnnnkwwk..",
    "...kk.knnnnnnnnk.kk...",
    "......knnnkknnnk......",
    ".....kwwwk..kwwwk.....",
    ".....kkkkk..kkkkk.....",
];
pub const LAPLA_W: i32 = 22;
pub const LAPLA_H: i32 = 31;

const EYES_OPEN: &str = "...kwwkkwwwwykkywwk...";
const EYES_SHUT: &str = "...kwwwwwwwwywwywwk...";
const STEP_A: [&str; 2] = [".....kwwwkkkwwwk......", ".....kkkkk.kkkkk......"];
const STEP_B: [&str; 2] = ["......kwwwkkkwwwk.....", "......kkkkk.kkkkk....."];

#[derive(Clone, Copy, PartialEq)]
pub enum Eyes {
    Open,
    Shut,
    Happy,
}

/// Lapla with these eyes; `step` picks a walking frame.
pub fn lapla(eyes: Eyes, step: Option<bool>) -> Vec<&'static str> {
    let mut rows = LAPLA.to_vec();
    (rows[15], rows[16]) = match eyes {
        Eyes::Open => (EYES_OPEN, EYES_OPEN),
        Eyes::Shut => (EYES_SHUT, EYES_OPEN),
        Eyes::Happy => (EYES_OPEN, EYES_SHUT),
    };
    if let Some(left) = step {
        let [a, b] = if left { STEP_A } else { STEP_B };
        (rows[29], rows[30]) = (a, b);
    }
    rows
}

/// A spinning gold coin, three frames from face to edge.
pub const COIN: [[&str; 8]; 3] = [
    ["..kkkk..", ".kyyyyk.", "kyYyyyok", "kyYyyyok", "kyYyyyok", "kyyyyyok", ".koooook", "..kkkk.."],
    ["...kk...", "..kyyk..", ".kyYyok.", ".kyYyok.", ".kyYyok.", ".kyyyok.", "..kook..", "...kk..."],
    ["...kk...", "...kyk..", "...kyk..", "...kyk..", "...kyk..", "...kyk..", "...kok..", "...kk..."],
];

/// A pile of banknotes for Lapla's nap.
pub const NOTES: [&str; 10] = [
    "..kkkkkkkkkkkk..",
    "..kEEEEEEEEEEk..",
    "kkkkkkkkkkkkkkk.",
    "kEEEEEEEEEEEEEk.",
    "kEeEEEyyEEEeEEk.",
    "kEEEEEyyEEEEEEk.",
    "kkkkkkkkkkkkkkkk",
    ".kEEEEEEEEEEEEEk",
    ".kEeEEEyyEEEeEEk",
    ".kkkkkkkkkkkkkkk",
];

/// The slot machine symbols. The last one, a vault box, is the jackpot.
pub const SYMBOLS: [[&str; 8]; 4] = [
    ["......ek", ".....ek.", "....e.e.", "...e...e", ".kkk.kkk", "krrrkrrr", "krwrkrwr", ".kkk.kkk"],
    ["kkkkkkkk", "krrrrrrk", "kkkkkrrk", "...krrk.", "..krrk..", "..krrk..", ".krrk...", ".kkkk..."],
    ["...kk...", "..kyyk..", ".kyYyyk.", ".kyYyyk.", ".kyyyyk.", "kyyyyyyk", "kkkkkkkk", "...kk..."],
    ["kkkkkkkk", "kwwwwwgk", "kwggggSk", "kwgkkgSk", "kwggkgSk", "kwggggSk", "kgSSSSSk", "kkkkkkkk"],
];
pub const JACKPOT: usize = 3;
