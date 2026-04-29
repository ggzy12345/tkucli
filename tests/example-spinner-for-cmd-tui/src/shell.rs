use tokio::io::AsyncReadExt;
use tku_core::context::Ctx;
use tku_core::prelude::TaskSpinner;

/// Run a shell command and stream its output to the TUI bubble or CLI stdout.
///
/// Use this for commands that return quickly and whose output **is** the result
/// (e.g. `multipass list`, `multipass info`). No spinner is shown.
///
/// - **TUI mode**: each meaningful output line replaces the bubble body in-place.
/// - **CLI mode**: each line is printed directly to stdout as it arrives.
///
/// Returns `(exit_status, full_output)`.
pub async fn run_streaming(
    ctx:     &Ctx,
    command: &str,
) -> std::io::Result<(std::process::ExitStatus, String)> {
    let mut child = tokio::process::Command::new("sh")
        .arg("-c")
        .arg(command)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    let mut stdout = child.stdout.take().unwrap();
    let mut stderr = child.stderr.take().unwrap();

    let (tx, mut rx) = tokio::sync::mpsc::channel::<Vec<u8>>(100);
    let tx_err = tx.clone();

    tokio::spawn(async move {
        let mut buf = [0u8; 256];
        while let Ok(n) = stdout.read(&mut buf).await {
            if n == 0 { break; }
            let _ = tx.send(buf[..n].to_vec()).await;
        }
    });

    tokio::spawn(async move {
        let mut buf = [0u8; 256];
        while let Ok(n) = stderr.read(&mut buf).await {
            if n == 0 { break; }
            let _ = tx_err.send(buf[..n].to_vec()).await;
        }
    });

    let mut full_output = String::new();

    let status = loop {
        tokio::select! {
            Some(bytes) = rx.recv() => {
                let text = String::from_utf8_lossy(&bytes).to_string();
                full_output.push_str(&text);

                for line in text
                    .split(|c| c == '\r' || c == '\n')
                    .filter(|s| !s.trim().is_empty())
                    .filter(|s| s.trim().chars().any(|c| c.is_alphabetic()))
                {
                    if ctx.tui_mode() {
                        ctx.progress.send(line.trim());
                    } else {
                        println!("{}", line.trim());
                    }
                }
            }
            res = child.wait() => {
                break res?;
            }
        }
    };

    Ok((status, full_output.trim().to_string()))
}

/// Run a shell command with a spinner for long-running operations.
///
/// Use this for commands with unpredictable duration where you want an
/// animated spinner with live status updates (e.g. `multipass launch`,
/// `multipass delete`).
///
/// Each meaningful output line updates the spinner message / TUI bubble body.
///
/// Returns `(exit_status, full_output)`.
pub async fn run_with_spinner(
    spinner: &TaskSpinner<'_>,
    command: &str,
) -> std::io::Result<(std::process::ExitStatus, String)> {
    let mut child = tokio::process::Command::new("sh")
        .arg("-c")
        .arg(command)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    let mut stdout = child.stdout.take().unwrap();
    let mut stderr = child.stderr.take().unwrap();

    let (tx, mut rx) = tokio::sync::mpsc::channel::<Vec<u8>>(100);
    let tx_err = tx.clone();

    tokio::spawn(async move {
        let mut buf = [0u8; 256];
        while let Ok(n) = stdout.read(&mut buf).await {
            if n == 0 { break; }
            let _ = tx.send(buf[..n].to_vec()).await;
        }
    });

    tokio::spawn(async move {
        let mut buf = [0u8; 256];
        while let Ok(n) = stderr.read(&mut buf).await {
            if n == 0 { break; }
            let _ = tx_err.send(buf[..n].to_vec()).await;
        }
    });

    let mut full_output = String::new();

    let status = loop {
        tokio::select! {
            Some(bytes) = rx.recv() => {
                let text = String::from_utf8_lossy(&bytes).to_string();
                full_output.push_str(&text);

                if let Some(line) = text
                    .split(|c| c == '\r' || c == '\n')
                    .filter(|s| !s.trim().is_empty())
                    .filter(|s| s.trim().chars().any(|c| c.is_alphabetic()))
                    .last()
                {
                    spinner.update(line.trim());
                }
            }
            res = child.wait() => {
                break res?;
            }
        }
    };

    Ok((status, full_output.trim().to_string()))
}
