/*
 * Guest x86-64 independiente para cortex9-kvm-vmm.
 *
 * El VMM lo carga en 1 MiB y entra con la CPU ya en modo largo (paginación
 * de 4 niveles, CS de 64 bits). El guest:
 *   1. inicializa la UART 16550 de COM1 (0x3f8) a 115200 8N1,
 *   2. imprime un banner,
 *   3. lee EFER, CR0 y CR4 y comprueba LMA, PG y PAE,
 *   4. cuenta y suma los primos menores que PRIME_LIMIT por división,
 *   5. imprime los resultados en decimal,
 *   6. escribe el código de salida en el puerto 0xf4 y ejecuta HLT.
 *
 * Todo el código es independiente de la posición (direccionamiento
 * relativo a RIP).
 */
    .intel_syntax noprefix
    .code64

    .set COM1,        0x3f8
    .set DEBUG_EXIT,  0xf4
    .set PRIME_LIMIT, 100000
    .set MSR_EFER,    0xc0000080

    .section .text.entry, "ax"
    .globl _start
_start:
    cld
    /* UART: DLAB=1, divisor 1 (115200), 8N1, FIFO, DTR|RTS. */
    mov dx, COM1 + 3
    mov al, 0x80
    out dx, al
    mov dx, COM1 + 0
    mov al, 0x01
    out dx, al
    mov dx, COM1 + 1
    xor eax, eax
    out dx, al
    mov dx, COM1 + 3
    mov al, 0x03
    out dx, al
    mov dx, COM1 + 2
    mov al, 0xc7
    out dx, al
    mov dx, COM1 + 4
    mov al, 0x03
    out dx, al

    lea rsi, [rip + msg_banner]
    call puts

    /* Comprobación del modo: EFER.LMA (bit 10), CR0.PG (bit 31), CR4.PAE (bit 5). */
    mov ecx, MSR_EFER
    rdmsr
    bt eax, 10
    jnc fail
    mov rax, cr0
    bt rax, 31
    jnc fail
    mov rax, cr4
    bt rax, 5
    jnc fail
    lea rsi, [rip + msg_mode]
    call puts

    /* Trabajo real: cantidad y suma de primos < PRIME_LIMIT. */
    xor r12d, r12d          /* cantidad */
    xor r13d, r13d          /* suma */
    mov rbx, 2
1:
    cmp rbx, PRIME_LIMIT
    jae 3f
    mov rdi, rbx
    call is_prime
    test eax, eax
    jz 2f
    inc r12
    add r13, rbx
2:
    inc rbx
    jmp 1b
3:
    lea rsi, [rip + msg_count]
    call puts
    mov rax, r12
    call putdec
    lea rsi, [rip + msg_sum]
    call puts
    mov rax, r13
    call putdec
    mov al, '\n'
    call putc

    lea rsi, [rip + msg_done]
    call puts
    xor eax, eax
    jmp exit

fail:
    lea rsi, [rip + msg_fail]
    call puts
    mov al, 1
exit:
    mov dx, DEBUG_EXIT
    out dx, al
4:
    hlt
    jmp 4b

/* is_prime(rdi) -> eax = 1 si rdi es primo. Usa rax, rcx, rdx. */
is_prime:
    xor eax, eax
    cmp rdi, 2
    jb 9f
    je 8f
    test dil, 1
    jz 9f
    mov rcx, 3
5:
    mov rax, rcx
    mul rcx                 /* rdx:rax = d*d */
    cmp rax, rdi
    ja 8f
    mov rax, rdi
    xor edx, edx
    div rcx
    test rdx, rdx
    jz 7f
    add rcx, 2
    jmp 5b
7:
    xor eax, eax
    ret
8:
    mov eax, 1
9:
    ret

/* putc(al): espera THRE en LSR y transmite. Preserva todo salvo rdx. */
putc:
    push rax
    mov dx, COM1 + 5
6:
    in al, dx
    test al, 0x20
    jz 6b
    pop rax
    mov dx, COM1
    out dx, al
    ret

/* puts(rsi): cadena terminada en NUL. */
puts:
    push rsi
10:
    lodsb
    test al, al
    jz 11f
    call putc
    jmp 10b
11:
    pop rsi
    ret

/* putdec(rax): imprime rax sin signo en decimal. */
putdec:
    push rbx
    push rcx
    sub rsp, 32
    lea rbx, [rsp + 31]
    mov byte ptr [rbx], 0
    mov rcx, 10
12:
    xor edx, edx
    div rcx
    add dl, '0'
    dec rbx
    mov [rbx], dl
    test rax, rax
    jnz 12b
    mov rsi, rbx
    call puts
    add rsp, 32
    pop rcx
    pop rbx
    ret

    .section .rodata
msg_banner: .asciz "cortex9-kvm-vmm guest: hola desde x86-64\n"
msg_mode:   .asciz "modo: EFER.LMA=1 CR0.PG=1 CR4.PAE=1\n"
msg_count:  .asciz "primos<100000: cantidad="
msg_sum:    .asciz " suma="
msg_done:   .asciz "guest: fin\n"
msg_fail:   .asciz "guest: FALLO comprobando el modo largo\n"
