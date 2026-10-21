/// Simple strokes in a cell six tenths wide and one unit tall.
pub(super) fn glyph(character: char) -> &'static [&'static [(f32, f32)]] {
    match character {
        'A' => &[
            &[(0.0, 0.0), (0.3, 1.0), (0.6, 0.0)],
            &[(0.12, 0.4), (0.48, 0.4)],
        ],
        'B' => &[
            &[
                (0.0, 0.0),
                (0.0, 1.0),
                (0.4, 1.0),
                (0.6, 0.85),
                (0.6, 0.65),
                (0.4, 0.5),
                (0.0, 0.5),
            ],
            &[(0.4, 0.5), (0.6, 0.35), (0.6, 0.15), (0.4, 0.0), (0.0, 0.0)],
        ],
        'C' => &[&[
            (0.6, 0.85),
            (0.45, 1.0),
            (0.15, 1.0),
            (0.0, 0.8),
            (0.0, 0.2),
            (0.15, 0.0),
            (0.45, 0.0),
            (0.6, 0.15),
        ]],
        'D' => &[&[
            (0.0, 0.0),
            (0.0, 1.0),
            (0.35, 1.0),
            (0.6, 0.75),
            (0.6, 0.25),
            (0.35, 0.0),
            (0.0, 0.0),
        ]],
        'E' => &[
            &[(0.6, 1.0), (0.0, 1.0), (0.0, 0.0), (0.6, 0.0)],
            &[(0.0, 0.5), (0.45, 0.5)],
        ],
        'F' => &[
            &[(0.0, 0.0), (0.0, 1.0), (0.6, 1.0)],
            &[(0.0, 0.5), (0.45, 0.5)],
        ],
        'G' => &[&[
            (0.6, 0.85),
            (0.45, 1.0),
            (0.15, 1.0),
            (0.0, 0.8),
            (0.0, 0.2),
            (0.15, 0.0),
            (0.6, 0.0),
            (0.6, 0.5),
            (0.35, 0.5),
        ]],
        'H' => &[
            &[(0.0, 0.0), (0.0, 1.0)],
            &[(0.6, 0.0), (0.6, 1.0)],
            &[(0.0, 0.5), (0.6, 0.5)],
        ],
        'I' => &[
            &[(0.0, 1.0), (0.6, 1.0)],
            &[(0.3, 1.0), (0.3, 0.0)],
            &[(0.0, 0.0), (0.6, 0.0)],
        ],
        'J' => &[&[
            (0.0, 1.0),
            (0.6, 1.0),
            (0.6, 0.2),
            (0.45, 0.0),
            (0.15, 0.0),
            (0.0, 0.2),
        ]],
        'K' => &[
            &[(0.0, 0.0), (0.0, 1.0)],
            &[(0.6, 1.0), (0.0, 0.45), (0.6, 0.0)],
        ],
        'L' => &[&[(0.0, 1.0), (0.0, 0.0), (0.6, 0.0)]],
        'M' => &[&[(0.0, 0.0), (0.0, 1.0), (0.3, 0.5), (0.6, 1.0), (0.6, 0.0)]],
        'N' => &[&[(0.0, 0.0), (0.0, 1.0), (0.6, 0.0), (0.6, 1.0)]],
        'O' | '0' => &[&[
            (0.15, 0.0),
            (0.0, 0.2),
            (0.0, 0.8),
            (0.15, 1.0),
            (0.45, 1.0),
            (0.6, 0.8),
            (0.6, 0.2),
            (0.45, 0.0),
            (0.15, 0.0),
        ]],
        'P' => &[&[
            (0.0, 0.0),
            (0.0, 1.0),
            (0.4, 1.0),
            (0.6, 0.8),
            (0.6, 0.65),
            (0.4, 0.5),
            (0.0, 0.5),
        ]],
        'Q' => &[
            &[
                (0.15, 0.0),
                (0.0, 0.2),
                (0.0, 0.8),
                (0.15, 1.0),
                (0.45, 1.0),
                (0.6, 0.8),
                (0.6, 0.2),
                (0.45, 0.0),
                (0.15, 0.0),
            ],
            &[(0.35, 0.25), (0.6, 0.0)],
        ],
        'R' => &[
            &[
                (0.0, 0.0),
                (0.0, 1.0),
                (0.4, 1.0),
                (0.6, 0.8),
                (0.6, 0.65),
                (0.4, 0.5),
                (0.0, 0.5),
            ],
            &[(0.3, 0.5), (0.6, 0.0)],
        ],
        'S' => &[&[
            (0.6, 0.9),
            (0.45, 1.0),
            (0.15, 1.0),
            (0.0, 0.8),
            (0.1, 0.6),
            (0.5, 0.4),
            (0.6, 0.2),
            (0.45, 0.0),
            (0.15, 0.0),
            (0.0, 0.1),
        ]],
        'T' => &[&[(0.0, 1.0), (0.6, 1.0)], &[(0.3, 1.0), (0.3, 0.0)]],
        'U' => &[&[
            (0.0, 1.0),
            (0.0, 0.2),
            (0.15, 0.0),
            (0.45, 0.0),
            (0.6, 0.2),
            (0.6, 1.0),
        ]],
        'V' => &[&[(0.0, 1.0), (0.3, 0.0), (0.6, 1.0)]],
        'W' => &[&[(0.0, 1.0), (0.1, 0.0), (0.3, 0.5), (0.5, 0.0), (0.6, 1.0)]],
        'X' => &[&[(0.0, 1.0), (0.6, 0.0)], &[(0.0, 0.0), (0.6, 1.0)]],
        'Y' => &[
            &[(0.0, 1.0), (0.3, 0.5), (0.6, 1.0)],
            &[(0.3, 0.5), (0.3, 0.0)],
        ],
        'Z' => &[&[(0.0, 1.0), (0.6, 1.0), (0.0, 0.0), (0.6, 0.0)]],
        '1' => &[
            &[(0.1, 0.8), (0.3, 1.0), (0.3, 0.0)],
            &[(0.0, 0.0), (0.6, 0.0)],
        ],
        '2' => &[&[
            (0.0, 0.8),
            (0.15, 1.0),
            (0.45, 1.0),
            (0.6, 0.8),
            (0.6, 0.65),
            (0.0, 0.0),
            (0.6, 0.0),
        ]],
        '3' => &[&[
            (0.0, 0.9),
            (0.15, 1.0),
            (0.45, 1.0),
            (0.6, 0.8),
            (0.3, 0.5),
            (0.6, 0.2),
            (0.45, 0.0),
            (0.15, 0.0),
            (0.0, 0.1),
        ]],
        '4' => &[&[(0.45, 0.0), (0.45, 1.0), (0.0, 0.3), (0.6, 0.3)]],
        '5' => &[&[
            (0.6, 1.0),
            (0.0, 1.0),
            (0.0, 0.55),
            (0.4, 0.55),
            (0.6, 0.35),
            (0.6, 0.15),
            (0.45, 0.0),
            (0.15, 0.0),
            (0.0, 0.1),
        ]],
        '6' => &[&[
            (0.55, 0.9),
            (0.4, 1.0),
            (0.15, 0.8),
            (0.0, 0.4),
            (0.0, 0.2),
            (0.15, 0.0),
            (0.45, 0.0),
            (0.6, 0.2),
            (0.6, 0.4),
            (0.45, 0.6),
            (0.15, 0.6),
            (0.0, 0.4),
        ]],
        '7' => &[&[(0.0, 1.0), (0.6, 1.0), (0.15, 0.0)]],
        '8' => &[&[
            (0.3, 0.5),
            (0.0, 0.75),
            (0.15, 1.0),
            (0.45, 1.0),
            (0.6, 0.75),
            (0.3, 0.5),
            (0.0, 0.25),
            (0.15, 0.0),
            (0.45, 0.0),
            (0.6, 0.25),
            (0.3, 0.5),
        ]],
        '9' => &[&[
            (0.6, 0.6),
            (0.45, 0.4),
            (0.15, 0.4),
            (0.0, 0.6),
            (0.0, 0.8),
            (0.15, 1.0),
            (0.45, 1.0),
            (0.6, 0.8),
            (0.6, 0.6),
            (0.45, 0.2),
            (0.2, 0.0),
            (0.05, 0.1),
        ]],
        '!' => &[&[(0.3, 1.0), (0.3, 0.3)], &[(0.3, 0.05), (0.3, 0.0)]],
        '?' => &[
            &[
                (0.0, 0.8),
                (0.15, 1.0),
                (0.45, 1.0),
                (0.6, 0.8),
                (0.6, 0.65),
                (0.3, 0.4),
                (0.3, 0.3),
            ],
            &[(0.3, 0.05), (0.3, 0.0)],
        ],
        '♥' => &[&[
            (0.3, 0.1),
            (0.0, 0.55),
            (0.0, 0.75),
            (0.15, 0.85),
            (0.3, 0.65),
            (0.45, 0.85),
            (0.6, 0.75),
            (0.6, 0.55),
            (0.3, 0.1),
        ]],
        _ => &[],
    }
}

