//! VMM mínimo sobre KVM: arranca un guest x86-64 directamente en modo largo
//! y expone una consola serie 16550 emulada en PIO.

pub mod boot;
pub mod guest;
pub mod layout;
pub mod mem;
pub mod uart;
pub mod vmm;
