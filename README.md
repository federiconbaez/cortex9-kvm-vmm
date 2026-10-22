# cortex9-kvm-vmm

VMM mínimo en Rust sobre KVM: arranca un guest x86-64 directamente en modo largo y expone una consola serie 16550 emulada por PIO.

> **ES** abajo · **EN** below

## ES

### Qué hace

- Abre `/dev/kvm` con `kvm-ioctls`, crea una VM con una vCPU y 4 MiB de memoria.
- Escribe una GDT (nula, código de 64 bits con L=1, datos) en `0x500` y tablas de páginas de 4 niveles (PML4 → PDPT → PD con páginas de 2 MiB) que mapean con identidad el primer GiB.
- Configura CR0 (PE, PG, …), CR3, CR4.PAE, EFER.LME|LMA, segmentos planos, RIP en `0x100000` y la pila, y entra directo en modo largo, sin pasar por modo real ni protegido.
- Emula una UART 16550 en `0x3f8` (DLAB y divisor, IER, FCR/IIR, LCR, MCR con loopback, LSR, MSR, scratch). Solo transmite; no recibe ni genera interrupciones.
- El guest (`guest/guest.S`, ensamblado en `build.rs` con `as`, `ld` y `objcopy`) inicializa la UART, imprime un banner, lee EFER, CR0 y CR4 para comprobar LMA, PG y PAE, cuenta y suma los primos menores que 100000 y termina escribiendo el código de salida en el puerto `0xf4`.

Salida serie esperada:

```text
cortex9-kvm-vmm guest: hola desde x86-64
modo: EFER.LMA=1 CR0.PG=1 CR4.PAE=1
primos<100000: cantidad=9592 suma=454396537
guest: fin
```

### Alcance

Lo medido en la máquina de desarrollo (Debian 13, kernel 6.12, Intel Xeon virtualizado con 8 vCPU, rustc 1.99.1 estable y nightly 2026-10-21, binutils 2.44), **sin acceso a `/dev/kvm`**:

| Verificación | Comando | Resultado |
|---|---|---|
| Tests unitarios (layout, GDT, tablas de páginas, sregs/regs, UART, memoria, imagen del guest) | `cargo test --all-targets` | 28 pasan, 0 fallan; 3 de arranque ignoradas |
| Miri sobre los tests unitarios | `cargo +nightly miri test --lib` | 28 pasan |
| AddressSanitizer sobre los tests unitarios | `RUSTFLAGS=-Zsanitizer=address cargo +nightly test -Zbuild-std --lib --target x86_64-unknown-linux-gnu` | 28 pasan |
| `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` | | sin diferencias ni avisos |
| Guest emulado en Unicorn con las mismas tablas de páginas | `python3 tools/emulate_guest.py <guest.bin>` | salida idéntica a la esperada, código 0 |
| Control negativo en Unicorn sin CR0.PG ni CR4.PAE | `... --no-long-mode` | imprime `FALLO`, código 1 |
| Pruebas de arranque con `CORTEX9_KVM=1` sin acceso a `/dev/kvm` | `CORTEX9_KVM=1 cargo test --test boot -- --include-ignored` | las 3 fallan con `Permission denied`, como se espera |

**El arranque real sobre KVM no se ejecutó en esta máquina.** Lo cubre el job `kvm-boot` del CI, que habilita `/dev/kvm` en el runner, falla si el dispositivo no es accesible y compara la salida serie completa. Hasta que ese job corra en verde, el arranque sobre KVM no está demostrado.

### Pruebas de arranque y `CORTEX9_KVM`

Las pruebas de `tests/boot.rs` están marcadas `#[ignore]` porque necesitan `/dev/kvm`.

- `cargo test`: quedan ignoradas.
- `CORTEX9_KVM=1 cargo test --test boot -- --include-ignored`: corren y **fallan** si `/dev/kvm` no se puede abrir en lectura y escritura. Así corren en el CI.
- `cargo test --test boot -- --include-ignored` sin `CORTEX9_KVM=1`: cada prueba imprime `SALTEADA` y el motivo, y el harness la cuenta como pasada aunque no corrió.

### Uso

```sh
cargo run --release              # arranca el guest incluido
cargo run --release -- --image guest.bin --max-exits 100000
```

Requiere Linux x86-64, acceso de lectura y escritura a `/dev/kvm` y binutils (`as`, `ld`, `objcopy`) para compilar el guest.

### Limitaciones

