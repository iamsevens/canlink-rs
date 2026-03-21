use canlink_hal::CanResult;
use canlink_tscan::daemon::client::{DaemonClient, InitParams};
use canlink_tscan::daemon::{ConnectResult, Op};
use canlink_tscan::TscanDaemonConfig;
use std::env;
use std::time::Instant;

fn parse_args() -> (String, usize) {
    let mut args = env::args().skip(1);
    let mode = args.next().unwrap_or_else(|| "by_handle".to_string());
    let iterations = args
        .next()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(1000);
    (mode, iterations)
}

fn main() -> CanResult<()> {
    let (mode, iterations) = parse_args();
    if mode != "by_handle" && mode != "disconnect_all" {
        eprintln!("usage: disconnect_client_stress <by_handle|disconnect_all> [iterations]");
        std::process::exit(2);
    }

    let mut client = DaemonClient::connect(
        &TscanDaemonConfig::default(),
        InitParams {
            enable_fifo: true,
            enable_error_frame: false,
            use_hw_time: false,
        },
    )?;

    let start_all = Instant::now();
    for i in 1..=iterations {
        let t0 = Instant::now();
        let connect = client.request_auto(Op::Connect {
            serial: String::new(),
        })?;
        let info: ConnectResult = connect.decode_data()?;
        if mode == "by_handle" {
            client.request_auto(Op::DisconnectByHandle {
                handle: info.handle,
            })?;
        } else {
            client.request_auto(Op::DisconnectAll)?;
        }
        let elapsed_ms = t0.elapsed().as_secs_f64() * 1000.0;
        if i == 1 || i % 50 == 0 {
            println!("iter {}/{} ok ({:.1} ms)", i, iterations, elapsed_ms);
        }
    }

    let total = start_all.elapsed().as_secs_f64();
    println!(
        "done: mode={}, iterations={}, elapsed={:.2}s",
        mode, iterations, total
    );
    Ok(())
}
