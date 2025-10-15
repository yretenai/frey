// SPDX-FileCopyrightText: 2025 Ada Freya Ahmed (neptuwunium)
// SPDX-License-Identifier: EUPL-1.2

use std::fs::{File, create_dir_all};
use std::io::Write;
#[cfg(target_os = "linux")]
use std::os::unix::raw::pid_t;
use std::path::PathBuf;
use std::process::exit;

use anyhow::Result;
use clap::Parser;
use colog::format::CologStyle;
use colored::Colorize;
use env_logger::fmt::Formatter;
use log::{LevelFilter, Record, error, info};
use witch_common::engine::ebex::ObjectInfoRegistry;
#[cfg(target_os = "linux")]
use witch_common::memory::linux_mem::LinuxMemoryReader;
use witch_common::memory::neptuwunium_dump::Np93DumpReader;
#[cfg(target_os = "windows")]
use witch_common::memory::windows_mem::Win32MemoryReader;
use witch_common::memory::windows_minidump::Win32DumpReader;
use witch_common::memory::{MemoryCursor, MemoryReaderType};

#[derive(Parser)]
#[command(version, about)]
struct Cli {
	#[arg(long, help = "np93 dump path")]
	np93_dump: Option<PathBuf>,
	#[arg(long, help = "minidump path")]
	minidump: Option<PathBuf>,
	#[cfg(any(target_os = "linux", target_os = "windows"))]
	#[arg(long, help = "process pid")]
	pid: u32,

	#[arg(index = 1, required = true, help = "folder to output files to")]
	output_path: PathBuf,

	#[arg(short = 'q', long, help = "output only errors and warnings")]
	quiet: bool,

	#[arg(short = 'v', long, help = "output verbose information")]
	verbose: bool,
}

pub struct PrefixModule;

impl CologStyle for PrefixModule {
	fn format(&self, buf: &mut Formatter, record: &Record<'_>) -> Result<(), std::io::Error> {
		let sep = self.line_separator();
		let prefix = self.prefix_token(&record.level());

		write!(buf, "{}", prefix)?;
		write!(buf, "{}{}{}", "[".blue().bold(), format!("{}", buf.timestamp()).bright_cyan(), "]".blue().bold())?;
		write!(buf, "{}{}{}", "[".blue().bold(), record.target().bright_purple(), "] ".blue().bold())?;
		writeln!(buf, "{}", record.args().to_string().replace('\n', &sep))?;

		Ok(())
	}
}

// todo: move me to a common sorcery lib
pub fn init_logging(filter: LevelFilter) {
	log::set_max_level(filter);

	let mut builder = colog::basic_builder();
	builder.format(colog::formatter(PrefixModule));
	builder.filter(None, filter);
	builder.init();
}

fn main() -> Result<()> {
	let args = Cli::parse();

	let mut log_level = LevelFilter::Info;
	if args.quiet {
		log_level = LevelFilter::Warn;
	} else if args.verbose {
		log_level = LevelFilter::Debug;
	}

	init_logging(log_level);

	let reader: MemoryReaderType;
	if let Some(np93) = args.np93_dump {
		reader = MemoryReaderType::Neptuwunium(Np93DumpReader::new(np93.as_path())?);
	} else if let Some(minidump) = args.minidump {
		reader = MemoryReaderType::Minidump(Win32DumpReader::new(minidump.as_path())?);
	} else {
		#[cfg(target_os = "windows")]
		{
			reader = MemoryReaderType::Windows(Win32MemoryReader::new(args.pid)?);
		}
		#[cfg(target_os = "linux")]
		{
			reader = MemoryReaderType::Linux(LinuxMemoryReader::new(args.pid as pid_t)?);
		}

		error!("need to provide either a --np93-dump or --minidump");
		exit(1);
	}

	let output_dir = args.output_path;
	if !output_dir.exists() {
		create_dir_all(&output_dir)?;
	}

	let mut cursor = MemoryCursor::new(reader);
	let ebex = ObjectInfoRegistry::new(&mut cursor)?;

	let elements_path = output_dir.join(format!("{:?}_ObjectInfos.ldjson", cursor.inner.game_type()));
	let mut elements_json = File::options().write(true).create(true).truncate(true).open(&elements_path)?;
	for (_, element) in ebex.elements {
		info!("{}", element.name);
		let json = serde_json::to_string(&element)?;
		elements_json.write_all(json.as_bytes())?;
		elements_json.write_all(b"\n")?;
	}

	// todo: modules

	Ok(())
}
