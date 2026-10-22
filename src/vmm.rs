//! VM de una vCPU sobre KVM: memoria, estado de modo largo y bucle de salidas.

use crate::boot::{initial_regs, long_mode_sregs};
use crate::layout::{setup_memory, LayoutError, GUEST_MEM_SIZE};
use crate::mem::GuestMemory;
use crate::uart::Serial16550;
use kvm_bindings::{kvm_userspace_memory_region, KVM_MAX_CPUID_ENTRIES};
use kvm_ioctls::{Kvm, VcpuExit};

/// Puerto de salida de depuración (como `isa-debug-exit` de QEMU).
pub const DEBUG_EXIT_PORT: u16 = 0xf4;

/// Cómo terminó el guest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuestExit {
    /// El guest escribió `code` en `DEBUG_EXIT_PORT`.
    DebugExit(u8),
    /// El guest ejecutó HLT.
    Hlt,
    /// Triple fault o apagado.
    Shutdown,
    /// Se alcanzó el límite de salidas sin terminar.
    ExitLimit,
}

/// Resultado de una ejecución.
#[derive(Debug)]
pub struct RunReport {
    pub exit: GuestExit,
    pub serial: Vec<u8>,
    pub vm_exits: u64,
}

/// Errores del VMM.
#[derive(Debug)]
pub enum VmmError {
    Kvm(&'static str, kvm_ioctls::Error),
    Layout(LayoutError),
    Memory,
    UnhandledExit(String),
}

impl std::fmt::Display for VmmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Kvm(what, e) => write!(f, "KVM {what}: {e}"),
            Self::Layout(e) => write!(f, "layout: {e}"),
            Self::Memory => write!(f, "no se pudo reservar la memoria del guest"),
            Self::UnhandledExit(s) => write!(f, "salida de vCPU no manejada: {s}"),
        }
    }
}

impl std::error::Error for VmmError {}

impl From<LayoutError> for VmmError {
    fn from(e: LayoutError) -> Self {
        Self::Layout(e)
    }
}

fn kvm<T>(what: &'static str, r: Result<T, kvm_ioctls::Error>) -> Result<T, VmmError> {
    r.map_err(|e| VmmError::Kvm(what, e))
}

/// Arranca `image` en modo largo y ejecuta hasta que termine o se agoten
/// `max_exits` salidas de VM.
pub fn run_guest(image: &[u8], max_exits: u64) -> Result<RunReport, VmmError> {
    // `mem` se declara antes que `vm` y `vcpu`, así que se destruye después
    // de ellos en cualquier camino de salida (incluidos los errores).
    let mut mem = GuestMemory::new(GUEST_MEM_SIZE).ok_or(VmmError::Memory)?;
    let entry = setup_memory(mem.as_mut_slice(), image)?;

    let kvm_fd = kvm("open /dev/kvm", Kvm::new())?;
    let vm = kvm("create_vm", kvm_fd.create_vm())?;

    let region = kvm_userspace_memory_region {
        slot: 0,
        flags: 0,
        guest_phys_addr: 0,
        memory_size: mem.len() as u64,
        userspace_addr: mem.host_addr(),
    };
    // SAFETY: la región vive en `mem`, que sobrevive a `vm` y `vcpu`.
    kvm("set_user_memory_region", unsafe {
        vm.set_user_memory_region(region)
    })?;

    let mut vcpu = kvm("create_vcpu", vm.create_vcpu(0))?;
    let cpuid = kvm(
        "get_supported_cpuid",
        kvm_fd.get_supported_cpuid(KVM_MAX_CPUID_ENTRIES),
    )?;
    kvm("set_cpuid2", vcpu.set_cpuid2(&cpuid))?;

    let sregs = long_mode_sregs(kvm("get_sregs", vcpu.get_sregs())?);
    kvm("set_sregs", vcpu.set_sregs(&sregs))?;
    kvm("set_regs", vcpu.set_regs(&initial_regs(entry)))?;

    let mut uart = Serial16550::default();
    let mut vm_exits = 0u64;
    let exit = loop {
        if vm_exits >= max_exits {
            break GuestExit::ExitLimit;
        }
        vm_exits += 1;
        match kvm("run", vcpu.run())? {
            VcpuExit::IoOut(port, data) if uart.handles(port) => {
                for (i, b) in data.iter().enumerate() {
                    uart.write(port.wrapping_add(i as u16), *b);
                }
            }
            VcpuExit::IoOut(DEBUG_EXIT_PORT, data) => {
                break GuestExit::DebugExit(data.first().copied().unwrap_or(0));
            }
            VcpuExit::IoOut(_, _) => {} // puertos sin dispositivo: se ignoran
            VcpuExit::IoIn(port, data) => {
                for (i, b) in data.iter_mut().enumerate() {
                    let p = port.wrapping_add(i as u16);
                    *b = if uart.handles(p) { uart.read(p) } else { 0xff };
                }
            }
            VcpuExit::Hlt => break GuestExit::Hlt,
            VcpuExit::Shutdown => break GuestExit::Shutdown,
            other => return Err(VmmError::UnhandledExit(format!("{other:?}"))),
        }
    };
    let serial = uart.take_output();
    Ok(RunReport {
        exit,
        serial,
        vm_exits,
    })
}