/// Hangul syllables use their Unicode initial, medial, and final indices.
pub(super) fn decompose(character: char) -> Option<(usize, usize, usize)> {
    let index = u32::from(character).checked_sub(0xac00)?;
    (index < 11172).then_some((
        (index / 588) as usize,
        ((index % 588) / 28) as usize,
        (index % 28) as usize,
    ))
}

pub(super) fn cell(character: char) -> (f32, f32) {
    if decompose(character).is_some() {
        (0.9, 1.05)
    } else {
        (0.6, 0.8)
    }
}

type Strokes = Vec<Vec<(f32, f32)>>;

pub(super) fn strokes(character: char) -> Strokes {
    let Some((initial, medial, final_index)) = decompose(character) else {
        return glyph(character)
            .iter()
            .map(|stroke| stroke.to_vec())
            .collect();
    };
    const INITIALS: [&str; 19] = [
        "ㄱ", "ㄱㄱ", "ㄴ", "ㄷ", "ㄷㄷ", "ㄹ", "ㅁ", "ㅂ", "ㅂㅂ", "ㅅ", "ㅅㅅ", "ㅇ", "ㅈ",
        "ㅈㅈ", "ㅊ", "ㅋ", "ㅌ", "ㅍ", "ㅎ",
    ];
    const FINALS: [&str; 28] = [
        "", "ㄱ", "ㄱㄱ", "ㄱㅅ", "ㄴ", "ㄴㅈ", "ㄴㅎ", "ㄷ", "ㄹ", "ㄹㄱ", "ㄹㅁ", "ㄹㅂ", "ㄹㅅ",
        "ㄹㅌ", "ㄹㅍ", "ㄹㅎ", "ㅁ", "ㅂ", "ㅂㅅ", "ㅅ", "ㅅㅅ", "ㅇ", "ㅈ", "ㅊ", "ㅋ", "ㅌ",
        "ㅍ", "ㅎ",
    ];
    let mut strokes = Vec::new();
    let bottom = if final_index == 0 { 0.0 } else { 0.32 };
    let height = 1.0 - bottom;
    let place = |bounds: [f32; 4]| {
        [
            bounds[0],
            bottom + bounds[1] * height,
            bounds[2],
            bounds[3] * height,
        ]
    };
    if let Some((horizontal, vertical)) = compound(medial) {
        consonants(
            &mut strokes,
            INITIALS[initial],
            place([0.0, 0.44, 0.52, 0.56]),
        );
        append(
            &mut strokes,
            vowel(horizontal),
            place([0.0, 0.0, 0.52, 0.36]),
        );
        append(&mut strokes, vowel(vertical), place([0.6, 0.0, 0.3, 1.0]));
    } else if matches!(medial, 8 | 12 | 13 | 17 | 18) {
        consonants(
            &mut strokes,
            INITIALS[initial],
            place([0.0, 0.44, 0.9, 0.56]),
        );
        append(&mut strokes, vowel(medial), place([0.0, 0.0, 0.9, 0.36]));
    } else {
        consonants(
            &mut strokes,
            INITIALS[initial],
            place([0.0, 0.0, 0.43, 1.0]),
        );
        append(&mut strokes, vowel(medial), place([0.51, 0.0, 0.39, 1.0]));
    }
    if final_index != 0 {
        consonants(&mut strokes, FINALS[final_index], [0.0, 0.0, 0.9, 0.25]);
    }
    strokes
}