- Una sola vCPU, sin IDT ni APIC: cualquier excepción en el guest termina en triple fault (`Shutdown`).
- La UART solo transmite; la recepción y las interrupciones no están implementadas.
- No hay MMIO, PCI, virtio ni reloj.
- No arranca Linux: el protocolo de arranque de 64 bits de `bzImage` no está implementado.
- Memoria fija de 4 MiB; las tablas mapean 1 GiB.
- La comprobación en Unicorn valida la lógica del guest, no la GDT ni los sregs que arma el VMM.
- Sin mediciones de rendimiento.

### Licencia

Apache-2.0. Ver [LICENSE](LICENSE).

## EN

### What it does

- Opens `/dev/kvm` with `kvm-ioctls` and creates a VM with one vCPU and 4 MiB of memory.
- Writes a GDT (null, 64-bit code with L=1, data) at `0x500` and 4-level page tables (PML4 → PDPT → PD with 2 MiB pages) that identity-map the first GiB.
- Sets CR0 (PE, PG, …), CR3, CR4.PAE, EFER.LME|LMA, flat segments, RIP at `0x100000` and the stack, and enters long mode directly, without going through real or protected mode.
- Emulates a 16550 UART at `0x3f8` (DLAB and divisor, IER, FCR/IIR, LCR, MCR with loopback, LSR, MSR, scratch). Transmit only; no receive and no interrupts.
- The guest (`guest/guest.S`, assembled in `build.rs` with `as`, `ld` and `objcopy`) initialises the UART, prints a banner, reads EFER, CR0 and CR4 to check LMA, PG and PAE, counts and sums the primes below 100000, and exits by writing its exit code to port `0xf4`.

Expected serial output: see the block in the Spanish section above.

### Scope

Measured on the development machine (Debian 13, kernel 6.12, virtualised Intel Xeon with 8 vCPUs, rustc 1.99.1 stable and nightly 2026-10-21, binutils 2.44), **without access to `/dev/kvm`**:

| Check | Command | Result |
|---|---|---|
| Unit tests (layout, GDT, page tables, sregs/regs, UART, memory, guest image) | `cargo test --all-targets` | 28 pass, 0 fail; 3 boot tests ignored |
| Miri over the unit tests | `cargo +nightly miri test --lib` | 28 pass |
| AddressSanitizer over the unit tests | `RUSTFLAGS=-Zsanitizer=address cargo +nightly test -Zbuild-std --lib --target x86_64-unknown-linux-gnu` | 28 pass |
| `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` | | no diffs, no warnings |
| Guest emulated in Unicorn with the same page tables | `python3 tools/emulate_guest.py <guest.bin>` | output identical to the expected one, exit code 0 |
| Negative control in Unicorn without CR0.PG and CR4.PAE | `... --no-long-mode` | prints `FALLO`, exit code 1 |
| Boot tests with `CORTEX9_KVM=1` and no `/dev/kvm` access | `CORTEX9_KVM=1 cargo test --test boot -- --include-ignored` | all 3 fail with `Permission denied`, as expected |

**A real boot on KVM was not run on this machine.** It is covered by the CI `kvm-boot` job, which enables `/dev/kvm` on the runner, fails if the device is not accessible, and compares the full serial output. Until that job runs green, booting on KVM is not demonstrated.

### Boot tests and `CORTEX9_KVM`

The tests in `tests/boot.rs` are `#[ignore]` because they need `/dev/kvm`.

- `cargo test`: they stay ignored.
- `CORTEX9_KVM=1 cargo test --test boot -- --include-ignored`: they run and **fail** if `/dev/kvm` cannot be opened read-write. This is how CI runs them.
- `cargo test --test boot -- --include-ignored` without `CORTEX9_KVM=1`: each test prints `SALTEADA` (skipped) with the reason, and the harness counts it as passed even though it did not run.

### Usage

```sh
cargo run --release              # boots the bundled guest
cargo run --release -- --image guest.bin --max-exits 100000
```

Needs x86-64 Linux, read-write access to `/dev/kvm`, and binutils (`as`, `ld`, `objcopy`) to build the guest.

### Limitations

- Single vCPU, no IDT and no APIC: any guest exception ends in a triple fault (`Shutdown`).
- The UART only transmits; receive and interrupts are not implemented.
- No MMIO, PCI, virtio or clock.
- Does not boot Linux: the 64-bit `bzImage` boot protocol is not implemented.
- Fixed 4 MiB of memory; the page tables map 1 GiB.
- The Unicorn check validates the guest logic, not the GDT or sregs the VMM sets up.
- No performance measurements.

### License

Apache-2.0. See [LICENSE](LICENSE).
