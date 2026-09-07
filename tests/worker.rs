use brotli::CompressorWriter;
use judge_ma_di::worker::decode_source_code;
use std::io::Write;

fn compress_json(value: &impl serde::Serialize) -> Vec<u8> {
    let json = serde_json::to_vec(value).unwrap();
    let mut compressor = CompressorWriter::new(Vec::new(), 4096, 11, 22);
    compressor.write_all(&json).unwrap();
    compressor.into_inner()
}

fn compress_code(source: &str) -> Vec<u8> {
    compress_json(&vec![source])
}

#[test]
fn decodes_a_single_element_array_back_to_the_source() {
    let compressed = compress_code("int main() {}");

    assert_eq!(decode_source_code(&compressed).unwrap(), "int main() {}");
}

#[test]
fn decodes_a_bare_json_string_back_to_the_source() {
    // Real-world shape: the frontend types decompressCode's result as
    // string[], but observed submissions store a plain JSON string.
    let compressed = compress_json(&"int main() {}");

    assert_eq!(decode_source_code(&compressed).unwrap(), "int main() {}");
}

#[test]
fn errors_on_an_empty_array() {
    let json = serde_json::to_vec(&Vec::<String>::new()).unwrap();
    let mut compressor = CompressorWriter::new(Vec::new(), 4096, 11, 22);
    compressor.write_all(&json).unwrap();
    let compressed = compressor.into_inner();

    assert!(decode_source_code(&compressed).is_err());
}

#[test]
fn errors_on_invalid_brotli_data() {
    assert!(decode_source_code(b"not brotli data").is_err());
}

#[test]
fn decodes_a_typical_python_solution() {
    // Same bare-JSON-string shape observed in production, with an
    // ordinary a+b solution rather than a real user's submission.
    let source = "a = int(input())\r\nb = int(input())\r\nprint(a + b)";
    let compressed = compress_json(&source);

    assert_eq!(decode_source_code(&compressed).unwrap(), source);
}

#[test]
fn errors_on_brotli_bomb_exceeding_10mb() {
    // 11 MB of repeated characters compresses to small brotli payload
    let large_code = "a".repeat(11 * 1024 * 1024);
    let compressed = compress_json(&large_code);

    let err = decode_source_code(&compressed).unwrap_err();
    assert!(err.to_string().contains("exceeds maximum size"));
}
