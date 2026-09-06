use rmcp::handler::server::wrapper::Parameters;
use sequentialthinking_rs::counter::{CounterInput, CounterOutput, CounterResponse, CounterState};
use sequentialthinking_rs::sink::NoopThoughtSink;
use sequentialthinking_rs::thinking::SequentialThinkingState;
use sequentialthinking_rs::SequentialThinkingServer;

#[test]
fn test_counter_initialization_and_stepping() {
    let mut state = CounterState::with_sink(Box::new(NoopThoughtSink));

    // Initialize counter with total: 5
    let init = CounterInput {
        total: Some(5),
        ..Default::default()
    };
    let res1 = state.process(init).unwrap();
    match res1 {
        CounterOutput::Progress(r) => {
            assert_eq!(r.name, "default");
            assert_eq!(r.current, 1);
            assert_eq!(r.total, Some(5));
            assert_eq!(r.remaining, Some(4));
            assert!(!r.done);
        }
        CounterOutput::Finished(_) => panic!("expected Progress"),
    }

    // Step 2
    let res2 = state.process(CounterInput::default()).unwrap();
    match res2 {
        CounterOutput::Progress(r) => {
            assert_eq!(r.current, 2);
            assert_eq!(r.remaining, Some(3));
            assert!(!r.done);
        }
        CounterOutput::Finished(_) => panic!("expected Progress"),
    }

    // Step 3
    let res3 = state.process(CounterInput::default()).unwrap();
    match res3 {
        CounterOutput::Progress(r) => {
            assert_eq!(r.current, 3);
            assert_eq!(r.remaining, Some(2));
            assert!(!r.done);
        }
        CounterOutput::Finished(_) => panic!("expected Progress"),
    }

    // Step 4
    let res4 = state.process(CounterInput::default()).unwrap();
    match res4 {
        CounterOutput::Progress(r) => {
            assert_eq!(r.current, 4);
            assert_eq!(r.remaining, Some(1));
            assert!(!r.done);
        }
        CounterOutput::Finished(_) => panic!("expected Progress"),
    }

    // Step 5 (reaches total)
    let res5 = state.process(CounterInput::default()).unwrap();
    match res5 {
        CounterOutput::Progress(r) => {
            assert_eq!(r.current, 5);
            assert_eq!(r.remaining, Some(0));
            assert!(r.done);
        }
        CounterOutput::Finished(_) => panic!("expected Progress"),
    }

    // Step 6 (already finished)
    let res6 = state.process(CounterInput::default()).unwrap();
    match res6 {
        CounterOutput::Finished(msg) => {
            assert_eq!(msg, "Your counter has finished, move on.");
        }
        CounterOutput::Progress(_) => panic!("expected Finished message"),
    }
}

#[test]
fn test_counter_status_query_and_custom_step() {
    let mut state = CounterState::with_sink(Box::new(NoopThoughtSink));

    // Initialize
    state
        .process(CounterInput {
            total: Some(10),
            ..Default::default()
        })
        .unwrap();

    // Query status without stepping (step: 0)
    let query_res = state
        .process(CounterInput {
            step: Some(0),
            ..Default::default()
        })
        .unwrap();
    match query_res {
        CounterOutput::Progress(r) => {
            assert_eq!(r.current, 1);
            assert_eq!(r.remaining, Some(9));
            assert!(!r.done);
        }
        CounterOutput::Finished(_) => panic!("expected Progress"),
    }

    // Custom step increment (+3)
    let step_res = state
        .process(CounterInput {
            step: Some(3),
            ..Default::default()
        })
        .unwrap();
    match step_res {
        CounterOutput::Progress(r) => {
            assert_eq!(r.current, 4);
            assert_eq!(r.remaining, Some(6));
            assert!(!r.done);
        }
        CounterOutput::Finished(_) => panic!("expected Progress"),
    }

    // Explicit current override
    let override_res = state
        .process(CounterInput {
            current: Some(9),
            ..Default::default()
        })
        .unwrap();
    match override_res {
        CounterOutput::Progress(r) => {
            assert_eq!(r.current, 9);
            assert_eq!(r.remaining, Some(1));
            assert!(!r.done);
        }
        CounterOutput::Finished(_) => panic!("expected Progress"),
    }
}

