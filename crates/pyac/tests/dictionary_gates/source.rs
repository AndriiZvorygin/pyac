//! The bootstrap gate: the authored source must compile to the published volume.
//!
//! This is the test that makes the repository self-hosting. Everything else can be
//! argued about; if the source compiles to a different volume than the one the
//! project ships, then regenerating the dictionary depends on something outside
//! this tree and the extraction has not achieved its purpose.
//!
//! The companion gates below it prove the same properties *without* the published
//! volume, so a clean clone still checks something. This one needs both artifacts,
//! so it skips when either is absent.

use super::fixtures;

#[test]
fn compiling_the_source_reproduces_the_published_volume() {
    let (Some(compiled), Some(published)) =
        (fixtures::compiled_source(), fixtures::published_bytes())
    else {
        return;
    };
    fixtures::assert_identical(compiled, published, "compile");
}

#[test]
fn the_authored_source_parses_at_full_scale() {
    let Some(parsed) = fixtures::parsed() else {
        return;
    };

    assert!(
        parsed.entries.len() > 60_000,
        "expected the whole vocabulary, parsed {} entries",
        parsed.entries.len()
    );
    assert!(
        !parsed.metadata.languages.is_empty(),
        "the profile metadata must declare languages"
    );

    // Every entry must be addressable. A key is mandatory; an English surface is
    // not — 18 compounds are reachable only through their components, which is how
    // the source records a decomposition with no English name of its own.
    let mut without_key = 0;
    let mut without_any_reference = 0;
    for entry in &parsed.entries {
        if entry.key.is_empty() {
            without_key += 1;
        }
        let has_something = !entry.lexical_forms.is_empty()
            || !entry.component_keys.is_empty()
            || !entry.source_links.is_empty()
            || !entry.source_glosses.is_empty();
        if !has_something {
            without_any_reference += 1;
        }
    }
    assert_eq!(without_key, 0, "{without_key} entries have no key");
    assert_eq!(
        without_any_reference, 0,
        "{without_any_reference} entries have neither a form, a component, nor a reference"
    );
}

#[test]
fn compiling_the_source_is_deterministic() {
    // Determinism is a property of the encoder: the same model must give the same
    // bytes. Parsing the 96 MB source twice would only re-test the parser, and it
    // costs more than everything else in this binary.
    let Some(parsed) = fixtures::parsed() else {
        return;
    };
    let volume =
        pyac::dictionary::source_to_volume(parsed.clone()).expect("the source becomes a volume");
    let first = pyac::dictionary::encode_dictionary(&volume).expect("the parsed source encodes");
    let second =
        pyac::dictionary::encode_dictionary(&volume).expect("the parsed source encodes again");
    assert_eq!(
        first, second,
        "compile is not deterministic; the encoder has nondeterministic ordering"
    );
}

#[test]
fn a_compiled_source_decodes_and_re_encodes_identically() {
    // The round-trip property, provable without the published volume. It is weaker
    // than byte-identity against the shipped artifact, but it runs everywhere and
    // catches any lossy encode or decode.
    let Some(compiled) = fixtures::compiled_source() else {
        return;
    };

    let volume = pyac::dictionary::decode_dictionary(std::io::Cursor::new(compiled.to_vec()))
        .expect("the compiled volume decodes");
    assert!(
        volume.entries.len() > 60_000,
        "decoded {} entries",
        volume.entries.len()
    );
    let reencoded =
        pyac::dictionary::encode_dictionary(&volume).expect("the decoded volume re-encodes");
    fixtures::assert_identical(&reencoded, compiled, "compile -> decode -> encode");
}
