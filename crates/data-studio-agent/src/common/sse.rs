/// Drain complete SSE event blocks from a byte buffer.
///
/// Network chunks from `bytes_stream()` may split a multi-byte UTF-8 sequence
/// mid-character, so decoding per chunk corrupts the stream (each orphaned byte
/// becomes U+FFFD). A block terminated by `\n\n` can never end inside a
/// multi-byte char (`0x0A` is ASCII and never appears in a UTF-8 continuation
/// byte), so decoding only complete blocks is always lossless.
///
/// Returns `false` when a block consumer requests to stop, `true` when the
/// buffer holds no more complete blocks.
pub fn drain_event_blocks(buf: &mut Vec<u8>, mut on_block: impl FnMut(&str) -> bool) -> bool {
    loop {
        let Some(pos) = buf.windows(2).position(|w| w == b"\n\n") else {
            return true;
        };
        let block: String = String::from_utf8_lossy(&buf[..pos]).into_owned();
        buf.drain(..pos + 2);
        if !on_block(&block) {
            return false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blocks_split_across_chunks_preserve_multibyte_chars() {
        // 客 = E5 AE A2, 州 = E5 B7 9E — split inside both characters across feeds.
        let mut buf = Vec::new();
        buf.extend_from_slice("data: {\"x\": \"".as_bytes());
        buf.extend_from_slice(&[0xE5]); // first byte of 客
        assert!(drain_event_blocks(&mut buf, |_| panic!("no complete block yet")));
        buf.extend_from_slice(&[0xAE, 0xA2]);
        buf.extend_from_slice("测试\"}\n\ndata: {\"y\": \"杭".as_bytes());
        buf.extend_from_slice(&[0xE5, 0xB7]); // first two bytes of 州
        let mut blocks = Vec::new();
        assert!(drain_event_blocks(&mut buf, |b| {
            blocks.push(b.to_string());
            true
        }));
        assert_eq!(blocks, vec![r#"data: {"x": "客测试"}"#.to_string()]);
        let mut expected: Vec<u8> = "data: {\"y\": \"杭".as_bytes().to_vec();
        expected.extend_from_slice(&[0xE5, 0xB7]);
        assert_eq!(buf, expected);

        // Remaining partial block completes on the next chunk.
        buf.extend_from_slice(&[0x9E, b'"', b'}', b'\n', b'\n']);
        assert!(drain_event_blocks(&mut buf, |b| {
            blocks.push(b.to_string());
            true
        }));
        assert_eq!(blocks[1], r#"data: {"y": "杭州"}"#);
        assert!(buf.is_empty());
    }

    #[test]
    fn test_consumer_stop_propagates() {
        let mut buf = b"block one\n\ndata: [DONE]\n\n".to_vec();
        let mut seen = Vec::new();
        let ok = drain_event_blocks(&mut buf, |b| {
            seen.push(b.to_string());
            !b.contains("[DONE]")
        });
        assert!(!ok);
        assert_eq!(seen, vec!["block one".to_string(), "data: [DONE]".to_string()]);
        assert!(buf.is_empty());
    }
}
