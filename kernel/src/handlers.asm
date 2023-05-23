global interrupt_handlers

extern generic_interrupt_handler

%macro make_interrupt_handler 2
[global interrupt_handler_%1]
interrupt_handler_%1:
%if %2 == 0
    push 0
%endif

    test qword [rsp + 16], 0x3
    jz .dont_swapgs
    swapgs
    .dont_swapgs:

    xchg [rsp], rax

    push rbx
    push rcx
    push rdx
    push rbp
    push rsi
    push rdi
    push r8
    push r9
    push r10
    push r11
    push r12
    push r13
    push r14
    push r15
    push rax

    mov rdi, %1
    mov rsi, rsp

    ; jmp $

    ; call the generic interrupt handler
    call generic_interrupt_handler

    ; pop the error code
    add rsp, 8

    pop r15
    pop r14
    pop r13
    pop r12
    pop r11
    pop r10
    pop r9
    pop r8
    pop rdi
    pop rsi
    pop rbp
    pop rdx
    pop rcx
    pop rbx
    pop rax

    test qword [rsp + 8], 0x3
    ; skip the SWAPGS instruction if CS & 0b11 == 0b00.
    jz .dont_swapgs_again
    swapgs
    .dont_swapgs_again:
    
    ; voila! we're done!
    iretq
%endmacro

%macro interrupt_handler_no_error_code 1
    make_interrupt_handler %1, 0
%endmacro

%macro interrupt_handler_error_code 1
    make_interrupt_handler %1, 1
%endmacro

interrupt_handler_no_error_code 0
interrupt_handler_no_error_code 1
interrupt_handler_no_error_code 2
interrupt_handler_no_error_code 3
interrupt_handler_no_error_code 4
interrupt_handler_no_error_code 5
interrupt_handler_no_error_code 6
interrupt_handler_no_error_code 7

interrupt_handler_error_code 8
interrupt_handler_no_error_code 9
interrupt_handler_error_code 10
interrupt_handler_error_code 11
interrupt_handler_error_code 12
interrupt_handler_error_code 13
interrupt_handler_error_code 14
interrupt_handler_no_error_code 15

interrupt_handler_no_error_code 16

interrupt_handler_error_code 17

interrupt_handler_no_error_code 18
interrupt_handler_no_error_code 19
interrupt_handler_no_error_code 20
interrupt_handler_no_error_code 21
interrupt_handler_no_error_code 22
interrupt_handler_no_error_code 23
interrupt_handler_no_error_code 24
interrupt_handler_no_error_code 25
interrupt_handler_no_error_code 26
interrupt_handler_no_error_code 27
interrupt_handler_no_error_code 28
interrupt_handler_no_error_code 29

interrupt_handler_error_code 30

interrupt_handler_no_error_code 31

%assign i 32
%rep 224
    interrupt_handler_no_error_code i
%assign i i + 1
%endrep
