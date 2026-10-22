//! Imagen del guest ensamblada por `build.rs` y los valores que debe imprimir.

/// Imagen plana del guest (`guest/guest.S`), enlazada para `GUEST_LOAD_ADDR`.
pub static GUEST_IMAGE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/guest.bin"));

/// Primera línea que imprime el guest.
pub const BANNER: &str = "cortex9-kvm-vmm guest: hola desde x86-64";
/// Línea que confirma EFER.LMA, CR0.PG y CR4.PAE leídos desde el guest.
pub const MODE_LINE: &str = "modo: EFER.LMA=1 CR0.PG=1 CR4.PAE=1";
/// Última línea antes de la salida.
pub const DONE_LINE: &str = "guest: fin";
/// Límite del conteo de primos (debe coincidir con `PRIME_LIMIT` en el .S).
pub const PRIME_LIMIT: u64 = 100_000;

/// Cantidad y suma de primos menores que `limit`, con una criba en el host.
/// Sirve de referencia independiente para lo que calcula el guest.
pub fn reference_primes(limit: u64) -> (u64, u64) {
    let n = limit as usize;
    let mut sieve = vec![true; n.max(2)];
    sieve[0] = false;
    sieve[1] = false;
    let mut i = 2;
    while i * i < n {
        if sieve[i] {
            let mut j = i * i;
            while j < n {
                sieve[j] = false;
                j += i;
            }
        }
        i += 1;
    }
    sieve
        .iter()
        .enumerate()
        .take(n)
        .filter(|(_, &p)| p)
        .fold((0, 0), |(c, s), (k, _)| (c + 1, s + k as u64))
}

/// Salida serie completa que se espera del guest.
pub fn expected_serial() -> String {
    let (count, sum) = reference_primes(PRIME_LIMIT);
    format!(
        "{BANNER}\n{MODE_LINE}\nprimos<{PRIME_LIMIT}: cantidad={count} suma={sum}\n{DONE_LINE}\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{setup_memory, translate, GUEST_LOAD_ADDR, GUEST_MEM_SIZE, PML4_ADDR};

    fn contains(hay: &[u8], needle: &[u8]) -> bool {
        hay.windows(needle.len()).any(|w| w == needle)
    }

    #[test]
    fn reference_primes_known_values() {
        assert_eq!(reference_primes(10), (4, 17));
        assert_eq!(reference_primes(100), (25, 1060));
        assert_eq!(reference_primes(PRIME_LIMIT), (9592, 454_396_537));
    }

    #[test]
    fn image_is_flat_and_starts_at_entry() {
        assert!(!GUEST_IMAGE.is_empty());
        assert!(GUEST_IMAGE.len() < 4096, "{} bytes", GUEST_IMAGE.len());
        assert_eq!(GUEST_IMAGE[0], 0xfc, "la entrada debe ser `cld`");
        assert_ne!(&GUEST_IMAGE[..4], b"\x7fELF", "debe ser binario plano");
    }

    #[test]
    fn image_contains_messages() {
        for m in [BANNER, MODE_LINE, DONE_LINE, "primos<100000: cantidad="] {
            assert!(contains(GUEST_IMAGE, m.as_bytes()), "falta {m:?}");
        }
    }

    #[test]
    fn image_loads_into_mapped_memory() {
        let mut mem = vec![0u8; GUEST_MEM_SIZE];
        let entry = setup_memory(&mut mem, GUEST_IMAGE).unwrap();
        let start = GUEST_LOAD_ADDR as usize;
        assert_eq!(&mem[start..start + GUEST_IMAGE.len()], GUEST_IMAGE);
        let last = entry + GUEST_IMAGE.len() as u64 - 1;
        assert_eq!(translate(&mem, PML4_ADDR, last), Some(last));
    }

    #[test]
    fn expected_serial_shape() {
        let s = expected_serial();
        assert!(s.starts_with(BANNER));
        assert!(s.contains("cantidad=9592 suma=454396537\n"));
        assert_eq!(s.lines().count(), 4);
    }
}
