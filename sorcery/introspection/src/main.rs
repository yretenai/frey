// SPDX-FileCopyrightText: 2025 Ada Freya Ahmed (neptuwunium)
// SPDX-License-Identifier: EUPL-1.2

use std::collections::HashMap;
use std::fmt::Display;
use std::fs::{File, create_dir_all};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::Result;
use clap::Parser;
use colog::format::CologStyle;
use colored::Colorize;
use env_logger::fmt::Formatter;
#[cfg(target_os = "linux")]
use libc::pid_t;
use log::{LevelFilter, Record, info};
use witch_common::engine::LuminousGame;
use witch_common::engine::asset_factory::AssetFactory;
use witch_common::engine::ebex::{
	ObjectFunctionFlag, ObjectFunctionType, ObjectFunctionTypeFlag, ObjectInfoPrimitiveType, ObjectInfoRegistry,
};
use witch_common::engine::game_module::GameModules;
#[cfg(target_os = "linux")]
use witch_common::memory::linux_mem::LinuxMemoryReader;
use witch_common::memory::neptuwunium_dump::Np93DumpReader;
#[cfg(target_os = "windows")]
use witch_common::memory::windows_mem::Win32MemoryReader;
use witch_common::memory::windows_minidump::Win32DumpReader;
use witch_common::memory::{MemoryCursor, MemoryReader};

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

	let reader: MemoryReader;
	if let Some(np93) = args.np93_dump {
		reader = MemoryReader::Neptuwunium(Np93DumpReader::new(np93.as_path())?);
	} else if let Some(minidump) = args.minidump {
		reader = MemoryReader::Minidump(Win32DumpReader::new(minidump.as_path())?);
	} else {
		#[cfg(target_os = "windows")]
		{
			reader = MemoryReader::Windows(Win32MemoryReader::new(args.pid)?);
		}
		#[cfg(target_os = "linux")]
		{
			reader = MemoryReader::Linux(LinuxMemoryReader::new(args.pid as pid_t)?);
		}

		#[cfg(not(any(target_os = "linux", target_os = "windows")))]
		{
			log::error!("need to provide either a --np93-dump or --minidump");
			std::process::exit(1);
		}
	}

	let output_dir = args.output_path;
	if !output_dir.exists() {
		create_dir_all(&output_dir)?;
	}

	let mut cursor = MemoryCursor::new(reader);
	let game_type = cursor.inner.game_type();

	if game_type == LuminousGame::FORSPOKEN {
		let modules = GameModules::new(&mut cursor)?;
		write_ldjson(&output_dir, "Modules", game_type, modules.modules)?;
	}

	let asset_factories = AssetFactory::new(&mut cursor)?;
	let ebex = ObjectInfoRegistry::new(&mut cursor)?;

	write_pseudocode(&output_dir, game_type, &ebex)?;
	write_ldjson(&output_dir, "AssetFactory", game_type, asset_factories.factories)?;
	write_ldjson(&output_dir, "ObjectInfos", game_type, ebex.elements)?;

	Ok(())
}

