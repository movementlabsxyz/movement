use tokio::process::Command;
use std::process::Stdio;
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};

async fn pipe_output<R: tokio::io::AsyncRead + Unpin + Send + 'static>(reader: R, mut writer: io::Stdout) {
    let mut reader = BufReader::new(reader).lines();
    while let Ok(Some(line)) = reader.next_line().await {
        if let Err(e) = writer.write_all(line.as_bytes()).await {
            eprintln!("Failed to write to stdout: {}", e);
            break;
        }
        if let Err(e) = writer.write_all(b"\n").await {
            eprintln!("Failed to write newline to stdout: {}", e);
            break;
        }
    }
}

async fn pipe_error_output<R: tokio::io::AsyncRead + Unpin + Send + 'static>(reader: R, mut writer: io::Stderr) {
    let mut reader = BufReader::new(reader).lines();
    while let Ok(Some(line)) = reader.next_line().await {
        if let Err(e) = writer.write_all(line.as_bytes()).await {
            eprintln!("Failed to write to stderr: {}", e);
            break;
        }
        if let Err(e) = writer.write_all(b"\n").await {
            eprintln!("Failed to write newline to stderr: {}", e);
            break;
        }
    }
}

pub async fn run_command(command: &str, args: &[&str]) -> Result<(), anyhow::Error> {
    let mut child = Command::new(command)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let stdout = child.stdout.take().ok_or(
        anyhow::anyhow!("Failed to capture standard output from command {}", command)
    )?;
    let stderr = child.stderr.take().ok_or(
        anyhow::anyhow!("Failed to capture standard error from command {}", command)
    )?;

    let stdout_writer = io::stdout();
    let stderr_writer = io::stderr();

    tokio::spawn(pipe_output(stdout, stdout_writer));
    tokio::spawn(pipe_error_output(stderr, stderr_writer));

    let status = child.wait().await?;
    if !status.success() {
        return Err(anyhow::anyhow!("Command {} failed with args {:?}", command, args));
    }
    Ok(())
}
