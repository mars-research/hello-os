global start
global long_mode_start
extern rust_main
global _bootinfo

section .text

bits 64
long_mode_start:

    ; initialize segments
    ; setup stack

    ; load 0 into all data segment registers
    mov ax, 0
    mov ss, ax
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax

    mov esp, stack_top

    call rust_main

    hlt


bits 32    ; By default, GRUB sets us to 32-bit mode.
start:
    ; setup page tables 
    call check_multiboot
    call set_up_page_tables
        
    call enable_paging

    ; switch to long mode 

    ; load the 64-bit GDT
    lgdt [gdt64.pointer]

    ; jump to long mode 
    jmp gdt64.code:long_mode_start

bits 32
check_multiboot:
    cmp eax, 0x36d76289 ; If multiboot, this value will be in the eax register on boot.
    mov [_bootinfo], ebx
    jne .no_multiboot
    ret
.no_multiboot:
    mov al, "0"
    jmp error

set_up_page_tables:
    ;
    ; connect pml4 and pml3

    ; write a loop that initializes pml3 to map 4GBs

    mov eax, p3_table
    or eax, 0b11
    mov [p4_table], eax

    ;Setign up page_table3
    ;for i in 0..3
    ; p3[i] = i*1GB + | Present | R/W | Huge
    xor edi, edi        ; i = 0
.loop:
    mov eax, edi
    shl eax, 30
    or eax, 0b10000011
    mov [p3_table + 8*edi], eax

    inc edi
    cmp edi, 4          ; compare i with 4
    jne .loop           ; jump if i != 4

    ret

enable_paging:
    ; load P4 to cr3 register (cpu uses this to access the P4 table)
    mov eax, p4_table
    mov cr3, eax

    ; enable PAE-flag in cr4 (Physical Address Extension)
    mov eax, cr4
    or eax, 1 << 5
    mov cr4, eax

    ; set the long mode bit in the EFER MSR (model specific register)
    mov ecx, 0xC0000080
    rdmsr
    or eax, 1 << 8
    wrmsr

    ; enable paging in the cr0 register
    mov eax, cr0
    or eax, 1 << 31
    mov cr0, eax

    ret

; Prints `ERR: ` and the given error code to screen and hangs.
; parameter: error code (in ascii) in al
error:
    mov dword [0xb8000], 0x4f524f45
    mov dword [0xb8004], 0x4f3a4f52
    mov dword [0xb8008], 0x4f204f20
    mov byte  [0xb800a], al
    hlt

section .rodata
gdt64:
    dq 0 ; zero entry
.code: equ $ - gdt64 
    dq (1<<43) | (1<<44) | (1<<47) | (1<<53) ; code segment
.pointer:
    dw $ - gdt64 - 1
    dq gdt64

section .bss
align 4096

p4_table:
    resb 4096
p3_table:
    resb 4096

stack_bottom:
    resb 4096 * 4 ; Reserve this many bytes
stack_top:

_bootinfo:
    resb 8 ; Place holder to save bootinfo entry