fn write_pseudocode(output_dir: &Path, game_type: LuminousGame, ebex: &ObjectInfoRegistry) -> Result<()> {
	let path = output_dir.join(format!("{:?}.cs", game_type));
	let mut cs_file = File::options().write(true).create(true).truncate(true).open(&path)?;
	for element in ebex.elements.values() {
		write!(cs_file, "struct {}", element.name)?;
		if let Some(base_type) = &element.base_type {
			write!(cs_file, " : {}", base_type)?;
		}

		if element.properties.properties.is_empty() && element.functions.is_empty() {
			writeln!(cs_file, " {{ }}")?;
			writeln!(cs_file)?;
			continue;
		}

		writeln!(cs_file, " {{")?;

		if !element.properties.properties.is_empty() {
			for property in element.properties.properties.values() {
				write!(cs_file, "\t")?;
				write!(cs_file, "{}", primitive_type_to_string(property.primitive_type).unwrap_or(property.type_name.as_str()))?;
				if property.item_count > 1 {
					write!(cs_file, "[{}]", property.item_count)?;
				}
				writeln!(
					cs_file,
					" {}; // Item Type: {:?}, Item: {:?}, Offset: {:#x}, Size: {:#x}",
					property.name, property.item_primitive_type, property.attributes, property.offset, property.size
				)?;
			}

			if !element.functions.is_empty() {
				writeln!(cs_file)?;
			}
		}

		for func in element.functions.values() {
			write!(cs_file, "\t")?;
			if func.flags.contains(ObjectFunctionFlag::Static) {
				write!(cs_file, "static ")?;
			}

			write!(cs_file, "{}", format_type(&func.return_type))?;
			write!(cs_file, " {}", func.name)?;
			writeln!(cs_file, "({});", func.argument_types.iter().map(format_type).collect::<Vec<_>>().join(", "))?;
		}

		writeln!(cs_file, "}};")?;
		writeln!(cs_file)?;
	}
	Ok(())
}

fn format_type(type_info: &ObjectFunctionType) -> String {
	if type_info.type_flag == ObjectFunctionTypeFlag::Void {
		return "void".to_string();
	}

	let primitive_name = primitive_type_to_string(type_info.primitive_type).unwrap_or(type_info.type_name.as_str());

	if type_info.type_flag == ObjectFunctionTypeFlag::None {
		primitive_name.to_string()
	} else {
		let mut assembled = String::default();
		if type_info.type_flag.contains(ObjectFunctionTypeFlag::Const) {
			assembled.push_str("const ");
		}

		if type_info.type_flag.contains(ObjectFunctionTypeFlag::Void) {
			assembled.push_str("void");
		} else {
			assembled.push_str(primitive_name);
		}

		if type_info.type_flag.contains(ObjectFunctionTypeFlag::Pointer) || type_info.type_flag.contains(ObjectFunctionTypeFlag::Reference)
		{
			assembled.push('*');
		}

		assembled
	}
}

fn primitive_type_to_string(primitive_type: ObjectInfoPrimitiveType) -> Option<&'static str> {
	Some(match primitive_type {
		ObjectInfoPrimitiveType::Int8 => "int8",
		ObjectInfoPrimitiveType::Int16 => "int16",
		ObjectInfoPrimitiveType::Int32 => "int32",
		ObjectInfoPrimitiveType::Int64 => "int64",
		ObjectInfoPrimitiveType::UInt8 => "uint8",
		ObjectInfoPrimitiveType::UInt16 => "uint16",
		ObjectInfoPrimitiveType::UInt32 => "uint32",
		ObjectInfoPrimitiveType::UInt64 => "uint64",
		ObjectInfoPrimitiveType::SizeT => "usize",
		ObjectInfoPrimitiveType::Bool => "bool",
		ObjectInfoPrimitiveType::Float => "float",
		ObjectInfoPrimitiveType::Double => "double",
		ObjectInfoPrimitiveType::String => "string",
		ObjectInfoPrimitiveType::Fixid => "FIXID",
		ObjectInfoPrimitiveType::Float4 => "Vec4",
		ObjectInfoPrimitiveType::Color => "Color",
		ObjectInfoPrimitiveType::Double4 => "Vec4D",
		_ => return None,
	})
}

fn write_ldjson<K, V: Display + serde::Serialize>(
	output_dir: &Path,
	name: &str,
	game_type: LuminousGame,
	elems: HashMap<K, V>,
) -> Result<()> {
	let path = output_dir.join(format!("{:?}_{}.ldjson", game_type, name));
	let mut json_file = File::options().write(true).create(true).truncate(true).open(&path)?;
	let log_str = name.to_string().yellow().bold();
	for (_, element) in elems {
		info!("[{}] {}", log_str, element);
		let json = serde_json::to_string(&element)?;
		json_file.write_all(json.as_bytes())?;
		json_file.write_all(b"\n")?;
	}

	Ok(())
}
