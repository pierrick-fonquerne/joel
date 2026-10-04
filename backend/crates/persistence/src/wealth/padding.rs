//! Space padding applied before encryption so that ciphertext length does not
//! reveal the size of the clear value.

use domain::wealth::WealthError;

/// Fixed byte width of an encrypted decimal amount.
pub(crate) const AMOUNT_WIDTH: usize = 40;

/// Block size, in bytes, that encrypted free text is padded up to.
pub(crate) const TEXT_BLOCK: usize = 64;

/// Right-pads `text` with spaces to exactly `width` bytes.
///
/// # Errors
/// Returns [`WealthError::Cipher`] when `text` is longer than `width`.
pub(crate) fn pad_to_width(text: &str, width: usize) -> Result<Vec<u8>, WealthError> {
    if text.len() > width {
        return Err(WealthError::Cipher);
    }
    let mut bytes = text.as_bytes().to_vec();
    bytes.resize(width, b' ');
    Ok(bytes)
}

/// Right-pads `text` with spaces to the next multiple of [`TEXT_BLOCK`] bytes
/// (at least one block).
pub(crate) fn pad_to_block(text: &str) -> Vec<u8> {
    let width = text.len().div_ceil(TEXT_BLOCK).max(1) * TEXT_BLOCK;
    let mut bytes = text.as_bytes().to_vec();
    bytes.resize(width, b' ');
    bytes
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn width_padding_is_exact_and_rejects_overflow() {
        assert_eq!(pad_to_width("1.5", 5).unwrap(), b"1.5  ");
        assert_eq!(pad_to_width("12345", 5).unwrap(), b"12345");
        assert_eq!(pad_to_width("123456", 5), Err(WealthError::Cipher));
    }

    #[test]
    fn block_padding_rounds_up_to_sixty_four() {
        assert_eq!(pad_to_block("").len(), 64);
        assert_eq!(pad_to_block("A").len(), 64);
        assert_eq!(pad_to_block(&"B".repeat(64)).len(), 64);
        assert_eq!(pad_to_block(&"B".repeat(65)).len(), 128);
    }
}
