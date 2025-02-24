use anyhow::Result;
use futures::future::try_join;
use itertools::Itertools;
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command as InnerCommand;
use tokio::signal::unix::{signal, SignalKind};
use tracing::info;

use std::ffi::OsStr;
use std::process::Stdio;

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
pub async fn run_command<C, I, S>(command: C, args: I) -> Result<String>
where
	C: AsRef<OsStr>,
	I: IntoIterator<Item = S>,
	S: AsRef<OsStr>,
{
	let mut command = Command::new(command);
	command.args(args);
	command.run_and_capture_output().await
}

/// Builder for running commands
pub struct Command(InnerCommand);

impl Command {
	pub fn new(program: impl AsRef<OsStr>) -> Self {
		let inner = InnerCommand::new(program);
		Self(inner)
	}

	pub fn arg<S>(&mut self, arg: S) -> &mut Self
	where
		S: AsRef<OsStr>,
	{
		self.0.arg(arg);
		self
	}

	pub fn args<I, S>(&mut self, args: I) -> &mut Self
	where
		I: IntoIterator<Item = S>,
		S: AsRef<OsStr>,
	{
		self.0.args(args);
		self
	}

	pub async fn run_and_capture_output(&mut self) -> Result<String> {
		let cmd_display = self.0.as_std().get_program().to_string_lossy().into_owned();
		let args_display = self.0.as_std().get_args().map(|s| s.to_string_lossy()).join(" ");

		info!("Running command: {cmd_display} {args_display}");

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

		let mut child = self.0.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?;

		let stdout = child.stdout.take().ok_or_else(|| {
			anyhow::anyhow!("Failed to capture standard output from command {cmd_display}")
		})?;
		let stderr = child.stderr.take().ok_or_else(|| {
			anyhow::anyhow!("Failed to capture standard error from command {cmd_display}")
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
				return Err(anyhow::anyhow!("Command {cmd_display} was terminated by signal"));
			}
		}

		let status = child.wait().await?;
		if !status.success() {
			return Err(anyhow::anyhow!(
				"Command {cmd_display} failed with args {args_display}\nError Output: {}",
				stderr_output
			));
		}

		Ok(stdout_output)
	}
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
