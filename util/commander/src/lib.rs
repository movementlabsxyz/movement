use anyhow::Result;
use futures::future::try_join;
use std::process::Stdio;
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::task::JoinHandle;
use tokio::signal::unix::{signal, SignalKind};

async fn pipe_output<R: tokio::io::AsyncRead + Unpin + Send + 'static>(
    reader: R,
    mut writer: io::Stdout,
    output: &mut String,
) -> Result<()> {
    let mut reader = BufReader::new(reader).lines();
    while let Ok(Some(line)) = reader.next_line().await {
        writer.write_all(line.as_bytes()).await?;
        writer.write_all(b"\n").await?;
        output.push_str(&line);
        output.push('\n');
    }
    Ok(())
}

async fn pipe_error_output<R: tokio::io::AsyncRead + Unpin + Send + 'static>(
    reader: R,
    mut writer: io::Stderr,
    output: &mut String,
) -> Result<()> {
    let mut reader = BufReader::new(reader).lines();
    while let Ok(Some(line)) = reader.next_line().await {
        writer.write_all(line.as_bytes()).await?;
        writer.write_all(b"\n").await?;
        output.push_str(&line);
        output.push('\n');
    }
    Ok(())
}

/// Runs a command, piping its output to stdout and stderr, and returns the stdout output if successful.
pub async fn run_command(command: &str, args: &[&str]) -> Result<String> {

    // print command out with args joined by space
    tracing::info!("Running command: {} {}", command, args.join(" "));

    // Setup signal handling to terminate the child process
    let (tx, rx) = tokio::sync::oneshot::channel();

    let mut sigterm = signal(SignalKind::terminate())?;
    let mut sigint = signal(SignalKind::interrupt())?;
    let mut sigquit = signal(SignalKind::quit())?;

    tokio::spawn(async move {
        tokio::select! {
            _ = sigterm.recv() => {
                let _ = tx.send(());
            }
            _ = sigint.recv() => {
                let _ = tx.send(());
            }
            _ = sigquit.recv() => {
                let _ = tx.send(());
            }
        }
    });

    let mut child = Command::new(command)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let stdout = child.stdout.take().ok_or_else(|| {
        anyhow::anyhow!("Failed to capture standard output from command {}", command)
    })?;
    let stderr = child.stderr.take().ok_or_else(|| {
        anyhow::anyhow!("Failed to capture standard error from command {}", command)
    })?;

    let mut stdout_output = String::new();
    let mut stderr_output = String::new();

    let stdout_writer = io::stdout();
    let stderr_writer = io::stderr();

    let stdout_future = pipe_output(stdout, stdout_writer, &mut stdout_output);
    let stderr_future = pipe_error_output(stderr, stderr_writer, &mut stderr_output);

    let combined_future = try_join(stdout_future, stderr_future);

    tokio::select! {
        output = combined_future => {
            output?;
        }
        _ = rx => {
            let _ = child.kill().await;
            return Err(anyhow::anyhow!("Command {} was terminated by signal", command));
        }
    }

    let status = child.wait().await?;
    if !status.success() {
        return Err(anyhow::anyhow!(
            "Command {} failed with args {:?}\nError Output: {}",
            command,
            args,
            stderr_output
        ));
    }

    Ok(stdout_output)
}

/// Runs a command, piping its output to stdout and stderr, and returns the stdout output if successful.
pub async fn spawn_command(command: String, args: Vec<String>) -> Result<(Option<u32>, JoinHandle<Result<String, anyhow::Error>>)> {

    // print command out with args joined by space
    tracing::info!("spawn command: {} {}", command, args.join(" "));

    let mut child = Command::new(&command)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

        let process_id = child.id();
        let join_handle = tokio::spawn({
            async move {
                let stdout = child.stdout.take().ok_or_else(|| {
                    anyhow::anyhow!("Failed to capture standard output from command {}", command)
                })?;
                let stderr = child.stderr.take().ok_or_else(|| {
                    anyhow::anyhow!("Failed to capture standard error from command {}", command)
                })?;

                let mut stdout_output = String::new();
                let mut stderr_output = String::new();

                let stdout_writer = io::stdout();
                let stderr_writer = io::stderr();

                let stdout_future = pipe_output(stdout, stdout_writer, &mut stdout_output);
                let stderr_future = pipe_error_output(stderr, stderr_writer, &mut stderr_output);

                let _ = try_join(stdout_future, stderr_future).await;

                let status = child.wait().await?;
                if !status.success() {
                    return Err(anyhow::anyhow!(
                        "Command {} spawn failed with args {:?}\nError Output: {}",
                        command,
                        args,
                        stderr_output
                    ));
                }

                Ok(stdout_output)                
            }
        });

        Ok((process_id,join_handle))
}

#[cfg(test)]
pub mod tests {

    use super::*;

    #[tokio::test]
    async fn test_run_command() -> Result<(), anyhow::Error> {
        let output = run_command("echo", &["Hello, world!"]).await?;
        assert_eq!(output, "Hello, world!\n");
        Ok(())
    }

}