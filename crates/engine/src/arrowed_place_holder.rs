/// Describes where a capelito's word starts relative to its definition cell,
/// and in which direction it runs.
#[derive(Debug, PartialEq, Eq)]
pub struct ArrowedPlaceHolder {
    pub capelito_type: u8,
    pub i_diff: isize,
    pub j_diff: isize,
    pub is_horizontal: bool,
    pub unicode_char: char, // For graphical purposes
    pub upper_location: bool, // For graphical purposes
}

/// Ordered by `capelito_type`, so type `n` lives at index `n - 1`.
pub const ARROWED_PLACE_HOLDERS: [ArrowedPlaceHolder; 6] = [
    ArrowedPlaceHolder {
        capelito_type: 1,
        i_diff: 0,
        j_diff: 1,
        is_horizontal: true,
        unicode_char: '\u{2192}',
        upper_location: true,
    },
    ArrowedPlaceHolder {
        capelito_type: 2,
        i_diff: 1,
        j_diff: 0,
        is_horizontal: false,
        unicode_char: '\u{2193}',
        upper_location: false,
    },
    ArrowedPlaceHolder {
        capelito_type: 3,
        i_diff: 0,
        j_diff: 1,
        is_horizontal: false,
        unicode_char: '\u{21B4}',
        upper_location: true,
    },
    ArrowedPlaceHolder {
        capelito_type: 4,
        i_diff: 0,
        j_diff: -1,
        is_horizontal: false,
        unicode_char: '\u{21B4}',
        upper_location: true,
    },
    ArrowedPlaceHolder {
        capelito_type: 5,
        i_diff: -1,
        j_diff: 0,
        is_horizontal: true,
        unicode_char: '\u{21B1}',
        upper_location: false,
    },
    ArrowedPlaceHolder {
        capelito_type: 6,
        i_diff: 1,
        j_diff: 0,
        is_horizontal: true,
        unicode_char: '\u{21B3}',
        upper_location: false,
    },
];

pub fn get_arrowed_place_holder(capelito_type: u8) -> Option<&'static ArrowedPlaceHolder> {
    let index = capelito_type.checked_sub(1)? as usize;
    ARROWED_PLACE_HOLDERS.get(index)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_matches_capelito_type() {
        for capelito_type in 1..=6 {
            let place_holder = get_arrowed_place_holder(capelito_type).unwrap();
            assert_eq!(place_holder.capelito_type, capelito_type);
        }
    }

    #[test]
    fn lookup_rejects_unknown_types() {
        assert!(get_arrowed_place_holder(0).is_none());
        assert!(get_arrowed_place_holder(7).is_none());
    }
}
