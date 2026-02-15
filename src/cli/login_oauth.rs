//! OAuth transport helpers for `soul login`.

use anyhow::{bail, Context, Result};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::Command;
use std::sync::mpsc;
use std::time::Duration;

pub(crate) const CALLBACK_TIMEOUT: Duration = Duration::from_secs(180);

#[derive(Debug)]
pub(crate) struct CallbackPayload {
    pub(crate) code: String,
    pub(crate) state: String,
}

pub(crate) fn open_browser(url: &str) -> Result<()> {
    #[cfg(target_os = "macos")]
    let opener = ("open", vec![url]);

    #[cfg(target_os = "linux")]
    let opener = ("xdg-open", vec![url]);

    #[cfg(target_os = "windows")]
    let opener = ("cmd", vec!["/C", "start", url]);

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    let opener = ("", Vec::new());

    if opener.0.is_empty() {
        bail!(
            "Automatic browser opening is not supported on this OS. Open this URL manually: {url}"
        );
    }

    let status = Command::new(opener.0)
        .args(opener.1)
        .status()
        .with_context(|| format!("Failed to launch browser opener command for URL: {url}"))?;

    if !status.success() {
        bail!("Failed to open browser automatically. Open this URL manually: {url}");
    }

    Ok(())
}

pub(crate) fn percent_encode(raw: &str) -> String {
    let mut out = String::new();
    for b in raw.bytes() {
        let is_unreserved =
            matches!(b, b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~');
        if is_unreserved {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

pub(crate) fn spawn_callback_listener() -> Result<(u16, mpsc::Receiver<Result<CallbackPayload>>)> {
    let listener =
        TcpListener::bind("127.0.0.1:0").context("Failed to bind local OAuth callback server")?;
    let port = listener.local_addr()?.port();
    let (tx, rx) = mpsc::channel();

    std::thread::spawn(move || {
        let result = receive_callback(listener);
        let _ = tx.send(result);
    });

    Ok((port, rx))
}

fn receive_callback(listener: TcpListener) -> Result<CallbackPayload> {
    let (mut stream, _) = listener
        .accept()
        .context("Did not receive OAuth callback request")?;
    stream
        .set_read_timeout(Some(CALLBACK_TIMEOUT))
        .context("Failed to set callback socket timeout")?;

    let mut buffer = [0_u8; 8192];
    let bytes_read = stream.read(&mut buffer)?;
    if bytes_read == 0 {
        bail!("OAuth callback request was empty");
    }

    let request = String::from_utf8_lossy(&buffer[..bytes_read]);
    let request_line = request.lines().next().unwrap_or_default();
    let target = request_line.split_whitespace().nth(1).unwrap_or("/");

    let query = target.split_once('?').map(|(_, q)| q).unwrap_or_default();

    let mut code = None;
    let mut state = None;
    let mut error = None;

    for pair in query.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        let key = percent_decode(k);
        let value = percent_decode(v);

        match key.as_str() {
            "code" => code = Some(value),
            "state" => state = Some(value),
            "error" => error = Some(value),
            _ => {}
        }
    }

    let response_body = if let Some(err) = &error {
        format!("OAuth failed: {err}. You can close this tab.")
    } else {
        "Soul Vault login complete. You can close this tab.".to_string()
    };
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        response_body.len(),
        response_body
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();

    if let Some(err) = error {
        bail!("OAuth provider returned an error: {err}");
    }

    let code = code.ok_or_else(|| anyhow::anyhow!("OAuth callback missing code parameter"))?;
    let state = state.ok_or_else(|| anyhow::anyhow!("OAuth callback missing state parameter"))?;

    Ok(CallbackPayload { code, state })
}

fn percent_decode(raw: &str) -> String {
    let bytes = raw.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;

    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                let h1 = bytes[i + 1];
                let h2 = bytes[i + 2];
                if let (Some(a), Some(b)) = (hex_val(h1), hex_val(h2)) {
                    out.push((a << 4) | b);
                    i += 3;
                } else {
                    out.push(bytes[i]);
                    i += 1;
                }
            }
            other => {
                out.push(other);
                i += 1;
            }
        }
    }

    String::from_utf8_lossy(&out).to_string()
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(10 + b - b'a'),
        b'A'..=b'F' => Some(10 + b - b'A'),
        _ => None,
    }
}