#[test]
fn test_multi_counter_isolation() {
    let mut state = CounterState::with_sink(Box::new(NoopThoughtSink));

    // Named counter "outer"
    state
        .process(CounterInput {
            name: Some("outer".to_string()),
            total: Some(2),
            ..Default::default()
        })
        .unwrap();

    // Named counter "inner"
    state
        .process(CounterInput {
            name: Some("inner".to_string()),
            total: Some(5),
            ..Default::default()
        })
        .unwrap();

    // Step "outer" to completion
    let outer_step = state
        .process(CounterInput {
            name: Some("outer".to_string()),
            ..Default::default()
        })
        .unwrap();
    match outer_step {
        CounterOutput::Progress(r) => {
            assert_eq!(r.name, "outer");
            assert_eq!(r.current, 2);
            assert!(r.done);
        }
        CounterOutput::Finished(_) => panic!("expected Progress"),
    }

    // Verify "inner" was unaffected
    let inner_check = state
        .process(CounterInput {
            name: Some("inner".to_string()),
            step: Some(0),
            ..Default::default()
        })
        .unwrap();
    match inner_check {
        CounterOutput::Progress(r) => {
            assert_eq!(r.name, "inner");
            assert_eq!(r.current, 1);
            assert_eq!(r.remaining, Some(4));
            assert!(!r.done);
        }
        CounterOutput::Finished(_) => panic!("expected Progress"),
    }

    assert_eq!(state.counters().len(), 2);
}

#[test]
fn test_counter_reset_and_extension() {
    let mut state = CounterState::with_sink(Box::new(NoopThoughtSink));

    // Initialize with 1 item and complete it
    state
        .process(CounterInput {
            total: Some(1),
            ..Default::default()
        })
        .unwrap();

    // Already finished call
    let fin = state.process(CounterInput::default()).unwrap();
    assert!(matches!(fin, CounterOutput::Finished(_)));

    // Extend total to 3 while stepping (default step: 1) -> advances current from 1 to 2
    let ext = state
        .process(CounterInput {
            total: Some(3),
            ..Default::default()
        })
        .unwrap();
    match ext {
        CounterOutput::Progress(r) => {
            assert_eq!(r.current, 2);
            assert_eq!(r.total, Some(3));
            assert_eq!(r.remaining, Some(1));
            assert!(!r.done);
        }
        CounterOutput::Finished(_) => panic!("expected Progress"),
    }

    // Reset counter
    let reset = state
        .process(CounterInput {
            reset: Some(true),
            total: Some(4),
            ..Default::default()
        })
        .unwrap();
    match reset {
        CounterOutput::Progress(r) => {
            assert_eq!(r.current, 1);
            assert_eq!(r.total, Some(4));
            assert_eq!(r.remaining, Some(3));
            assert!(!r.done);
        }
        CounterOutput::Finished(_) => panic!("expected Progress"),
    }
}

#[tokio::test]
async fn test_counter_tool_via_server() {
    let thinking_state = SequentialThinkingState::with_sink(Box::new(NoopThoughtSink));
    let counter_state = CounterState::with_sink(Box::new(NoopThoughtSink));
    let server = SequentialThinkingServer::with_states(thinking_state, counter_state);

    // Initial step with total: 2
    let input1 = CounterInput {
        total: Some(2),
        ..Default::default()
    };
    let result1 = server.counter(Parameters(input1)).await.unwrap();
    assert_eq!(result1.is_error, Some(false));
    let text1 = result1.content[0].as_text().unwrap();
    let resp1: CounterResponse = serde_json::from_str(&text1.text).unwrap();
    assert_eq!(resp1.name, "default");
    assert_eq!(resp1.current, 1);
    assert_eq!(resp1.total, Some(2));
    assert_eq!(resp1.remaining, Some(1));
    assert!(!resp1.done);

    // Step 2
    let result2 = server
        .counter(Parameters(CounterInput::default()))
        .await
        .unwrap();
    assert_eq!(result2.is_error, Some(false));
    let text2 = result2.content[0].as_text().unwrap();
    let resp2: CounterResponse = serde_json::from_str(&text2.text).unwrap();
    assert_eq!(resp2.current, 2);
    assert_eq!(resp2.remaining, Some(0));
    assert!(resp2.done);

    // Step 3 (finished notice)
    let result3 = server
        .counter(Parameters(CounterInput::default()))
        .await
        .unwrap();
    assert_eq!(result3.is_error, Some(false));
    let text3 = result3.content[0].as_text().unwrap();
    assert_eq!(text3.text, "Your counter has finished, move on.");
}
