  .text
  .intel_syntax noprefix
  .globl main
main:
.LBL0:
  push rbp
  mov rbp, rsp
  sub rsp, 16
  mov dword ptr [rbp-4], 2
  jmp .LBL1
.LBL1:
  mov eax, dword ptr [rbp-4]
  add rsp, 16
  pop rbp
  ret 
