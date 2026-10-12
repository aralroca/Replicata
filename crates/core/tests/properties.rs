//! Property tests for the canonical encodings.

use std::collections::BTreeMap;

use proptest::prelude::*;
use replicata_core::bundle::{hash_bundle, pack_files};
use replicata_core::{commitment, hash_files, JobSpec};

fn file_name() -> impl Strategy<Value = String> {
    proptest::collection::vec("[a-z0-9_]{1,8}", 1..3).prop_map(|parts| parts.join("/"))
}

fn files() -> impl Strategy<Value = BTreeMap<String, Vec<u8>>> {
    proptest::collection::btree_map(file_name(), proptest::collection::vec(any::<u8>(), 0..64), 0..8)
        // A name cannot be both a file and a directory prefix of another file.
        .prop_filter("no file/dir clashes", |map| {
            map.keys().all(|a| map.keys().all(|b| !b.starts_with(&format!("{a}/"))))
        })
}

fn spec() -> impl Strategy<Value = JobSpec> {
    (
        "[ -~]{1,40}",
        "\\PC{0,60}",
        proptest::collection::btree_map("[A-Z_]{1,10}", "[ -~]{0,10}", 0..4),
        any::<u64>(),
        proptest::collection::vec("[ -~]{1,10}", 1..4),
    )
        .prop_map(|(title, description, env, seed, cmd)| JobSpec {
            version: 1,
            title,
            description,
            image: "alpine".into(),
            image_digest: format!("sha256:{}", "ab".repeat(32)),
            cmd,
            env,
            inputs: vec![],
            cpus: 1,
            mem_mb: 128,
            timeout_s: 10,
            seed,
        })
}

proptest! {
    #[test]
    fn output_hash_is_order_independent(files in files(), seed in any::<u64>()) {
        let mut entries: Vec<(&str, &[u8])> = files.iter().map(|(n, b)| (n.as_str(), b.as_slice())).collect();
        let sorted = hash_files(entries.clone()).unwrap();

        let len = entries.len().max(1);
        entries.rotate_left((seed as usize) % len);
        entries.reverse();
        prop_assert_eq!(hash_files(entries).unwrap(), sorted);
    }

    #[test]
    fn changing_one_byte_or_name_changes_the_hash(files in files(), flip in any::<u8>()) {
        prop_assume!(!files.is_empty());
        let base = hash_files(files.iter().map(|(n, b)| (n.as_str(), b.as_slice()))).unwrap();
        let (first_name, first_bytes) = files.iter().next().unwrap();

        let mut changed_bytes = first_bytes.clone();
        match changed_bytes.first_mut() {
            Some(byte) => *byte ^= flip.max(1),
            None => changed_bytes.push(flip),
        }
        let with_bytes = files.iter().map(|(n, b)| if n == first_name { (n.as_str(), changed_bytes.as_slice()) } else { (n.as_str(), b.as_slice()) });
        prop_assert_ne!(hash_files(with_bytes).unwrap(), base);

        let renamed = format!("{first_name}x");
        prop_assume!(!files.contains_key(&renamed));
        let with_name = files.iter().map(|(n, b)| if n == first_name { (renamed.as_str(), b.as_slice()) } else { (n.as_str(), b.as_slice()) });
        prop_assert_ne!(hash_files(with_name).unwrap(), base);
    }

    #[test]
    fn bundle_roundtrip_preserves_the_hash(files in files()) {
        let list: Vec<(String, Vec<u8>)> = files.clone().into_iter().collect();
        let bundle = pack_files(&list).unwrap();

        prop_assert_eq!(
            hash_bundle(&bundle).unwrap(),
            hash_files(files.iter().map(|(n, b)| (n.as_str(), b.as_slice()))).unwrap()
        );
    }

    #[test]
    fn spec_json_roundtrip_keeps_the_hash(spec in spec()) {
        let json = spec.canonical_json();
        let parsed = JobSpec::parse(&json).unwrap();

        prop_assert_eq!(parsed.spec_hash(), spec.spec_hash());
        prop_assert_eq!(parsed.canonical_json(), json);
    }

    #[test]
    fn commitment_is_injective_in_practice(a in any::<[u8; 32]>(), b in any::<[u8; 32]>(), w in any::<[u8; 32]>()) {
        prop_assume!(a != b);
        prop_assert_ne!(commitment(&a, &b, &w), commitment(&b, &a, &w));
        prop_assert_ne!(commitment(&a, &a, &w), commitment(&b, &a, &w));
    }
}