fn append(strokes: &mut Strokes, source: Strokes, bounds: [f32; 4]) {
    strokes.extend(source.into_iter().map(|stroke| {
        stroke
            .into_iter()
            .map(|(horizontal, vertical)| {
                (
                    bounds[0] + horizontal * bounds[2],
                    bounds[1] + vertical * bounds[3],
                )
            })
            .collect()
    }));
}

fn consonants(strokes: &mut Strokes, letters: &str, bounds: [f32; 4]) {
    let count = letters.chars().count() as f32;
    for (index, letter) in letters.chars().enumerate() {
        let width = bounds[2] / count;
        append(
            strokes,
            consonant(letter),
            [
                bounds[0] + index as f32 * width,
                bounds[1],
                width,
                bounds[3],
            ],
        );
    }
}

fn consonant(character: char) -> Strokes {
    let paths: &[&[(f32, f32)]] = match character {
        'ㄱ' => &[&[(0.08, 0.92), (0.92, 0.92), (0.92, 0.08)]],
        'ㄴ' => &[&[(0.08, 0.92), (0.08, 0.08), (0.92, 0.08)]],
        'ㄷ' => &[&[(0.92, 0.92), (0.08, 0.92), (0.08, 0.08), (0.92, 0.08)]],
        'ㄹ' => &[&[
            (0.08, 0.92),
            (0.92, 0.92),
            (0.92, 0.5),
            (0.08, 0.5),
            (0.08, 0.08),
            (0.92, 0.08),
        ]],
        'ㅁ' => &[&[
            (0.08, 0.92),
            (0.92, 0.92),
            (0.92, 0.08),
            (0.08, 0.08),
            (0.08, 0.92),
        ]],
        'ㅂ' => &[
            &[(0.08, 0.92), (0.08, 0.08), (0.92, 0.08), (0.92, 0.92)],
            &[(0.08, 0.5), (0.92, 0.5)],
        ],
        'ㅅ' => &[&[(0.08, 0.08), (0.5, 0.92), (0.92, 0.08)]],
        'ㅈ' => &[
            &[(0.08, 0.92), (0.92, 0.92)],
            &[(0.08, 0.08), (0.5, 0.85), (0.92, 0.08)],
        ],
        'ㅊ' => &[
            &[(0.35, 0.96), (0.65, 0.96)],
            &[(0.08, 0.72), (0.92, 0.72)],
            &[(0.08, 0.08), (0.5, 0.65), (0.92, 0.08)],
        ],
        'ㅋ' => &[
            &[(0.08, 0.92), (0.92, 0.92), (0.92, 0.08)],
            &[(0.08, 0.5), (0.92, 0.5)],
        ],
        'ㅌ' => &[
            &[(0.92, 0.92), (0.08, 0.92), (0.08, 0.08), (0.92, 0.08)],
            &[(0.08, 0.5), (0.92, 0.5)],
        ],
        'ㅍ' => &[
            &[(0.08, 0.92), (0.92, 0.92)],
            &[(0.08, 0.08), (0.92, 0.08)],
            &[(0.28, 0.92), (0.28, 0.08)],
            &[(0.72, 0.92), (0.72, 0.08)],
        ],
        'ㅇ' | 'ㅎ' => {
            let (center, radius) = if character == 'ㅎ' {
                (0.32, 0.26)
            } else {
                (0.5, 0.42)
            };
            let circle = (0..=12)
                .map(|index| {
                    let angle = index as f32 * std::f32::consts::TAU / 12.0;
                    (0.5 + 0.42 * angle.cos(), center + radius * angle.sin())
                })
                .collect();
            let mut strokes = vec![circle];
            if character == 'ㅎ' {
                strokes.extend([
                    vec![(0.35, 0.96), (0.65, 0.96)],
                    vec![(0.08, 0.76), (0.92, 0.76)],
                ]);
            }
            return strokes;
        }
        _ => unreachable!(),
    };
    paths.iter().map(|path| path.to_vec()).collect()
}

