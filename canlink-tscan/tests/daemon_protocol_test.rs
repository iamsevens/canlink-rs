use canlink_tscan::daemon::{
    read_frame, write_frame, ErrorCode, HelloAck, Op, Request, Response, MAX_FRAME_SIZE,
};

#[test]
fn protocol_hello_ack_roundtrip() {
    let ack = HelloAck {
        protocol_version: 1,
        daemon_version: "daemon-test".to_string(),
    };
    let response = Response::ok_data(1, &ack);
    let json = serde_json::to_string(&response).expect("serialize response failed");
    let parsed: Response = serde_json::from_str(&json).expect("deserialize response failed");
    assert_eq!(parsed, response);
}

#[test]
fn request_roundtrip() {
    let request = Request::new(
        42,
        Op::Hello {
            protocol_version: 1,
            client_version: "client-test".to_string(),
        },
    );
    let json = serde_json::to_string(&request).expect("serialize request failed");
    let parsed: Request = serde_json::from_str(&json).expect("deserialize request failed");
    assert_eq!(parsed, request);
}

#[test]
fn codec_roundtrip() {
    let request = Request::new(
        1,
        Op::Hello {
            protocol_version: 1,
            client_version: "client".to_string(),
        },
    );
    let mut buf = Vec::new();
    write_frame(&mut buf, &request).expect("write frame failed");
    let parsed: Request = read_frame(&mut &buf[..]).expect("read frame failed");
    assert_eq!(parsed, request);
}

#[test]
fn codec_rejects_oversize_read() {
    let mut buf = Vec::new();
    let too_large = (MAX_FRAME_SIZE as u32) + 1;
    buf.extend_from_slice(&too_large.to_le_bytes());
    let err = read_frame::<_, Request>(&mut &buf[..]).expect_err("read should fail");
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
}

#[test]
fn codec_rejects_oversize_write() {
    let big_payload = "a".repeat(MAX_FRAME_SIZE + 1);
    let request = Request::new(
        2,
        Op::Hello {
            protocol_version: 1,
            client_version: big_payload,
        },
    );
    let mut buf = Vec::new();
    let err = write_frame(&mut buf, &request).expect_err("write should fail");
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
}

#[test]
fn error_response_is_marked_error() {
    let response = Response::error(9, ErrorCode::ProtocolError as u32, "invalid op");
    assert!(!response.is_ok());
}
