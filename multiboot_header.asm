; Multiboot v1 - Compliant Header for QEMU 
; We use multiboot v1 since Qemu "-kernel" doesn't support 
; multiboot v2
; https://www.gnu.org/software/grub/manual/multiboot/multiboot.html (Section 3.1.1)

; This part MUST be 4-byte aligned, so we solve that issue using 'ALIGN 4'
ALIGN 4
section .multiboot_header
    ; Multiboot macros to make a few lines later more readable
    MULTIBOOT_PAGE_ALIGN	equ 1<<0
    MULTIBOOT_MEMORY_INFO	equ 1<<1 
    MULTIBOOT_HEADER_MAGIC	equ 0x1BADB002                                   ; magic number
    MULTIBOOT_HEADER_FLAGS	equ MULTIBOOT_PAGE_ALIGN | MULTIBOOT_MEMORY_INFO ; flags
    MULTIBOOT_CHECKSUM	equ - (MULTIBOOT_HEADER_MAGIC + MULTIBOOT_HEADER_FLAGS)  ; checksum
                                                                                 ; (magic number + checksum + flags should equal 0)

    ; This is the GRUB Multiboot header. A boot signature
    dd MULTIBOOT_HEADER_MAGIC
    dd MULTIBOOT_HEADER_FLAGS
    dd MULTIBOOT_CHECKSUM

