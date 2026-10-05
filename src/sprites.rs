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

#[derive(Clone, Copy, PartialEq)]
pub enum Eyes {
    Open,
    Shut,
    Happy,
}

/// Lapla with these eyes.
pub fn lapla(eyes: Eyes) -> Vec<&'static str> {
    let mut rows = LAPLA.to_vec();
    (rows[15], rows[16]) = match eyes {
        Eyes::Open => (EYES_OPEN, EYES_OPEN),
        Eyes::Shut => (EYES_SHUT, EYES_OPEN),
        Eyes::Happy => (EYES_OPEN, EYES_SHUT),
    };
    rows
}

/// Lapla for the corner of the palette, 14 x 16.
const MINI: [&str; 16] = [
    "...kk....kk...",
    "..kwpk..kpwk..",
    "..kwpk..kpwk..",
    "..kwpk..kpwk..",
    "..kkkkkkkkkk..",
    "..kdGddddddk..",
    "..krrrrrrrrk..",
    ".kkkkkkkkkkkk.",
    "..kwwwwwyyyk..",
    ".kwwkwwwykyk..",
    ".kwwkwwwykyk..",
    ".kppwwPPyyyk..",
    "..kwwwwwwwyk..",
    "...kknrrnkk...",
    "..knnnwwnnnk..",
    "..knnnwwnnnk..",
];
pub const MINI_W: i32 = 14;
pub const MINI_H: i32 = 16;
const MINI_OPEN: &str = ".kwwkwwwykyk..";
const MINI_SHUT: &str = ".kwwwwwwywyk..";

pub fn mini(eyes: Eyes) -> Vec<&'static str> {
    let mut rows = MINI.to_vec();
    (rows[9], rows[10]) = match eyes {
        Eyes::Open => (MINI_OPEN, MINI_OPEN),
        Eyes::Shut => (MINI_SHUT, MINI_OPEN),
        Eyes::Happy => (MINI_OPEN, MINI_SHUT),
    };
    rows
}
