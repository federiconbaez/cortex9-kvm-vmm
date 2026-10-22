//! Memoria del guest: un bloque alineado a página, en cero.

use std::alloc::{alloc_zeroed, dealloc, Layout};
use std::ptr::NonNull;

/// Región de memoria anfitriona que respalda la RAM del guest.
pub struct GuestMemory {
    ptr: NonNull<u8>,
    layout: Layout,
}

// SAFETY: la región es memoria propia; el acceso concurrente lo regula `&`/`&mut`.
unsafe impl Send for GuestMemory {}

impl GuestMemory {
    pub const PAGE: usize = 4096;

    /// Reserva `size` bytes (múltiplo de 4 KiB, distinto de cero).
    pub fn new(size: usize) -> Option<Self> {
        if size == 0 || !size.is_multiple_of(Self::PAGE) {
            return None;
        }
        let layout = Layout::from_size_align(size, Self::PAGE).ok()?;
        // SAFETY: layout de tamaño no nulo.
        let ptr = NonNull::new(unsafe { alloc_zeroed(layout) })?;
        Some(Self { ptr, layout })
    }

    pub fn len(&self) -> usize {
        self.layout.size()
    }

    pub fn is_empty(&self) -> bool {
        false
    }

    /// Dirección anfitriona, para `KVM_SET_USER_MEMORY_REGION`.
    pub fn host_addr(&self) -> u64 {
        self.ptr.as_ptr() as u64
    }

    pub fn as_slice(&self) -> &[u8] {
        // SAFETY: ptr válido por len bytes inicializados (alloc_zeroed).
        unsafe { std::slice::from_raw_parts(self.ptr.as_ptr(), self.len()) }
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        // SAFETY: igual que as_slice, con acceso exclusivo por &mut self.
        unsafe { std::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len()) }
    }
}

impl Drop for GuestMemory {
    fn drop(&mut self) {
        // SAFETY: ptr y layout provienen de alloc_zeroed.
        unsafe { dealloc(self.ptr.as_ptr(), self.layout) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_sizes() {
        assert!(GuestMemory::new(0).is_none());
        assert!(GuestMemory::new(4097).is_none());
    }

    #[test]
    fn page_aligned_and_zeroed() {
        let mut m = GuestMemory::new(2 * GuestMemory::PAGE).unwrap();
        assert_eq!(m.host_addr() % GuestMemory::PAGE as u64, 0);
        assert!(m.as_slice().iter().all(|&b| b == 0));
        m.as_mut_slice()[8191] = 7;
        assert_eq!(m.as_slice()[8191], 7);
        assert_eq!(m.len(), 8192);
    }
}
