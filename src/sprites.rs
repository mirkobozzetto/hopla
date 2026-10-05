//! Pixel sprites, one `pixel::pal` char per pixel.

#[derive(Clone, Copy, PartialEq)]
pub enum Eyes {
    Open,
    Shut,
    Happy,
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

/// "hopla" in a 5 x 8 pixel font, one array per letter.
pub const LOGO: [[&str; 8]; 5] = [
    ["#....", "#....", "#.##.", "##..#", "#...#", "#...#", "#...#", "....."],
    [".....", ".....", ".###.", "#...#", "#...#", "#...#", ".###.", "....."],
    [".....", ".....", "####.", "#...#", "#...#", "####.", "#....", "#...."],
    ["##...", ".#...", ".#...", ".#...", ".#...", ".#...", ".##..", "....."],
    [".....", ".....", "####.", "....#", "#####", "#...#", "#####", "....."],
];
