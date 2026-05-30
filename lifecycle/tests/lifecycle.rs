use hiero_did_core::DIDError;
use hiero_did_core::Signer;
use hiero_did_lifecycle::LifecycleBuilder;
use hiero_did_lifecycle::LifecycleMessage;
use hiero_did_lifecycle::LifecycleRunner;
use hiero_did_lifecycle::LifecycleRunnerOptions;
use hiero_did_lifecycle::LifecycleStepKind;
use hiero_did_lifecycle::RunnerStatus;
use std::sync::Arc;
use std::sync::Mutex;

#[derive(Debug, Clone, Default)]
struct TestMessage {
    bytes: Vec<u8>,
    signature: Option<Vec<u8>>,
    events: Vec<String>,
}

impl LifecycleMessage for TestMessage {
    fn message_bytes(&self) -> Result<Vec<u8>, DIDError> {
        Ok(self.bytes.clone())
    }

    fn set_signature(&mut self, signature: Vec<u8>) -> Result<(), DIDError> {
        self.signature = Some(signature);
        self.events.push("signature".to_string());
        Ok(())
    }
}

struct TestSigner;

impl Signer for TestSigner {
    fn public_key_bytes(&self) -> Vec<u8> {
        vec![9; 32]
    }

    fn sign_bytes(&self, message: &[u8]) -> Result<Vec<u8>, DIDError> {
        let mut signature = b"signed:".to_vec();
        signature.extend_from_slice(message);
        Ok(signature)
    }
}

#[test]
fn builder_rejects_duplicate_step_labels() {
    let result = LifecycleBuilder::<TestMessage>::new()
        .pause("same")
        .expect("first step")
        .sign_with_signer("same");

    assert!(matches!(result, Err(DIDError::InvalidArgument(_))));
}

#[test]
fn builder_indexes_steps_by_label() {
    let builder = LifecycleBuilder::<TestMessage>::new()
        .pause("pause")
        .expect("pause")
        .attach_signature("signature")
        .expect("signature");

    assert_eq!(builder.len(), 2);
    assert_eq!(builder.get_index_by_label("signature"), Some(1));
    assert_eq!(
        builder.get_by_index(0).map(|step| step.kind()),
        Some(&LifecycleStepKind::Pause)
    );
}

#[tokio::test]
async fn sign_step_uses_signer_and_sets_signature() {
    let builder = LifecycleBuilder::<TestMessage>::new()
        .sign_with_signer("sign")
        .expect("sign");
    let runner = LifecycleRunner::new(builder);
    let signer = TestSigner;
    let mut options = LifecycleRunnerOptions::new(());
    options.signer = Some(&signer);

    let state = runner
        .process(
            TestMessage {
                bytes: b"abc".to_vec(),
                ..Default::default()
            },
            options,
        )
        .await
        .expect("process");

    assert_eq!(state.status, RunnerStatus::Success);
    assert_eq!(state.message.signature, Some(b"signed:abc".to_vec()));
}

#[tokio::test]
async fn external_signature_flow_pauses_and_resumes() {
    let builder = LifecycleBuilder::<TestMessage>::new()
        .pause("pause-for-signature")
        .expect("pause")
        .attach_signature("signature")
        .expect("signature")
        .pause("pause-before-publish")
        .expect("pause before publish")
        .callback("publish", |message, _context| {
            Box::pin(async move {
                message.events.push("published".to_string());
                Ok(())
            })
        })
        .expect("publish");
    let runner = LifecycleRunner::new(builder);

    let first = runner
        .process(
            TestMessage {
                bytes: b"abc".to_vec(),
                ..Default::default()
            },
            LifecycleRunnerOptions::new(()),
        )
        .await
        .expect("first process");

    assert_eq!(first.status, RunnerStatus::Pause);
    assert_eq!(first.label, "pause-for-signature");

    let mut signed_options = LifecycleRunnerOptions::new(());
    signed_options.signature = Some(vec![1, 2, 3]);
    let second = runner
        .resume(first, signed_options)
        .await
        .expect("resume with signature");

    assert_eq!(second.status, RunnerStatus::Pause);
    assert_eq!(second.label, "pause-before-publish");
    assert_eq!(second.message.signature, Some(vec![1, 2, 3]));

    let final_state = runner
        .resume(second, LifecycleRunnerOptions::new(()))
        .await
        .expect("publish resume");

    assert_eq!(final_state.status, RunnerStatus::Success);
    assert_eq!(
        final_state.message.events.last(),
        Some(&"published".to_string())
    );
}

#[tokio::test]
async fn hooks_for_pause_run_after_resume() {
    let builder = LifecycleBuilder::<TestMessage>::new()
        .pause("pause")
        .expect("pause")
        .callback("next", |message, _context| {
            Box::pin(async move {
                message.events.push("next".to_string());
                Ok(())
            })
        })
        .expect("next");
    let mut runner = LifecycleRunner::new(builder);
    runner.on_complete("pause", |message| {
        Box::pin(async move {
            assert!(message.events.is_empty());
            Ok(())
        })
    });

    let paused = runner
        .process(TestMessage::default(), LifecycleRunnerOptions::new(()))
        .await
        .expect("pause");
    let state = runner
        .resume(paused, LifecycleRunnerOptions::new(()))
        .await
        .expect("resume");

    assert_eq!(state.status, RunnerStatus::Success);
    assert_eq!(state.message.events, vec!["next"]);
}

#[tokio::test]
async fn catch_handler_converts_error_state() {
    let caught = Arc::new(Mutex::new(false));
    let caught_in_handler = Arc::clone(&caught);
    let builder = LifecycleBuilder::<TestMessage>::new()
        .callback("fail", |_message, _context| {
            Box::pin(async { Err(DIDError::InvalidArgument("bad step".to_string())) })
        })
        .expect("fail")
        .catch("catch", move |_error| {
            let caught_in_handler = Arc::clone(&caught_in_handler);
            Box::pin(async move {
                *caught_in_handler.lock().expect("lock") = true;
                Ok(())
            })
        });
    let runner = LifecycleRunner::new(builder);
    let state = runner
        .process(TestMessage::default(), LifecycleRunnerOptions::new(()))
        .await
        .expect("catch");

    assert_eq!(state.status, RunnerStatus::Error);
    assert!(state.error.expect("error").contains("bad step"));
    assert!(*caught.lock().expect("lock"));
}
