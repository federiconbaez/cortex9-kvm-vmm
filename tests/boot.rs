//! Pruebas de arranque reales sobre KVM.
//!
//! Están marcadas `#[ignore]` porque necesitan `/dev/kvm`. En CI se ejecutan
//! con `CORTEX9_KVM=1 cargo test --test boot -- --include-ignored`: con esa
//! variable, si `/dev/kvm` no existe o no se puede abrir, la prueba FALLA.
//! Sin la variable (por ejemplo `--include-ignored` en una máquina sin KVM)
//! se saltean imprimiendo el motivo.

use cortex9_kvm_vmm::guest::{expected_serial, GUEST_IMAGE};
use cortex9_kvm_vmm::vmm::{run_guest, GuestExit};
use std::fs::OpenOptions;

const IGNORE_MSG: &str =
    "necesita /dev/kvm: CORTEX9_KVM=1 cargo test --test boot -- --include-ignored";

/// Devuelve `true` si hay que correr la prueba. Con `CORTEX9_KVM=1` exige
/// que `/dev/kvm` se pueda abrir en lectura/escritura y si no, entra en pánico.
fn kvm_required(test: &str) -> bool {
    if std::env::var("CORTEX9_KVM").as_deref() != Ok("1") {
        eprintln!("SALTEADA {test}: CORTEX9_KVM no es 1; {IGNORE_MSG}");
        return false;
    }
    if let Err(e) = OpenOptions::new().read(true).write(true).open("/dev/kvm") {
        panic!("CORTEX9_KVM=1 pero /dev/kvm no se puede abrir en lectura/escritura: {e}");
    }
    true
}

#[test]
#[ignore = "necesita /dev/kvm; ver CORTEX9_KVM"]
fn boots_long_mode_guest_and_reads_serial() {
    if !kvm_required("boots_long_mode_guest_and_reads_serial") {
        return;
    }
    let report = run_guest(GUEST_IMAGE, 100_000).expect("run_guest");
    let serial = String::from_utf8_lossy(&report.serial);
    print!("{serial}");
    eprintln!("exit={:?} vm_exits={}", report.exit, report.vm_exits);
    assert_eq!(report.exit, GuestExit::DebugExit(0), "serial: {serial}");
    assert_eq!(serial, expected_serial());
    assert!(serial.contains("cantidad=9592 suma=454396537"));
    // Cada byte transmitido pasa por al menos dos salidas (LSR + THR).
    assert!(report.vm_exits >= 2 * report.serial.len() as u64);
}

#[test]
#[ignore = "necesita /dev/kvm; ver CORTEX9_KVM"]
fn debug_exit_code_is_propagated() {
    if !kvm_required("debug_exit_code_is_propagated") {
        return;
    }
    // mov al, 7 ; out 0xf4, al ; hlt
    let image = [0xb0, 0x07, 0xe6, 0xf4, 0xf4];
    let report = run_guest(&image, 100).expect("run_guest");
    assert_eq!(report.exit, GuestExit::DebugExit(7));
    assert!(report.serial.is_empty());
}

#[test]
#[ignore = "necesita /dev/kvm; ver CORTEX9_KVM"]
fn hlt_and_exit_limit() {
    if !kvm_required("hlt_and_exit_limit") {
        return;
    }
    let report = run_guest(&[0xf4], 100).expect("run_guest");
    assert_eq!(report.exit, GuestExit::Hlt);
    assert_eq!(report.vm_exits, 1);

    // Bucle infinito escribiendo en un puerto sin dispositivo:
    // 1: out 0x80, al ; jmp 1b
    let spin = [0xe6, 0x80, 0xeb, 0xfc];
    let report = run_guest(&spin, 50).expect("run_guest");
    assert_eq!(report.exit, GuestExit::ExitLimit);
    assert_eq!(report.vm_exits, 50);
}
