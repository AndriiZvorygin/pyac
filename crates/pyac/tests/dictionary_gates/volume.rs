//! The volume writer gate: decode then encode must reproduce the bytes.
//!
//! A lossy decode would corrupt every entry an edit command touched, silently —
//! the volume would still verify, because the decoder and the verifier share the
//! same reading of the format. Only a byte comparison against the original catches
//! that.

use super::fixtures;

#[test]
fn decoding_the_published_volume_reproduces_it_byte_for_byte() {
    let (Some(volume), Some(original)) = (fixtures::volume(), fixtures::published_bytes()) else {
        return;
    };

    assert!(
        volume.entries.len() > 64_000,
        "expected the whole vocabulary, decoded {} entries",
        volume.entries.len()
    );

    let reencoded =
        pyac::dictionary::encode_dictionary(volume).expect("the decoded model re-encodes");
    fixtures::assert_identical(&reencoded, original, "decode -> encode");
}

#[test]
fn rendering_the_published_volume_and_recompiling_reproduces_it() {
    // The model surviving decode/encode is weaker than the *source text*
    // surviving render/parse: this exercises the source writer, the parser's
    // inverse, and every record kind at full scale. A renderer that drops a field
    // or escapes text differently still decodes cleanly and fails only here.
    let (Some(rendered), Some(original)) =
        (fixtures::rendered_source(), fixtures::published_bytes())
    else {
        return;
    };

    let reparsed = pyac::dictionary::parse_source(rendered).expect("the rendered source parses");
    let volume =
        pyac::dictionary::source_to_volume(reparsed).expect("the rendered source becomes a volume");
    let recompiled =
        pyac::dictionary::encode_dictionary(&volume).expect("the rendered source re-encodes");

    fixtures::assert_identical(&recompiled, original, "render -> compile");
}
