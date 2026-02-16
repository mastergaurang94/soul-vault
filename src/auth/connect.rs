//! OAuth connect flow shared by CLI and TUI.

use anyhow::{bail, Context, Result};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::Command;
use std::sync::mpsc;
use std::time::Duration;

use crate::auth::{exchange_code_for_token, oauth_config, save_credentials};
use crate::types::Provider;

const CALLBACK_TIMEOUT: Duration = Duration::from_secs(180);

pub async fn connect_provider(provider: &Provider) -> Result<()> {
    match provider {
        Provider::Claude => connect_claude().await,
        Provider::ChatGpt | Provider::Gemini => bail!(
            "OAuth for {} is coming soon. Use local import for now.",
            provider.display_name()
        ),
    }
}

async fn connect_claude() -> Result<()> {
    let oauth = oauth_config(&Provider::Claude);
    let state = format!("soul-{}", chrono::Utc::now().timestamp_millis());
    let (port, callback_rx) = spawn_callback_listener()?;
    let redirect_uri = format!("http://127.0.0.1:{port}/oauth/callback");

    let auth_url = format!(
        "{}?response_type=code&client_id={}&redirect_uri={}&scope={}&state={}",
        oauth.auth_url,
        percent_encode(&oauth.client_id),
        percent_encode(&redirect_uri),
        percent_encode(&oauth.scope),
        percent_encode(&state)
    );

    open_browser(&auth_url)?;
    let callback = callback_rx
        .recv_timeout(CALLBACK_TIMEOUT)
        .context("Timed out waiting for OAuth callback. Try connecting again.")??;

    if callback.state != state {
        bail!("OAuth state mismatch. Please retry.");
    }

    let credentials = exchange_code_for_token(&oauth, &callback.code, &redirect_uri).await?;
    save_credentials(&credentials)?;
    Ok(())
}

#[derive(Debug)]
struct CallbackPayload {
    code: String,
    state: String,
}

fn open_browser(url: &str) -> Result<()> {
    #[cfg(target_os = "macos")]
    let opener = ("open", vec![url]);
    #[cfg(target_os = "linux")]
    let opener = ("xdg-open", vec![url]);
    #[cfg(target_os = "windows")]
    let opener = ("cmd", vec!["/C", "start", url]);
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    let opener = ("", Vec::new());

    if opener.0.is_empty() {
        bail!("Automatic browser opening is not supported on this OS.");
    }

    let status = Command::new(opener.0)
        .args(opener.1)
        .status()
        .with_context(|| format!("Failed to launch browser for URL: {url}"))?;

    if !status.success() {
        bail!("Failed to open browser automatically.");
    }
    Ok(())
}

fn percent_encode(raw: &str) -> String {
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

fn spawn_callback_listener() -> Result<(u16, mpsc::Receiver<Result<CallbackPayload>>)> {
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
        "Soul Vault connection complete. You can close this tab.".to_string()
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
                if let (Some(a), Some(b)) = (hex_val(bytes[i + 1]), hex_val(bytes[i + 2])) {
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
