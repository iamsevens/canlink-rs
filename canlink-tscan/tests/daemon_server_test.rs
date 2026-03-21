use canlink_tscan::daemon::server::run_server_with_io;
use canlink_tscan::daemon::{read_frame, write_frame, Op, Request, Response, Status};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn unique_trace_file() -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time error")
        .as_nanos();
    std::env::temp_dir().join(format!("canlink-tscan-daemon-server-trace-{nanos}.log"))
}

#[test]
fn server_hello_success() {
    let _guard = ENV_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|err| err.into_inner());

    let mut input = Vec::new();
    write_frame(
        &mut input,
        &Request::new(
            1,
            Op::Hello {
                protocol_version: 1,
                client_version: "test".to_string(),
            },
        ),
    )
    .expect("failed to write hello frame");

    let mut output = Vec::new();
    run_server_with_io(&mut &input[..], &mut output).expect("server run failed");

    let response: Response = read_frame(&mut &output[..]).expect("failed to read response");
    assert_eq!(response.id, 1);
    assert_eq!(response.status, Status::Ok);
}

#[test]
fn server_hello_version_mismatch() {
    let _guard = ENV_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|err| err.into_inner());

    let mut input = Vec::new();
    write_frame(
        &mut input,
        &Request::new(
            9,
            Op::Hello {
                protocol_version: 2,
                client_version: "test".to_string(),
            },
        ),
    )
    .expect("failed to write hello frame");

    let mut output = Vec::new();
    run_server_with_io(&mut &input[..], &mut output).expect("server run failed");

    let response: Response = read_frame(&mut &output[..]).expect("failed to read response");
    assert_eq!(response.id, 9);
    assert_eq!(response.status, Status::Error);
}

#[test]
fn server_delay_injection_traces_once_per_process() {
    let _guard = ENV_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|err| err.into_inner());

    let trace_path = unique_trace_file();
    std::env::set_var("TRACE_PATH", &trace_path);
    std::env::set_var("DELAY_ON_OP_ONCE", "HELLO");
    std::env::set_var("DELAY_MS", "1");
    std::env::remove_var("EXIT_ON_OP_ONCE");

    let mut input = Vec::new();
    for id in [1_u64, 2_u64] {
        write_frame(
            &mut input,
            &Request::new(
                id,
                Op::Hello {
                    protocol_version: 1,
                    client_version: "test".to_string(),
                },
            ),
        )
        .expect("failed to write hello frame");
    }

    let mut output = Vec::new();
    run_server_with_io(&mut &input[..], &mut output).expect("server run failed");

    let response1: Response = read_frame(&mut &output[..]).expect("failed to read first response");
    assert_eq!(response1.id, 1);
    assert_eq!(response1.status, Status::Ok);

    let remaining = {
        let mut slice = &output[..];
        let _ = read_frame::<_, Response>(&mut slice).expect("failed to read first response");
        slice
    };
    let response2: Response =
        read_frame(&mut &remaining[..]).expect("failed to read second response");
    assert_eq!(response2.id, 2);
    assert_eq!(response2.status, Status::Ok);

    std::env::remove_var("TRACE_PATH");
    std::env::remove_var("DELAY_ON_OP_ONCE");
    std::env::remove_var("DELAY_MS");

    let trace = std::fs::read_to_string(trace_path).expect("read trace file failed");
    let hello_count = trace.lines().filter(|line| *line == "OP:HELLO").count();
    let delay_count = trace
        .lines()
        .filter(|line| *line == "INJECT_DELAY_ONCE:HELLO:1")
        .count();

    assert_eq!(hello_count, 2, "unexpected trace:\n{trace}");
    assert_eq!(delay_count, 1, "unexpected trace:\n{trace}");
}
