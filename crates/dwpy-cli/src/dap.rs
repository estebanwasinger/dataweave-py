use dwpy_core::dap::DebugAdapter;
use serde_json::Value;
use std::{
    io::{self, BufRead, Write},
    sync::mpsc,
    time::Duration,
};

// Read input independently so cooperative execution can continue while waiting
// for the client's next request. The session itself stays on this thread.
pub fn run() -> Result<(), String> {
    let (send, receive) = mpsc::channel();
    std::thread::spawn(move || {
        let stdin = io::stdin();
        let mut input = stdin.lock();
        loop {
            match read_message(&mut input) {
                Ok(Some(message)) => {
                    if send.send(Ok(message)).is_err() {
                        break;
                    }
                }
                Ok(None) => break,
                Err(error) => {
                    let _ = send.send(Err(error));
                    break;
                }
            }
        }
    });
    let mut adapter = DebugAdapter::new();
    let stdout = io::stdout();
    let mut output = stdout.lock();
    loop {
        let incoming = if adapter.is_running() {
            match receive.recv_timeout(Duration::from_millis(1)) {
                Ok(v) => Some(v),
                Err(mpsc::RecvTimeoutError::Timeout) => None,
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        } else {
            match receive.recv() {
                Ok(v) => Some(v),
                Err(_) => break,
            }
        };
        if let Some(request) = incoming {
            let mut request = request?;
            if request["command"] == "launch" && request["arguments"]["source"].is_null() {
                if let Some(path) = request["arguments"]["program"].as_str() {
                    match std::fs::read_to_string(path) {
                        Ok(source) => request["arguments"]["source"] = Value::String(source),
                        Err(error) => {
                            // Let the common adapter produce a structured launch error.
                            eprintln!("Could not read debug program {path}: {error}");
                        }
                    }
                }
            }
            for message in adapter.dispatch(request) {
                write_message(&mut output, &message)?;
            }
        }
        for message in adapter.poll(200) {
            write_message(&mut output, &message)?;
        }
    }
    Ok(())
}
fn read_message(reader: &mut impl BufRead) -> Result<Option<Value>, String> {
    let mut length = None;
    let mut headers = 0;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).map_err(|e| e.to_string())? == 0 {
            return Ok(None);
        }
        headers += line.len();
        if headers > 8192 {
            return Err("DAP headers exceed 8 KiB".into());
        }
        if line == "\r\n" || line == "\n" {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            if name.eq_ignore_ascii_case("Content-Length") {
                length = Some(
                    value
                        .trim()
                        .parse::<usize>()
                        .map_err(|_| "Invalid Content-Length")?,
                );
            }
        }
    }
    let length = length.ok_or("Missing Content-Length")?;
    if length > 64 * 1024 * 1024 {
        return Err("DAP request exceeds 64 MiB".into());
    }
    let mut bytes = vec![0; length];
    reader.read_exact(&mut bytes).map_err(|e| e.to_string())?;
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|e| e.to_string())
}
fn write_message(writer: &mut impl Write, message: &Value) -> Result<(), String> {
    let bytes = serde_json::to_vec(message).map_err(|e| e.to_string())?;
    write!(writer, "Content-Length: {}\r\n\r\n", bytes.len()).map_err(|e| e.to_string())?;
    writer.write_all(&bytes).map_err(|e| e.to_string())?;
    writer.flush().map_err(|e| e.to_string())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn utf8_framing() {
        let message = serde_json::json!({"seq":1,"type":"request","command":"evaluate","arguments":{"expression":"\"🐈\""}});
        let mut bytes = vec![];
        write_message(&mut bytes, &message).unwrap();
        assert_eq!(
            read_message(&mut io::Cursor::new(bytes)).unwrap(),
            Some(message)
        );
    }
    #[test]
    fn rejects_bad_length() {
        assert!(read_message(&mut io::Cursor::new(b"Content-Length: -1\r\n\r\n")).is_err());
    }
}
