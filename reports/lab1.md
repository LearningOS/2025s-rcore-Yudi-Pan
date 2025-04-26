# Lab-1-潘宇迪

## 实现功能

引入了一个新的系统调用 ``sys_trace``（ID 为 410）用来追踪当前任务系统调用的历史信息。
事实上，系统会用一个二维静态数组，存储所有任务的所有系统调用的历史信息。
具体保存在TaskManagerInner中，并暴露find_trace_info方法。
同时，在syscall函数入口处设置代码，检查并增加系统调用count。系统调用sys_trace只负责读取数据并返回值。

## 问答题

一.

SBI:
RustSBI version 0.3.0-alpha.4, adapting to RISC-V SBI v1.0.0
RustSBI-QEMU Version 0.2.0-alpha.2

- ch2b_bad_address: PageFault in application, kernel killed it.
访问了不存在的地址，出现了页表错误
- ch2b_bad_instructions: IllegalInstruction in application, kernel killed it.
使用了S特权级的指令，出现非法指令错误。
- ch2b_bad_register: IllegalInstruction in application, kernel killed it.
使用了S特权级的指令访问了受保护的寄存器，出现非法指令错误。
二.
1.刚进入 __restore 时，sp 代表了内核栈(分配过TrapContext的)，sp始终都会指向当前执行的程序流的栈，除了发生跳转之前会改变。

使用情景：

- 被run_next_app或goto_restore函数调用，用来启动应用程序。
- 当 trap_handler 返回之后，使用__restore从trap中返回。

2.从内核栈顶的 Trap 上下文恢复CSR，因为不能直接从内存写入CSR，只能先读到通用寄存器，再写入CSR。这三个寄存器分别是：

- sstatus的SPP等字段给出 Trap 发生之前 CPU 处在哪个特权级（S/U）等信息。
- sepc 当 Trap 是一个异常的时候，记录 Trap 发生之前执行的最后一条指令的地址。
- sscratch 作为中转寄存器保存着用户栈地址。

3.因为 x2不在这里恢复，而x4不需要恢复，应用程序用不到。

4.该指令之后，sp中保存着用户栈地址，sscratch保存着内核栈地址。

5.状态切换发生在sret指令，sret是 RISC-V 特权指令，硬件会检查sstatus.SPP 位，若 SPP=0，则返回到​用户态​（U-Mode），并将 sepc 的值载入 PC，继续执行用户态代码。

6.该指令之后，sp中保存着内核栈地址，sscratch保存着用户栈地址。

7.因为stvec中保存着__alltraps的地址，因此从指令 csrrw sp, sscratch, sp开始都运行在S特权级。

## 荣誉准则

1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与以下各位就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：
腾讯元宝，但未使用其提供的代码。
2. 此外，我也参考了以下资料 ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：
rCore-Tutorial-Guide-2025S文档
3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。
4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。

## 看法

比较简单，代码量和思维量不大，受益于Rust编译器，几乎不需要看log或debug。
