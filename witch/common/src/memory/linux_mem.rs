// SPDX-FileCopyrightText: 2025 Ada Freya Ahmed (neptuwunium)
// SPDX-License-Identifier: EUPL-1.2

use std::ffi::c_void;
use std::fs;

use anyhow::{Result, bail};
use libc::pid_t;

use crate::engine::LuminousPointer;
use crate::memory::MemoryReader;
use crate::memory::linux_proc::{MemoryMapping, get_process_base};

pub struct LinuxMemoryReader {
	pid: pid_t,
	proc: Vec<MemoryMapping>,
}

impl LinuxMemoryReader {
	pub fn new(pid: pid_t) -> Result<LinuxMemoryReader> {
		let mut proc: Vec<MemoryMapping> = Vec::new();
		let proc_bytes = match fs::read(format!("/proc/{}/proc", pid)).ok() {
			Some(bytes) => bytes,
			None => bail!("failed to read /proc/{}/proc", pid),
		};
		let proc_txt = String::from_utf8_lossy(&proc_bytes).to_string();
		for line in proc_txt.split("\n") {
			if let Some(module) = MemoryMapping::new(line) {
				proc.push(module);
			}
		}
		Ok(LinuxMemoryReader {
			pid,
			proc,
		})
	}
}

impl MemoryReader for LinuxMemoryReader {
	fn read(&mut self, address: LuminousPointer<()>, buf: &mut [u8]) -> Result<()> {
		let local_iov = libc::iovec {
			iov_base: buf.as_mut_ptr() as *mut c_void,
			iov_len: buf.len(),
		};

		let remote_iov = libc::iovec {
			iov_base: address.inner as *mut c_void,
			iov_len: buf.len(),
		};

		let read = unsafe { libc::process_vm_readv(self.pid, &local_iov, 1, &remote_iov, 1, 0) };
		if read != buf.len() as isize {
			bail!("process_vm_readv failed: {}", std::io::Error::last_os_error());
		}

		Ok(())
	}

	fn get_base_address(&self) -> LuminousPointer<()> {
		get_process_base(self.get_process_name(), &self.proc)
	}

	fn get_process_name(&self) -> Option<String> {
		let path = fs::read(format!("/proc/{}/comm", self.pid)).ok()?;
		Some(String::from_utf8_lossy(&path).to_string().split(&['\\', '/'][..]).next_back()?.to_string())
	}
}
