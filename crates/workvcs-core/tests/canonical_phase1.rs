use proptest::prelude::*;
use workvcs_core::canonical::ImportDigestDomain;
use workvcs_core::*;

fn uuid(bytes: [u8; 16]) -> EntityId {
    EntityId::from_bytes(v7_bytes(bytes)).unwrap()
}

fn entity_version(bytes: [u8; 16]) -> EntityVersionId {
    EntityVersionId::from_bytes(v7_bytes(bytes)).unwrap()
}

fn relation(bytes: [u8; 16]) -> RelationId {
    RelationId::from_bytes(v7_bytes(bytes)).unwrap()
}

fn relation_version(bytes: [u8; 16]) -> RelationVersionId {
    RelationVersionId::from_bytes(v7_bytes(bytes)).unwrap()
}

fn v7_bytes(mut bytes: [u8; 16]) -> [u8; 16] {
    bytes[6] = (bytes[6] & 0x0f) | 0x70;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    bytes
}

#[test]
fn ce_01_to_ce_04_profile_rejects_float_and_accepts_safe_integer_domain() {
    assert!(parse_canonical_json(br#"{"n":9007199254740991}"#).is_ok());
    assert!(parse_canonical_json(br#"{"n":9007199254740992}"#).is_err());
    assert!(parse_canonical_json(br#"{"n":1.0}"#).is_err());
    assert!(parse_canonical_json(br#"{"n":1e0}"#).is_err());
}

#[test]
fn ce_05_duplicate_keys_are_invalid() {
    assert!(parse_canonical_json(br#"{"a":1,"a":2}"#).is_err());
}

#[test]
fn ce_06_ce_07_jcs_sorting_uses_utf16_and_preserves_array_order() {
    let input = r#"{"z":[2,1],"":1,"𐀀":2,"a":3}"#;
    let value = parse_canonical_json(input.as_bytes()).unwrap();
    let bytes = canonical_bytes(&value).unwrap();
    assert_eq!(
        String::from_utf8(bytes).unwrap(),
        r#"{"a":3,"z":[2,1],"𐀀":2,"":1}"#
    );
}

#[test]
fn ce_08_different_map_insertion_order_yields_same_canonical_bytes_and_digest() {
    let left = CanonicalValue::object(vec![
        ("b".to_owned(), CanonicalValue::safe_integer(2).unwrap()),
        ("a".to_owned(), CanonicalValue::safe_integer(1).unwrap()),
    ])
    .unwrap();
    let right = CanonicalValue::object(vec![
        ("a".to_owned(), CanonicalValue::safe_integer(1).unwrap()),
        ("b".to_owned(), CanonicalValue::safe_integer(2).unwrap()),
    ])
    .unwrap();

    assert_eq!(
        canonical_bytes(&left).unwrap(),
        canonical_bytes(&right).unwrap()
    );
    assert_eq!(
        entity_version_digest(&left).unwrap(),
        entity_version_digest(&right).unwrap()
    );
}

#[test]
fn ce_09_uuid_and_digest_text_are_strictly_lowercase_canonical() {
    let id = EntityId::parse_canonical("018f0f5a-75c0-7b27-8f77-68e8f5e9274f").unwrap();
    assert_eq!(id.as_uuid().get_version_num(), 7);
    assert_eq!(id.to_string(), "018f0f5a-75c0-7b27-8f77-68e8f5e9274f");
    assert!(EntityId::parse_canonical("018F0F5A-75C0-7B27-8F77-68E8F5E9274F").is_err());
    assert!(EntityId::parse_canonical("f81d4fae-7dec-11d0-a765-00a0c91e6bf6").is_err());
    assert!(EntityId::parse_canonical("550e8400-e29b-41d4-a716-446655440000").is_err());
    assert!(EntityId::from_bytes([0; 16]).is_err());

    let digest = Digest::raw(b"abc");
    assert_eq!(Digest::from_hex(&digest.to_hex()).unwrap(), digest);
    assert!(Digest::from_hex(&digest.to_hex().to_uppercase()).is_err());

    let encoded_id = serde_json::to_string(&id).unwrap();
    assert_eq!(encoded_id, r#""018f0f5a-75c0-7b27-8f77-68e8f5e9274f""#);
    assert!(serde_json::from_str::<EntityId>(r#""018F0F5A-75C0-7B27-8F77-68E8F5E9274F""#).is_err());
    assert!(serde_json::from_str::<Digest>(&format!(r#""{}""#, digest.to_hex())).is_ok());
    assert!(
        serde_json::from_str::<Digest>(&format!(r#""{}""#, digest.to_hex().to_uppercase()))
            .is_err()
    );
}

#[test]
fn ce_10_domain_separation_prevents_cross_kind_digest_reuse() {
    let value = parse_canonical_json(br#"{"title":"same"}"#).unwrap();
    assert_ne!(
        entity_version_digest(&value).unwrap(),
        relation_version_digest(&value).unwrap()
    );
}

#[test]
fn ce_11_content_object_digest_uses_raw_bytes() {
    let raw = br#"{"b":2,"a":1}"#;
    let canonical = br#"{"a":1,"b":2}"#;
    assert_ne!(content_object_digest(raw), content_object_digest(canonical));
    assert_eq!(content_object_digest(raw), Digest::raw(raw));
}

#[test]
fn ce_12_immutable_import_requires_canonical_fixed_point() {
    let value = parse_canonical_json(br#"{"a":1,"b":2}"#).unwrap();
    let digest = entity_version_digest(&value).unwrap();
    assert!(
        validate_import_fixed_point(
            br#"{"a":1,"b":2}"#,
            digest,
            ImportDigestDomain::EntityVersion
        )
        .is_ok()
    );
    assert!(
        validate_import_fixed_point(
            br#"{"b":2,"a":1}"#,
            digest,
            ImportDigestDomain::EntityVersion
        )
        .is_err()
    );
}

#[test]
fn ce_13_work_state_mapping_digest_is_insertion_order_independent() {
    let e1 = uuid([1; 16]);
    let e2 = uuid([2; 16]);
    let ev1 = entity_version([11; 16]);
    let ev2 = entity_version([22; 16]);
    let left = WorkState::new([(e2, ev2), (e1, ev1)], []).unwrap();
    let right = WorkState::new([(e1, ev1), (e2, ev2)], []).unwrap();
    assert_eq!(
        work_state_mapping_digest(&left),
        work_state_mapping_digest(&right)
    );
}

#[test]
fn ce_14_work_state_mapping_digest_separates_entity_and_relation_sections() {
    let object = [3; 16];
    let version = [4; 16];
    let as_entity = WorkState::new([(uuid(object), entity_version(version))], []).unwrap();
    let as_relation = WorkState::new([], [(relation(object), relation_version(version))]).unwrap();
    assert_ne!(
        work_state_mapping_digest(&as_entity),
        work_state_mapping_digest(&as_relation)
    );
}

#[test]
fn ce_13_work_state_mapping_rejects_duplicate_subject_keys() {
    let e1 = uuid([1; 16]);
    let ev1 = entity_version([11; 16]);
    let ev2 = entity_version([22; 16]);
    assert!(WorkState::new([(e1, ev1), (e1, ev2)], []).is_err());
}

proptest! {
    #[test]
    fn prop_object_insertion_order_is_canonical(a in any::<i16>(), b in any::<i16>()) {
        let a = (a as i64).clamp(-1000, 1000);
        let b = (b as i64).clamp(-1000, 1000);
        let left = CanonicalValue::object(vec![
            ("b".to_owned(), CanonicalValue::safe_integer(b).unwrap()),
            ("a".to_owned(), CanonicalValue::safe_integer(a).unwrap()),
        ]).unwrap();
        let right = CanonicalValue::object(vec![
            ("a".to_owned(), CanonicalValue::safe_integer(a).unwrap()),
            ("b".to_owned(), CanonicalValue::safe_integer(b).unwrap()),
        ]).unwrap();

        prop_assert_eq!(canonical_bytes(&left).unwrap(), canonical_bytes(&right).unwrap());
        prop_assert_eq!(entity_version_digest(&left).unwrap(), entity_version_digest(&right).unwrap());
    }
}
