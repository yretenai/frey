// SPDX-FileCopyrightText: 2025 Ada Freya Ahmed (neptuwunium)
// SPDX-License-Identifier: EUPL-1.2

use flexi_logger::{Duplicate, FileSpec, Logger, detailed_format};

#[cfg(target_os = "windows")]
mod scarlet;

fn main() -> anyhow::Result<()> {
	Logger::try_with_env_or_str("debug")?
		.log_to_file(FileSpec::default().basename("scarlet-launcher").directory("scarlet").suppress_timestamp())
		.duplicate_to_stderr(Duplicate::Info)
		.format(detailed_format)
		.start()?;

	#[cfg(target_os = "windows")]
	scarlet::scarlet_main()?;

	#[cfg(not(target_os = "windows"))]
	log::error!("this executable only works on windows");

	Ok(())
}
