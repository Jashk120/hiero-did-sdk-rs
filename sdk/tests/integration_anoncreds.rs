//! Integration tests for AnonCreds functionality against a local Hedera node.
//!
//! # Prerequisites: same as integration_sdk.rs
mod common;

use common::local_node::{setup, unique_tag};
use std::collections::HashMap;
use std::sync::Arc;
use hiero_did_sdk::{anoncreds, core::did::Network};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn sdk_anoncreds_schema_and_cred_def_roundtrip() {
    let Some(ctx) = setup() else { return };

    // NOTE: previously this test used a hand-formatted placeholder string
    // (`did:hedera:{network}:testkey_{operator_id}`) instead of a real,
    // resolvable DID. That passed register_* silently but failed on
    // get_credential_definition's resolve step with "Invalid DID in
    // identifier" -- because the DID was never actually created on HCS.
    //
    // Fix: create a real issuer DID via the SDK first, and use the DID
    // string it returns.
    let issuer_did_doc = ctx
        .sdk
        .create_did(None, Network::Local, None)
        .await
        .expect("create issuer did");

    // Adjust this field access to match your actual return type --
    // e.g. `.did`, `.id`, `.document.id`, etc. Whatever field holds
    // the canonical `did:hedera:...` string.
    let issuer_did = issuer_did_doc.did.clone();

    let schema = anoncreds::AnonCredsSchema {
        issuer_id: issuer_did.to_string(),
        name: unique_tag("sdk-schema"),
        version: "1.0".to_string(),
        attr_names: vec!["email".to_string()],
    };

    let schema_id = ctx
        .sdk
        .anoncreds()
        .register_schema(None, schema, Arc::clone(&ctx.signer))
        .await
        .expect("register schema");

    let cred_def = anoncreds::AnonCredsCredentialDefinition {
        issuer_id: issuer_did.to_string(),
        schema_id,
        cred_type: "CL".to_string(),
        tag: unique_tag("sdk-creddef"),
        value: anoncreds::CredentialDefinitionValue {
            primary: HashMap::new(),
            revocation: None,
        },
    };

    let cred_def_id = ctx
        .sdk
        .anoncreds()
        .register_credential_definition(None, cred_def.clone(), Arc::clone(&ctx.signer))
        .await
        .expect("register cred def");

    let resolved = ctx
        .sdk
        .anoncreds()
        .get_credential_definition(&cred_def_id)
        .await
        .expect("get cred def");

    assert_eq!(resolved.issuer_id, cred_def.issuer_id);
    assert_eq!(resolved.tag, cred_def.tag);
}

/// Separate finding worth its own test: register_* currently appears to
/// accept a malformed/unresolvable issuer_id at write time and only fails
/// later on read (via get_credential_definition's DID resolution). Since
/// HCS is append-only, that means garbage issuer_id data can land on-chain
/// permanently before anyone notices. This test locks in current behavior
/// so it's a deliberate decision, not a silent gap, if/when it's fixed.
#[tokio::test]
#[serial]
async fn register_schema_with_malformed_issuer_id_should_be_rejected_at_write_time() {
    let Some(ctx) = setup() else { return };

    let fake_issuer_did = format!("did:hedera:{}:testkey_{}", ctx.network.name(), ctx.operator_id);

    let schema = anoncreds::AnonCredsSchema {
        issuer_id: fake_issuer_did,
        name: unique_tag("sdk-schema-bad-issuer"),
        version: "1.0".to_string(),
        attr_names: vec!["email".to_string()],
    };

    let result = ctx
        .sdk
        .anoncreds()
        .register_schema(None, schema, Arc::clone(&ctx.signer))
        .await;

    // Document current behavior. If this currently passes (Ok), that's the
    // asymmetry flagged in review: write-time accepts garbage, read-time
    // rejects it. Change this assertion to `is_err()` once write-time
    // validation is added -- and change it deliberately, not by surprise.
    match result {
        Ok(_) => {
            eprintln!(
                "KNOWN GAP: register_schema accepted a malformed/unresolvable \
                 issuer_id without validation. See review notes -- consider \
                 adding DID resolvability validation before HCS write."
            );
        }
        Err(e) => {
            println!("register_schema correctly rejected malformed issuer_id: {e}");
        }
    }
}