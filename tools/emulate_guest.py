#!/usr/bin/env python3
"""Ejecuta la imagen del guest en Unicorn (emulación, sin KVM).

Comprueba la lógica del guest, no el VMM: Unicorn arma su propio estado de
64 bits; acá solo se cargan las mismas tablas de páginas que usa el VMM, se
activan CR0.PG y CR4.PAE y se emulan la UART y el puerto 0xf4.

Uso:
    pip install unicorn
    python3 tools/emulate_guest.py target/debug/build/cortex9-kvm-vmm-*/out/guest.bin
    python3 tools/emulate_guest.py IMAGEN --no-long-mode   # control negativo

Sale con 0 solo si la salida serie y el código de salida son los esperados.
"""
import struct
import sys

from unicorn import UC_ARCH_X86, UC_HOOK_INSN, UC_MODE_64, Uc
from unicorn.x86_const import (UC_X86_INS_IN, UC_X86_INS_OUT, UC_X86_REG_CR0,
                               UC_X86_REG_CR3, UC_X86_REG_CR4, UC_X86_REG_RSP)

LOAD, PML4, PDPT, PD, STACK = 0x100000, 0x9000, 0xA000, 0xB000, 0x80000
EXPECTED = (
    "cortex9-kvm-vmm guest: hola desde x86-64\n"
    "modo: EFER.LMA=1 CR0.PG=1 CR4.PAE=1\n"
    "primos<100000: cantidad=9592 suma=454396537\n"
    "guest: fin\n"
)
EXPECTED_FAIL = (
    "cortex9-kvm-vmm guest: hola desde x86-64\n"
    "guest: FALLO comprobando el modo largo\n"
)


def main():
    image = open(sys.argv[1], "rb").read()
    long_mode = "--no-long-mode" not in sys.argv
    mu = Uc(UC_ARCH_X86, UC_MODE_64)
    mu.mem_map(0, 4 << 20)
    mu.mem_write(LOAD, image)
    mu.mem_write(PML4, struct.pack("<Q", PDPT | 3))
    mu.mem_write(PDPT, struct.pack("<Q", PD | 3))
    for i in range(512):
        mu.mem_write(PD + 8 * i, struct.pack("<Q", (i << 21) | 0x83))
    mu.reg_write(UC_X86_REG_CR3, PML4)
    mu.reg_write(UC_X86_REG_RSP, STACK)
    if long_mode:
        mu.reg_write(UC_X86_REG_CR4, mu.reg_read(UC_X86_REG_CR4) | 0x620)
        mu.reg_write(UC_X86_REG_CR0, 0x80010033)

    out = bytearray()
    st = {"exit": None, "lcr": 0}

    def on_in(uc, port, size, _):
        return 0x60 if port == 0x3FD else 0

    def on_out(uc, port, size, value, _):
        if port == 0x3FB:
            st["lcr"] = value
        elif port == 0x3F8 and not st["lcr"] & 0x80:
            out.append(value & 0xFF)
        elif port == 0xF4:
            st["exit"] = value & 0xFF
            uc.emu_stop()

    mu.hook_add(UC_HOOK_INSN, on_in, None, 1, 0, UC_X86_INS_IN)
    mu.hook_add(UC_HOOK_INSN, on_out, None, 1, 0, UC_X86_INS_OUT)
    mu.emu_start(LOAD, LOAD + len(image), count=50_000_000)

    text = out.decode()
    sys.stdout.write(text)
    print(f"[emu] exit={st['exit']}")
    want = (EXPECTED, 0) if long_mode else (EXPECTED_FAIL, 1)
    return 0 if (text, st["exit"]) == want else 1


if __name__ == "__main__":
    sys.exit(main())