fn compound(medial: usize) -> Option<(usize, usize)> {
    match medial {
        9 => Some((8, 0)),
        10 => Some((8, 1)),
        11 => Some((8, 20)),
        14 => Some((13, 4)),
        15 => Some((13, 5)),
        16 => Some((13, 20)),
        19 => Some((18, 20)),
        _ => None,
    }
}

fn vowel(medial: usize) -> Strokes {
    let mut strokes = Vec::new();
    if matches!(medial, 8 | 12 | 13 | 17 | 18) {
        strokes.push(vec![(0.08, 0.5), (0.92, 0.5)]);
        if medial != 18 {
            let end = if matches!(medial, 8 | 12) { 0.92 } else { 0.08 };
            let positions: &[f32] = if matches!(medial, 12 | 17) {
                &[0.32, 0.68]
            } else {
                &[0.5]
            };
            for &horizontal in positions {
                strokes.push(vec![(horizontal, 0.5), (horizontal, end)]);
            }
        }
    } else {
        let paired = matches!(medial, 1 | 3 | 5 | 7);
        let leftward = matches!(medial, 4..=7);
        let axis = if paired {
            if leftward { 0.5 } else { 0.15 }
        } else {
            0.5
        };
        strokes.push(vec![(axis, 0.08), (axis, 0.92)]);
        if paired {
            strokes.push(vec![(0.85, 0.08), (0.85, 0.92)]);
        }
        if medial != 20 {
            let end = if leftward {
                0.08
            } else if paired {
                0.6
            } else {
                0.92
            };
            let positions: &[f32] = if matches!(medial, 2 | 3 | 6 | 7) {
                &[0.35, 0.65]
            } else {
                &[0.5]
            };
            for &vertical in positions {
                strokes.push(vec![(axis, vertical), (end, vertical)]);
            }
        }
    }
    strokes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_syllables_decompose_into_their_unicode_jamo_indices() {
        for (character, expected) in [
            ('가', (0, 0, 0)),
            ('각', (0, 0, 1)),
            ('똠', (4, 8, 16)),
            ('뷁', (7, 15, 9)),
            ('의', (11, 19, 0)),
            ('한', (18, 0, 4)),
            ('글', (0, 18, 8)),
        ] {
            assert_eq!(decompose(character), Some(expected));
        }
        assert_eq!(decompose('A'), None);
        assert_eq!(decompose('ㄱ'), None);
    }

    #[test]
    fn every_hangul_syllable_has_nonempty_strokes_inside_its_cell() {
        for character in '가'..='힣' {
            let strokes = strokes(character);
            assert!(!strokes.is_empty(), "{character}");
            for stroke in strokes {
                assert!(stroke.len() >= 2, "{character}");
                assert!(
                    stroke.iter().all(|&(horizontal, vertical)| {
                        (0.0..=0.9).contains(&horizontal) && (0.0..=1.0).contains(&vertical)
                    }),
                    "{character}"
                );
            }
        }
    }

    #[test]
    fn jamo_layout_reserves_separate_regions_for_vowels_and_finals() {
        let vertical = strokes('가');
        assert!(vertical[0].iter().all(|point| point.0 < 0.45));
        assert!(vertical[1..].iter().flatten().all(|point| point.0 > 0.5));
        let horizontal = strokes('고');
        assert!(horizontal[0].iter().all(|point| point.1 > 0.44));
        assert!(horizontal[1..].iter().flatten().all(|point| point.1 < 0.36));
        let compound = strokes('과');
        assert!(
            compound[0]
                .iter()
                .all(|point| point.0 < 0.52 && point.1 > 0.44)
        );
        assert!(
            compound[1..3]
                .iter()
                .flatten()
                .all(|point| point.0 < 0.52 && point.1 < 0.36)
        );
        assert!(compound[3..].iter().flatten().all(|point| point.0 > 0.6));
        for character in ['각', '똠', '뷁', '한', '글', '값'] {
            let (initial, medial, _) = decompose(character).unwrap();
            let without_final =
                char::from_u32(0xac00 + (initial * 588 + medial * 28) as u32).unwrap();
            let upper_count = strokes(without_final).len();
            let with_final = strokes(character);
            assert!(
                with_final[..upper_count]
                    .iter()
                    .flatten()
                    .all(|point| point.1 >= 0.32)
            );
            assert!(
                with_final[upper_count..]
                    .iter()
                    .flatten()
                    .all(|point| point.1 <= 0.25)
            );
        }
    }

    #[test]
    fn every_glyph_has_nonempty_strokes_inside_its_cell() {
        for character in ('A'..='Z').chain('0'..='9').chain(['!', '?', '♥']) {
            let strokes = glyph(character);
            assert!(!strokes.is_empty(), "{character}");
            for stroke in strokes {
                assert!(stroke.len() >= 2);
                assert!(stroke.iter().all(|&(horizontal, vertical)| {
                    (0.0..=0.6).contains(&horizontal) && (0.0..=1.0).contains(&vertical)
                }));
            }
        }
    }
}
