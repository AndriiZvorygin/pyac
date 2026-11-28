use std::fs;
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use pyac::dictionary::{
    ArgumentSlot, DictionaryField, DictionaryMetadata, DictionaryValue, DictionaryVolume,
    DictionaryVolumeEntry, LexicalForm, OperationParameter, OperationSignature, SourceGloss,
    SourceLink, encode_dictionary, encode_text_sentence,
};
use serde_json::Value;

fn temporary_directory() -> std::path::PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "pyac-dictionary-cli-{}-{timestamp}",
        std::process::id()
    ))
}

/// The fixture volume, built through the library.
///
/// The CLI test is about the CLI: `examine`, `surface`, `entry`, `criterion`.
/// Building its fixture by hand in English Pyash would be inventing source the
/// repository does not otherwise contain, and `dictionary_compile.rs` is already
/// the gate on source-to-volume. So the fixture is constructed as a model and
/// encoded, and the test reads it back through the command line.
fn fixture() -> DictionaryVolume {
    let definition = encode_text_sentence(
        &[0xa96a, 0x6c22],
        &[DictionaryField {
            grammatical_case: 0x245e,
            value: DictionaryValue::Text("A dictionary entry used by the CLI test."),
        }],
    )
    .expect("encode definition");
    DictionaryVolume {
        metadata: DictionaryMetadata {
            kind: "English Pyash dictionary".to_owned(),
            title: "CLI test volume".to_owned(),
            revision: 1,
            languages: vec!["en".to_owned()],
            normalization_policy: "NFC".to_owned(),
        },
        entries: vec![DictionaryVolumeEntry {
            key: vec![0xa96a, 0x6c22],
            lexical_forms: vec![LexicalForm {
                language: "en".to_owned(),
                surface: "dictionary".to_owned(),
                normalized: "dictionary".to_owned(),
            }],
            source_links: vec![SourceLink {
                dataset: "wordnet".to_owned(),
                release: "OEWN 2025".to_owned(),
                identifier: "n:06311813-n".to_owned(),
                relation: "exact-sense".to_owned(),
            }],
            source_glosses: vec![SourceGloss {
                language: "en".to_owned(),
                text: "A dictionary entry used by the CLI test.".to_owned(),
                dataset: "wordnet".to_owned(),
                release: "OEWN 2025".to_owned(),
                identifier: "n:06311813-n".to_owned(),
            }],
            component_keys: Vec::new(),
            task_keys: Vec::new(),
            references: Vec::new(),
            operation_signatures: vec![OperationSignature {
                verb: vec![0xa96a],
                mood: vec![0x245e],
                arguments: vec![OperationParameter {
                    case_type: ArgumentSlot {
                        grammatical_case: vec![0x229e],
                        noun_class: vec![0x225e],
                    },
                    tag_label: "subject".to_owned(),
                    descriptive_specification: "what the entry is about".to_owned(),
                }],
                results: Vec::new(),
            }],
            definition: Some(definition),
        }],
    }
}

fn run(args: &[&str]) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_pyac-dictionary"))
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run dictionary executable");
    assert!(
        output.status.success(),
        "command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("JSON command result")
}

#[test]
fn compiled_cli_examines_looks_up_and_measures_a_volume() {
    let directory = temporary_directory();
    fs::create_dir_all(&directory).expect("create temp directory");
    let volume = directory.join("dictionary.pyac");
    let volume_arg = volume.to_str().expect("UTF-8 temp path");
    fs::write(
        &volume,
        encode_dictionary(&fixture()).expect("encode volume"),
    )
    .expect("write volume");

    let inspected = run(&["examine", volume_arg]);
    assert_eq!(inspected["metadata"]["title"], "CLI test volume");
    assert_eq!(inspected["entryCount"], 1);
    assert!(inspected["openMs"].as_f64().is_some());

    let surface = run(&["surface", volume_arg, "en", "dictionary"]);
    let surface_entries = run(&["surface-entries", volume_arg, "en", "dictionary"]);
    let offset = surface_entries["matches"][0]["offset"].clone();
    assert_eq!(surface["matches"], serde_json::json!([offset]));
    assert!(
        !surface_entries["matches"][0]["codelets"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(surface["lookupNs"].as_u64().is_some());

    let source = run(&[
        "source-entries",
        volume_arg,
        "wordnet",
        "OEWN 2025",
        "n:06311813-n",
    ]);
    assert_eq!(source["matches"].as_array().unwrap().len(), 1);
    assert!(
        !source["matches"][0]["codelets"]
            .as_array()
            .unwrap()
            .is_empty()
    );

    let entry = run(&["key", volume_arg, "a96a,6c22"]);
    assert_eq!(entry["key"], serde_json::json!([0xa96a, 0x6c22]));
    assert!(!entry["codelets"].as_array().unwrap().is_empty());

    let full_entry = run(&["entry", volume_arg, "a96a,6c22"]);
    assert_eq!(full_entry["hasDefinition"], true);
    assert!(
        full_entry["codelets"].as_array().unwrap().len()
            > entry["codelets"].as_array().unwrap().len()
    );

    let benchmark = run(&[
        "criterion",
        volume_arg,
        "1000",
        "surface",
        "en",
        "dictionary",
    ]);
    assert_eq!(benchmark["iterations"], 1000);
    assert_eq!(benchmark["warmupIterations"], 5);
    assert_eq!(benchmark["entryCount"], 1);
    assert_eq!(benchmark["matchesPerIteration"], 1);
    assert!(benchmark["lookupMeanNs"].as_u64().is_some());
    assert!(benchmark["lookupsPerSecond"].as_f64().unwrap() > 0.0);
    fs::remove_dir_all(directory).expect("remove temp directory");
}
