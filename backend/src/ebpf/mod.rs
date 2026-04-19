//! eBPF/XDP integration for early-stage packet filtering.
//!
//! Uses the [Aya](https://aya-rs.dev/) pure-Rust eBPF framework.
//! The eBPF program itself is written in Rust using `aya-ebpf` and compiled
//! to BPF bytecode as a separate crate (see `ebpf-xdp/` workspace member).
//!
//! This module provides the **userspace loader** that:
//! 1. Loads the compiled BPF ELF object into the kernel
//! 2. Attaches it as an XDP program to a network interface
//! 3. Manages the `drop_ips` BPF hash map for runtime IP blacklisting
//!
//! # Feature gate
//!
//! All functionality is behind `--features ebpf`. When disabled, `init_ebpf`
//! is a no-op that logs a warning.

#[cfg(feature = "ebpf")]
use aya::programs::{Xdp, XdpFlags};
#[cfg(feature = "ebpf")]
use aya::Ebpf;
#[cfg(feature = "ebpf")]
use aya::maps::HashMap;
#[cfg(feature = "ebpf")]
use std::net::Ipv4Addr;

#[cfg(feature = "ebpf")]
use thiserror::Error;

#[cfg(feature = "ebpf")]
#[derive(Debug, Error)]
pub enum EbpfError {
    #[error("Failed to load BPF program: {0}")]
    Load(#[from] aya::EbpfError),
    #[error("Failed to attach XDP to interface `{0}`: {1}")]
    Attach(String, aya::programs::ProgramError),
    #[error("Map operation failed: {0}")]
    Map(#[from] aya::maps::MapError),
}

/// Load and attach the XDP firewall program to `iface`.
///
/// The BPF ELF object is expected at a well-known path that the companion
/// `ebpf-xdp` crate produces. In production this would be baked via
/// `include_bytes_aligned!` or resolved from a config path.
///
/// # Errors
///
/// Returns [`EbpfError`] if the BPF object cannot be loaded or attached.
#[cfg(feature = "ebpf")]
pub fn init_ebpf(iface: &str, bpf_obj_path: &str) -> Result<Ebpf, EbpfError> {
    tracing::info!("Loading Aya eBPF XDP program from {} onto {}", bpf_obj_path, iface);

    let mut ebpf = Ebpf::load_file(bpf_obj_path)?;

    let program: &mut Xdp = ebpf.program_mut("xdp_firewall")
        .expect("BPF object missing `xdp_firewall` program section")
        .try_into()
        .expect("program is not XDP");

    program.load()
        .map_err(|e| EbpfError::Attach(iface.to_string(), e))?;
    program.attach(iface, XdpFlags::default())
        .map_err(|e| EbpfError::Attach(iface.to_string(), e))?;

    tracing::info!("XDP firewall attached to {}", iface);
    Ok(ebpf)
}

/// Insert an IPv4 address into the XDP drop map at runtime.
#[cfg(feature = "ebpf")]
pub fn block_ip(ebpf: &mut Ebpf, ip: Ipv4Addr) -> Result<(), EbpfError> {
    let mut drop_map: HashMap<_, u32, u32> =
        HashMap::try_from(ebpf.map_mut("drop_ips").expect("missing drop_ips map"))?;
    let ip_u32 = u32::from_be_bytes(ip.octets());
    drop_map.insert(ip_u32, 1, 0)?;
    tracing::warn!("eBPF: blacklisted IP {}", ip);
    Ok(())
}

/// Remove an IPv4 address from the XDP drop map.
#[cfg(feature = "ebpf")]
pub fn unblock_ip(ebpf: &mut Ebpf, ip: Ipv4Addr) -> Result<(), EbpfError> {
    let mut drop_map: HashMap<_, u32, u32> =
        HashMap::try_from(ebpf.map_mut("drop_ips").expect("missing drop_ips map"))?;
    let ip_u32 = u32::from_be_bytes(ip.octets());
    drop_map.remove(&ip_u32)?;
    tracing::info!("eBPF: unblocked IP {}", ip);
    Ok(())
}

/// No-op stub when compiled without `--features ebpf`.
#[cfg(not(feature = "ebpf"))]
pub fn init_ebpf(_iface: &str, _bpf_obj_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    tracing::warn!("eBPF support not compiled — rebuild with --features ebpf");
    Ok(())
}
