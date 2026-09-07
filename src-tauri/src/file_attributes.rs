pub fn attributes_to_hide(original: u32) -> u32 {
    original | 0x2 | 0x4
}

pub fn attributes_to_restore(original: u32) -> u32 {
    original
}

#[cfg(test)]
mod tests {
    use super::*;

    const HIDDEN: u32 = 0x2;
    const SYSTEM: u32 = 0x4;
    const ARCHIVE: u32 = 0x20;

    #[test]
    fn hiding_preserves_existing_flags_and_adds_hidden_and_system() {
        assert_eq!(attributes_to_hide(ARCHIVE), ARCHIVE | HIDDEN | SYSTEM);
    }

    #[test]
    fn restoring_returns_exact_original_attribute_mask() {
        assert_eq!(
            attributes_to_restore(ARCHIVE | HIDDEN | SYSTEM),
            ARCHIVE | HIDDEN | SYSTEM
        );
    }
}
