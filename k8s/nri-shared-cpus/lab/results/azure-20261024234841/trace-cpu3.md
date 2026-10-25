# CPU 3: what ran on it, interrupted it or was sent to it

| item | value |
|---|---|
| spin thread | tid 18949 (host) |
| window (CLOCK_REALTIME ns) | 1792886387714518683 to 1792886447716166008 (60 s) |
| clock | trace `mono` + 1792885855068786764 ns = CLOCK_REALTIME (0 ns of drift by the end); the text has µs resolution, events are matched to gaps within 10.00 µs |
| events enabled | sched:sched_switch if CPU == 3, sched:sched_waking if target_cpu == 3, with the sender's stack, sched:sched_wakeup if target_cpu == 3, with the sender's stack, sched:sched_wakeup_new if target_cpu == 3, with the sender's stack, sched:sched_migrate_task if orig_cpu == 3 || dest_cpu == 3, with the sender's stack, ipi:ipi_send_cpu if cpu == 3, with the sender's stack, ipi:ipi_send_cpumask if cpumask & CPUS{3}, with the sender's stack, ipi:ipi_entry if CPU == 3, csd:csd_queue_cpu if cpu == 3, with the sender's stack, csd:csd_function_entry if CPU == 3, irq:irq_handler_entry if CPU == 3, irq:softirq_entry if CPU == 3, timer:hrtimer_expire_entry if CPU == 3, timer:timer_expire_entry if CPU == 3, timer:tick_stop if CPU == 3, workqueue:workqueue_queue_work if cpu == 3, with the sender's stack, workqueue:workqueue_execute_start if CPU == 3 |
| events this kernel lacks | irq_vectors:local_timer_entry, irq_vectors:reschedule_entry, irq_vectors:call_function_entry, irq_vectors:call_function_single_entry |
| events lost | none |
| involuntary switches of the spin thread | 1 in the trace; 1 from /proc nonvoluntary_ctxt_switches |
| voluntary switches | 0 |

## Switches away from the spin thread

| at (CLOCK_REALTIME ns) | kind | prev_state | ran instead (comm/pid) | work it ran | off CPU µs | woken by (comm/pid on CPU) | waker's stack | gap µs |
|---|---|---|---|---|---|---|---|---|
| 1792886409985100764 | involuntary | R | kworker/3:0/39 | lru_add_drain_per_cpu, wq_barrier_func | 20.00 | systemd-gpt-aut/19411 on CPU 2 (sched_wakeup) | ttwu_do_activate < try_to_wake_up < wake_up_process < kick_pool < __queue_work < queue_work_on | 40.60 |

## IPIs and cross-calls sent to CPU 3

| at (CLOCK_REALTIME ns) | event | from CPU | sender (comm/pid) | callsite | callback | sender's stack |
|---|---|---|---|---|---|---|
| 1792886409985078764 | csd_queue_cpu | 2 | systemd-gpt-aut/19411 | irq_work_queue_on | nohz_full_kick_func | __smp_call_single_queue < irq_work_queue_on < tick_nohz_full_kick_cpu < tick_nohz_dep_set_cpu < enqueue_task_fair < enqueue_task < ttwu_do_activate < try_to_wake_up |
| 1792886409985079764 | ipi_send_cpu | 2 | systemd-gpt-aut/19411 | irq_work_queue_on | generic_smp_call_function_single_interrupt | __smp_call_single_queue < irq_work_queue_on < tick_nohz_full_kick_cpu < tick_nohz_dep_set_cpu < enqueue_task_fair < enqueue_task < ttwu_do_activate < try_to_wake_up |
| 1792886409985081764 | ipi_send_cpu | 2 | systemd-gpt-aut/19411 | check_preempt_wakeup_fair | 0x0 | resched_curr < check_preempt_wakeup_fair < wakeup_preempt < ttwu_do_activate < try_to_wake_up < wake_up_process < kick_pool < __queue_work |
| 1792886410108180764 | csd_queue_cpu | 0 | systemd/1 | kick_all_cpus_sync | do_nothing | smp_call_function_many_cond < kick_all_cpus_sync < __text_poke < aarch64_insn_copy < bpf_arch_text_copy < bpf_jit_binary_pack_finalize < bpf_int_jit_compile < bpf_prog_select_runtime |
| 1792886410108185764 | ipi_send_cpumask | 0 | systemd/1 | kick_all_cpus_sync | generic_smp_call_function_single_interrupt | smp_call_function_many_cond < kick_all_cpus_sync < __text_poke < aarch64_insn_copy < bpf_arch_text_copy < bpf_jit_binary_pack_finalize < bpf_int_jit_compile < bpf_prog_select_runtime |
| 1792886410108306764 | csd_queue_cpu | 0 | systemd/1 | kick_all_cpus_sync | do_nothing | smp_call_function_many_cond < kick_all_cpus_sync < bpf_int_jit_compile < bpf_prog_select_runtime < bpf_prog_load < __sys_bpf < __arm64_sys_bpf < invoke_syscall |
| 1792886410108308764 | ipi_send_cpumask | 0 | systemd/1 | kick_all_cpus_sync | generic_smp_call_function_single_interrupt | smp_call_function_many_cond < kick_all_cpus_sync < bpf_int_jit_compile < bpf_prog_select_runtime < bpf_prog_load < __sys_bpf < __arm64_sys_bpf < invoke_syscall |
| 1792886410108659764 | csd_queue_cpu | 0 | systemd/1 | kick_all_cpus_sync | do_nothing | smp_call_function_many_cond < kick_all_cpus_sync < __text_poke < aarch64_insn_copy < bpf_arch_text_copy < bpf_jit_binary_pack_finalize < bpf_int_jit_compile < bpf_prog_select_runtime |
| 1792886410108662764 | ipi_send_cpumask | 0 | systemd/1 | kick_all_cpus_sync | generic_smp_call_function_single_interrupt | smp_call_function_many_cond < kick_all_cpus_sync < __text_poke < aarch64_insn_copy < bpf_arch_text_copy < bpf_jit_binary_pack_finalize < bpf_int_jit_compile < bpf_prog_select_runtime |
| 1792886410108748764 | csd_queue_cpu | 0 | systemd/1 | kick_all_cpus_sync | do_nothing | smp_call_function_many_cond < kick_all_cpus_sync < bpf_int_jit_compile < bpf_prog_select_runtime < bpf_prog_load < __sys_bpf < __arm64_sys_bpf < invoke_syscall |
| 1792886410108750764 | ipi_send_cpumask | 0 | systemd/1 | kick_all_cpus_sync | generic_smp_call_function_single_interrupt | smp_call_function_many_cond < kick_all_cpus_sync < bpf_int_jit_compile < bpf_prog_select_runtime < bpf_prog_load < __sys_bpf < __arm64_sys_bpf < invoke_syscall |
| 1792886410108785764 | csd_queue_cpu | 0 | systemd/1 | kick_all_cpus_sync | do_nothing | smp_call_function_many_cond < kick_all_cpus_sync < __text_poke < aarch64_insn_copy < bpf_arch_text_copy < bpf_jit_binary_pack_finalize < bpf_int_jit_compile < bpf_prog_select_runtime |
| 1792886410108804764 | ipi_send_cpumask | 0 | systemd/1 | kick_all_cpus_sync | generic_smp_call_function_single_interrupt | smp_call_function_many_cond < kick_all_cpus_sync < __text_poke < aarch64_insn_copy < bpf_arch_text_copy < bpf_jit_binary_pack_finalize < bpf_int_jit_compile < bpf_prog_select_runtime |
| 1792886410108815764 | csd_queue_cpu | 0 | systemd/1 | kick_all_cpus_sync | do_nothing | smp_call_function_many_cond < kick_all_cpus_sync < bpf_int_jit_compile < bpf_prog_select_runtime < bpf_prog_load < __sys_bpf < __arm64_sys_bpf < invoke_syscall |
| 1792886410108816764 | ipi_send_cpumask | 0 | systemd/1 | kick_all_cpus_sync | generic_smp_call_function_single_interrupt | smp_call_function_many_cond < kick_all_cpus_sync < bpf_int_jit_compile < bpf_prog_select_runtime < bpf_prog_load < __sys_bpf < __arm64_sys_bpf < invoke_syscall |
| 1792886410109869764 | csd_queue_cpu | 0 | systemd/1 | kick_all_cpus_sync | do_nothing | smp_call_function_many_cond < kick_all_cpus_sync < __text_poke < aarch64_insn_copy < bpf_arch_text_copy < bpf_jit_binary_pack_finalize < bpf_int_jit_compile < bpf_prog_select_runtime |
| 1792886410109872764 | ipi_send_cpumask | 0 | systemd/1 | kick_all_cpus_sync | generic_smp_call_function_single_interrupt | smp_call_function_many_cond < kick_all_cpus_sync < __text_poke < aarch64_insn_copy < bpf_arch_text_copy < bpf_jit_binary_pack_finalize < bpf_int_jit_compile < bpf_prog_select_runtime |
| 1792886410109889764 | csd_queue_cpu | 0 | systemd/1 | kick_all_cpus_sync | do_nothing | smp_call_function_many_cond < kick_all_cpus_sync < bpf_int_jit_compile < bpf_prog_select_runtime < bpf_prog_load < __sys_bpf < __arm64_sys_bpf < invoke_syscall |
| 1792886410109891764 | ipi_send_cpumask | 0 | systemd/1 | kick_all_cpus_sync | generic_smp_call_function_single_interrupt | smp_call_function_many_cond < kick_all_cpus_sync < bpf_int_jit_compile < bpf_prog_select_runtime < bpf_prog_load < __sys_bpf < __arm64_sys_bpf < invoke_syscall |
| 1792886410109977764 | csd_queue_cpu | 0 | systemd/1 | kick_all_cpus_sync | do_nothing | smp_call_function_many_cond < kick_all_cpus_sync < __text_poke < aarch64_insn_copy < bpf_arch_text_copy < bpf_jit_binary_pack_finalize < bpf_int_jit_compile < bpf_prog_select_runtime |
| 1792886410109979764 | ipi_send_cpumask | 0 | systemd/1 | kick_all_cpus_sync | generic_smp_call_function_single_interrupt | smp_call_function_many_cond < kick_all_cpus_sync < __text_poke < aarch64_insn_copy < bpf_arch_text_copy < bpf_jit_binary_pack_finalize < bpf_int_jit_compile < bpf_prog_select_runtime |
| 1792886410109990764 | csd_queue_cpu | 0 | systemd/1 | kick_all_cpus_sync | do_nothing | smp_call_function_many_cond < kick_all_cpus_sync < bpf_int_jit_compile < bpf_prog_select_runtime < bpf_prog_load < __sys_bpf < __arm64_sys_bpf < invoke_syscall |
| 1792886410109991764 | ipi_send_cpumask | 0 | systemd/1 | kick_all_cpus_sync | generic_smp_call_function_single_interrupt | smp_call_function_many_cond < kick_all_cpus_sync < bpf_int_jit_compile < bpf_prog_select_runtime < bpf_prog_load < __sys_bpf < __arm64_sys_bpf < invoke_syscall |
| 1792886410110019764 | csd_queue_cpu | 0 | systemd/1 | kick_all_cpus_sync | do_nothing | smp_call_function_many_cond < kick_all_cpus_sync < __text_poke < aarch64_insn_copy < bpf_arch_text_copy < bpf_jit_binary_pack_finalize < bpf_int_jit_compile < bpf_prog_select_runtime |
| 1792886410110021764 | ipi_send_cpumask | 0 | systemd/1 | kick_all_cpus_sync | generic_smp_call_function_single_interrupt | smp_call_function_many_cond < kick_all_cpus_sync < __text_poke < aarch64_insn_copy < bpf_arch_text_copy < bpf_jit_binary_pack_finalize < bpf_int_jit_compile < bpf_prog_select_runtime |
| 1792886410110032764 | csd_queue_cpu | 0 | systemd/1 | kick_all_cpus_sync | do_nothing | smp_call_function_many_cond < kick_all_cpus_sync < bpf_int_jit_compile < bpf_prog_select_runtime < bpf_prog_load < __sys_bpf < __arm64_sys_bpf < invoke_syscall |
| 1792886410110033764 | ipi_send_cpumask | 0 | systemd/1 | kick_all_cpus_sync | generic_smp_call_function_single_interrupt | smp_call_function_many_cond < kick_all_cpus_sync < bpf_int_jit_compile < bpf_prog_select_runtime < bpf_prog_load < __sys_bpf < __arm64_sys_bpf < invoke_syscall |
| 1792886410110616764 | csd_queue_cpu | 0 | systemd/1 | kick_all_cpus_sync | do_nothing | smp_call_function_many_cond < kick_all_cpus_sync < __text_poke < aarch64_insn_copy < bpf_arch_text_copy < bpf_jit_binary_pack_finalize < bpf_int_jit_compile < bpf_prog_select_runtime |
| 1792886410110619764 | ipi_send_cpumask | 0 | systemd/1 | kick_all_cpus_sync | generic_smp_call_function_single_interrupt | smp_call_function_many_cond < kick_all_cpus_sync < __text_poke < aarch64_insn_copy < bpf_arch_text_copy < bpf_jit_binary_pack_finalize < bpf_int_jit_compile < bpf_prog_select_runtime |
| 1792886410110632764 | csd_queue_cpu | 0 | systemd/1 | kick_all_cpus_sync | do_nothing | smp_call_function_many_cond < kick_all_cpus_sync < bpf_int_jit_compile < bpf_prog_select_runtime < bpf_prog_load < __sys_bpf < __arm64_sys_bpf < invoke_syscall |
| 1792886410110634764 | ipi_send_cpumask | 0 | systemd/1 | kick_all_cpus_sync | generic_smp_call_function_single_interrupt | smp_call_function_many_cond < kick_all_cpus_sync < bpf_int_jit_compile < bpf_prog_select_runtime < bpf_prog_load < __sys_bpf < __arm64_sys_bpf < invoke_syscall |
| 1792886410110709764 | csd_queue_cpu | 0 | systemd/1 | kick_all_cpus_sync | do_nothing | smp_call_function_many_cond < kick_all_cpus_sync < __text_poke < aarch64_insn_copy < bpf_arch_text_copy < bpf_jit_binary_pack_finalize < bpf_int_jit_compile < bpf_prog_select_runtime |
| 1792886410110711764 | ipi_send_cpumask | 0 | systemd/1 | kick_all_cpus_sync | generic_smp_call_function_single_interrupt | smp_call_function_many_cond < kick_all_cpus_sync < __text_poke < aarch64_insn_copy < bpf_arch_text_copy < bpf_jit_binary_pack_finalize < bpf_int_jit_compile < bpf_prog_select_runtime |
| 1792886410110722764 | csd_queue_cpu | 0 | systemd/1 | kick_all_cpus_sync | do_nothing | smp_call_function_many_cond < kick_all_cpus_sync < bpf_int_jit_compile < bpf_prog_select_runtime < bpf_prog_load < __sys_bpf < __arm64_sys_bpf < invoke_syscall |
| 1792886410110723764 | ipi_send_cpumask | 0 | systemd/1 | kick_all_cpus_sync | generic_smp_call_function_single_interrupt | smp_call_function_many_cond < kick_all_cpus_sync < bpf_int_jit_compile < bpf_prog_select_runtime < bpf_prog_load < __sys_bpf < __arm64_sys_bpf < invoke_syscall |
| 1792886410110751764 | csd_queue_cpu | 0 | systemd/1 | kick_all_cpus_sync | do_nothing | smp_call_function_many_cond < kick_all_cpus_sync < __text_poke < aarch64_insn_copy < bpf_arch_text_copy < bpf_jit_binary_pack_finalize < bpf_int_jit_compile < bpf_prog_select_runtime |
| 1792886410110752764 | ipi_send_cpumask | 0 | systemd/1 | kick_all_cpus_sync | generic_smp_call_function_single_interrupt | smp_call_function_many_cond < kick_all_cpus_sync < __text_poke < aarch64_insn_copy < bpf_arch_text_copy < bpf_jit_binary_pack_finalize < bpf_int_jit_compile < bpf_prog_select_runtime |
| 1792886410110763764 | csd_queue_cpu | 0 | systemd/1 | kick_all_cpus_sync | do_nothing | smp_call_function_many_cond < kick_all_cpus_sync < bpf_int_jit_compile < bpf_prog_select_runtime < bpf_prog_load < __sys_bpf < __arm64_sys_bpf < invoke_syscall |
| 1792886410110764764 | ipi_send_cpumask | 0 | systemd/1 | kick_all_cpus_sync | generic_smp_call_function_single_interrupt | smp_call_function_many_cond < kick_all_cpus_sync < bpf_int_jit_compile < bpf_prog_select_runtime < bpf_prog_load < __sys_bpf < __arm64_sys_bpf < invoke_syscall |
| 1792886410129488764 | csd_queue_cpu | 2 | systemd/1 | kick_all_cpus_sync | do_nothing | smp_call_function_many_cond < kick_all_cpus_sync < __text_poke < aarch64_insn_copy < bpf_arch_text_copy < bpf_jit_binary_pack_finalize < bpf_int_jit_compile < bpf_prog_select_runtime |
| 1792886410129494764 | ipi_send_cpumask | 2 | systemd/1 | kick_all_cpus_sync | generic_smp_call_function_single_interrupt | smp_call_function_many_cond < kick_all_cpus_sync < __text_poke < aarch64_insn_copy < bpf_arch_text_copy < bpf_jit_binary_pack_finalize < bpf_int_jit_compile < bpf_prog_select_runtime |
| 1792886410129521764 | csd_queue_cpu | 2 | systemd/1 | kick_all_cpus_sync | do_nothing | smp_call_function_many_cond < kick_all_cpus_sync < bpf_int_jit_compile < bpf_prog_select_runtime < bpf_prog_load < __sys_bpf < __arm64_sys_bpf < invoke_syscall |
| 1792886410129522764 | ipi_send_cpumask | 2 | systemd/1 | kick_all_cpus_sync | generic_smp_call_function_single_interrupt | smp_call_function_many_cond < kick_all_cpus_sync < bpf_int_jit_compile < bpf_prog_select_runtime < bpf_prog_load < __sys_bpf < __arm64_sys_bpf < invoke_syscall |
| 1792886410130412764 | csd_queue_cpu | 2 | systemd/1 | kick_all_cpus_sync | do_nothing | smp_call_function_many_cond < kick_all_cpus_sync < __text_poke < aarch64_insn_copy < bpf_arch_text_copy < bpf_jit_binary_pack_finalize < bpf_int_jit_compile < bpf_prog_select_runtime |
| 1792886410130416764 | ipi_send_cpumask | 2 | systemd/1 | kick_all_cpus_sync | generic_smp_call_function_single_interrupt | smp_call_function_many_cond < kick_all_cpus_sync < __text_poke < aarch64_insn_copy < bpf_arch_text_copy < bpf_jit_binary_pack_finalize < bpf_int_jit_compile < bpf_prog_select_runtime |
| 1792886410130437764 | csd_queue_cpu | 2 | systemd/1 | kick_all_cpus_sync | do_nothing | smp_call_function_many_cond < kick_all_cpus_sync < bpf_int_jit_compile < bpf_prog_select_runtime < bpf_prog_load < __sys_bpf < __arm64_sys_bpf < invoke_syscall |
| 1792886410130438764 | ipi_send_cpumask | 2 | systemd/1 | kick_all_cpus_sync | generic_smp_call_function_single_interrupt | smp_call_function_many_cond < kick_all_cpus_sync < bpf_int_jit_compile < bpf_prog_select_runtime < bpf_prog_load < __sys_bpf < __arm64_sys_bpf < invoke_syscall |
| 1792886410130990764 | csd_queue_cpu | 2 | systemd/1 | kick_all_cpus_sync | do_nothing | smp_call_function_many_cond < kick_all_cpus_sync < __text_poke < aarch64_insn_copy < bpf_arch_text_copy < bpf_jit_binary_pack_finalize < bpf_int_jit_compile < bpf_prog_select_runtime |
| 1792886410130992764 | ipi_send_cpumask | 2 | systemd/1 | kick_all_cpus_sync | generic_smp_call_function_single_interrupt | smp_call_function_many_cond < kick_all_cpus_sync < __text_poke < aarch64_insn_copy < bpf_arch_text_copy < bpf_jit_binary_pack_finalize < bpf_int_jit_compile < bpf_prog_select_runtime |
| 1792886410131009764 | csd_queue_cpu | 2 | systemd/1 | kick_all_cpus_sync | do_nothing | smp_call_function_many_cond < kick_all_cpus_sync < bpf_int_jit_compile < bpf_prog_select_runtime < bpf_prog_load < __sys_bpf < __arm64_sys_bpf < invoke_syscall |
| 1792886410131010764 | ipi_send_cpumask | 2 | systemd/1 | kick_all_cpus_sync | generic_smp_call_function_single_interrupt | smp_call_function_many_cond < kick_all_cpus_sync < bpf_int_jit_compile < bpf_prog_select_runtime < bpf_prog_load < __sys_bpf < __arm64_sys_bpf < invoke_syscall |
| 1792886410132989764 | csd_queue_cpu | 0 | kworker/0:1/11 | kick_all_cpus_sync | do_nothing | smp_call_function_many_cond < kick_all_cpus_sync < __text_poke < aarch64_insn_set < bpf_arch_text_invalidate < bpf_prog_pack_free < bpf_jit_binary_pack_free < bpf_jit_free |
| 1792886410132993764 | ipi_send_cpumask | 0 | kworker/0:1/11 | kick_all_cpus_sync | generic_smp_call_function_single_interrupt | smp_call_function_many_cond < kick_all_cpus_sync < __text_poke < aarch64_insn_set < bpf_arch_text_invalidate < bpf_prog_pack_free < bpf_jit_binary_pack_free < bpf_jit_free |
| 1792886410136854764 | csd_queue_cpu | 0 | kworker/0:1/11 | kick_all_cpus_sync | do_nothing | smp_call_function_many_cond < kick_all_cpus_sync < __text_poke < aarch64_insn_set < bpf_arch_text_invalidate < bpf_prog_pack_free < bpf_jit_binary_pack_free < bpf_jit_free |
| 1792886410136858764 | ipi_send_cpumask | 0 | kworker/0:1/11 | kick_all_cpus_sync | generic_smp_call_function_single_interrupt | smp_call_function_many_cond < kick_all_cpus_sync < __text_poke < aarch64_insn_set < bpf_arch_text_invalidate < bpf_prog_pack_free < bpf_jit_binary_pack_free < bpf_jit_free |
| 1792886410136884764 | csd_queue_cpu | 0 | kworker/0:1/11 | kick_all_cpus_sync | do_nothing | smp_call_function_many_cond < kick_all_cpus_sync < __text_poke < aarch64_insn_set < bpf_arch_text_invalidate < bpf_prog_pack_free < bpf_jit_binary_pack_free < bpf_jit_free |
| 1792886410136886764 | ipi_send_cpumask | 0 | kworker/0:1/11 | kick_all_cpus_sync | generic_smp_call_function_single_interrupt | smp_call_function_many_cond < kick_all_cpus_sync < __text_poke < aarch64_insn_set < bpf_arch_text_invalidate < bpf_prog_pack_free < bpf_jit_binary_pack_free < bpf_jit_free |
| 1792886410136903764 | csd_queue_cpu | 0 | kworker/0:1/11 | kick_all_cpus_sync | do_nothing | smp_call_function_many_cond < kick_all_cpus_sync < __text_poke < aarch64_insn_set < bpf_arch_text_invalidate < bpf_prog_pack_free < bpf_jit_binary_pack_free < bpf_jit_free |
| 1792886410136904764 | ipi_send_cpumask | 0 | kworker/0:1/11 | kick_all_cpus_sync | generic_smp_call_function_single_interrupt | smp_call_function_many_cond < kick_all_cpus_sync < __text_poke < aarch64_insn_set < bpf_arch_text_invalidate < bpf_prog_pack_free < bpf_jit_binary_pack_free < bpf_jit_free |
| 1792886410136923764 | csd_queue_cpu | 0 | kworker/0:1/11 | kick_all_cpus_sync | do_nothing | smp_call_function_many_cond < kick_all_cpus_sync < __text_poke < aarch64_insn_set < bpf_arch_text_invalidate < bpf_prog_pack_free < bpf_jit_binary_pack_free < bpf_jit_free |
| 1792886410136924764 | ipi_send_cpumask | 0 | kworker/0:1/11 | kick_all_cpus_sync | generic_smp_call_function_single_interrupt | smp_call_function_many_cond < kick_all_cpus_sync < __text_poke < aarch64_insn_set < bpf_arch_text_invalidate < bpf_prog_pack_free < bpf_jit_binary_pack_free < bpf_jit_free |
| 1792886410136941764 | csd_queue_cpu | 0 | kworker/0:1/11 | kick_all_cpus_sync | do_nothing | smp_call_function_many_cond < kick_all_cpus_sync < __text_poke < aarch64_insn_set < bpf_arch_text_invalidate < bpf_prog_pack_free < bpf_jit_binary_pack_free < bpf_jit_free |
| 1792886410136942764 | ipi_send_cpumask | 0 | kworker/0:1/11 | kick_all_cpus_sync | generic_smp_call_function_single_interrupt | smp_call_function_many_cond < kick_all_cpus_sync < __text_poke < aarch64_insn_set < bpf_arch_text_invalidate < bpf_prog_pack_free < bpf_jit_binary_pack_free < bpf_jit_free |
| 1792886410136957764 | csd_queue_cpu | 0 | kworker/0:1/11 | kick_all_cpus_sync | do_nothing | smp_call_function_many_cond < kick_all_cpus_sync < __text_poke < aarch64_insn_set < bpf_arch_text_invalidate < bpf_prog_pack_free < bpf_jit_binary_pack_free < bpf_jit_free |
| 1792886410136958764 | ipi_send_cpumask | 0 | kworker/0:1/11 | kick_all_cpus_sync | generic_smp_call_function_single_interrupt | smp_call_function_many_cond < kick_all_cpus_sync < __text_poke < aarch64_insn_set < bpf_arch_text_invalidate < bpf_prog_pack_free < bpf_jit_binary_pack_free < bpf_jit_free |
| 1792886410136973764 | csd_queue_cpu | 0 | kworker/0:1/11 | kick_all_cpus_sync | do_nothing | smp_call_function_many_cond < kick_all_cpus_sync < __text_poke < aarch64_insn_set < bpf_arch_text_invalidate < bpf_prog_pack_free < bpf_jit_binary_pack_free < bpf_jit_free |
| 1792886410136974764 | ipi_send_cpumask | 0 | kworker/0:1/11 | kick_all_cpus_sync | generic_smp_call_function_single_interrupt | smp_call_function_many_cond < kick_all_cpus_sync < __text_poke < aarch64_insn_set < bpf_arch_text_invalidate < bpf_prog_pack_free < bpf_jit_binary_pack_free < bpf_jit_free |
| 1792886410136988764 | csd_queue_cpu | 0 | kworker/0:1/11 | kick_all_cpus_sync | do_nothing | smp_call_function_many_cond < kick_all_cpus_sync < __text_poke < aarch64_insn_set < bpf_arch_text_invalidate < bpf_prog_pack_free < bpf_jit_binary_pack_free < bpf_jit_free |
| 1792886410136989764 | ipi_send_cpumask | 0 | kworker/0:1/11 | kick_all_cpus_sync | generic_smp_call_function_single_interrupt | smp_call_function_many_cond < kick_all_cpus_sync < __text_poke < aarch64_insn_set < bpf_arch_text_invalidate < bpf_prog_pack_free < bpf_jit_binary_pack_free < bpf_jit_free |
| 1792886410144849764 | csd_queue_cpu | 2 | kworker/2:3/392 | kick_all_cpus_sync | do_nothing | smp_call_function_many_cond < kick_all_cpus_sync < __text_poke < aarch64_insn_set < bpf_arch_text_invalidate < bpf_prog_pack_free < bpf_jit_binary_pack_free < bpf_jit_free |
| 1792886410144853764 | ipi_send_cpumask | 2 | kworker/2:3/392 | kick_all_cpus_sync | generic_smp_call_function_single_interrupt | smp_call_function_many_cond < kick_all_cpus_sync < __text_poke < aarch64_insn_set < bpf_arch_text_invalidate < bpf_prog_pack_free < bpf_jit_binary_pack_free < bpf_jit_free |
| 1792886410144884764 | csd_queue_cpu | 2 | kworker/2:3/392 | kick_all_cpus_sync | do_nothing | smp_call_function_many_cond < kick_all_cpus_sync < __text_poke < aarch64_insn_set < bpf_arch_text_invalidate < bpf_prog_pack_free < bpf_jit_binary_pack_free < bpf_jit_free |
| 1792886410144886764 | ipi_send_cpumask | 2 | kworker/2:3/392 | kick_all_cpus_sync | generic_smp_call_function_single_interrupt | smp_call_function_many_cond < kick_all_cpus_sync < __text_poke < aarch64_insn_set < bpf_arch_text_invalidate < bpf_prog_pack_free < bpf_jit_binary_pack_free < bpf_jit_free |
| 1792886410144906764 | csd_queue_cpu | 2 | kworker/2:3/392 | kick_all_cpus_sync | do_nothing | smp_call_function_many_cond < kick_all_cpus_sync < __text_poke < aarch64_insn_set < bpf_arch_text_invalidate < bpf_prog_pack_free < bpf_jit_binary_pack_free < bpf_jit_free |
| 1792886410144907764 | ipi_send_cpumask | 2 | kworker/2:3/392 | kick_all_cpus_sync | generic_smp_call_function_single_interrupt | smp_call_function_many_cond < kick_all_cpus_sync < __text_poke < aarch64_insn_set < bpf_arch_text_invalidate < bpf_prog_pack_free < bpf_jit_binary_pack_free < bpf_jit_free |

By sender path:

- 24 x csd_queue_cpu kick_all_cpus_sync from smp_call_function_many_cond < kick_all_cpus_sync < __text_poke
- 24 x ipi_send_cpumask kick_all_cpus_sync from smp_call_function_many_cond < kick_all_cpus_sync < __text_poke
- 12 x csd_queue_cpu kick_all_cpus_sync from smp_call_function_many_cond < kick_all_cpus_sync < bpf_int_jit_compile
- 12 x ipi_send_cpumask kick_all_cpus_sync from smp_call_function_many_cond < kick_all_cpus_sync < bpf_int_jit_compile
- 1 x csd_queue_cpu irq_work_queue_on from __smp_call_single_queue < irq_work_queue_on < tick_nohz_full_kick_cpu
- 1 x ipi_send_cpu check_preempt_wakeup_fair from resched_curr < check_preempt_wakeup_fair < wakeup_preempt
- 1 x ipi_send_cpu irq_work_queue_on from __smp_call_single_queue < irq_work_queue_on < tick_nohz_full_kick_cpu

## Work items run on CPU 3

| at (CLOCK_REALTIME ns) | function | worker | queued by (comm/pid on CPU) | queuer's stack |
|---|---|---|---|---|
| 1792886409985103764 | lru_add_drain_per_cpu | kworker/3:0/39 | systemd-gpt-aut/19411 on CPU 2 | __queue_work < queue_work_on < __lru_add_drain_all < lru_add_drain_all < invalidate_bdev < blkdev_common_ioctl |
| 1792886409985106764 | wq_barrier_func | kworker/3:0/39 | not in the trace | - |

## Timers, ticks and interrupts on CPU 3

| event | count |
|---|---|
| ipi Function call interrupts | 37 |
| irq IPI (2) | 37 |
| cross-call do_nothing, | 36 |
| tick_stop success=0 dependency=SCHED | 3 |
| hrtimer tick_nohz_handler | 2 |
| irq arch_timer (10) | 2 |
| ipi Rescheduling interrupts | 1 |
| irq IPI (1) | 1 |
| tick_stop success=1 dependency=NONE | 1 |

## Gaps of 3.00 µs or more, with what overlapped each

| gap ended (CLOCK_REALTIME ns) | gap µs | on CPU 3 or sent to it, within the gap |
|---|---|---|
| 1792886387715573139 | 5.13 | nothing in the guest's trace: below the kernel (hypervisor or hardware) |
| 1792886387949084570 | 3.48 | nothing in the guest's trace: below the kernel (hypervisor or hardware) |
| 1792886409985124735 | 40.60 (the max) | irq IPI (2); ipi Function call interrupts; tick_stop success=0 dependency=SCHED; irq IPI (1); ipi Rescheduling interrupts; switch gp-hot -> kworker/3:0 (woken by systemd-gpt-aut/19411 on CPU 2); work lru_add_drain_per_cpu; work wq_barrier_func; switch kworker/3:0 -> gp-hot; csd_queue_cpu from CPU 2 systemd-gpt-aut/19411 via irq_work_queue_on; ipi_send_cpu from CPU 2 systemd-gpt-aut/19411 via irq_work_queue_on; ipi_send_cpu from CPU 2 systemd-gpt-aut/19411 via check_preempt_wakeup_fair |
| 1792886409988799571 | 9.53 | irq arch_timer (10); hrtimer tick_nohz_handler; tick_stop success=0 dependency=SCHED |
| 1792886409992796015 | 5.97 | irq arch_timer (10); hrtimer tick_nohz_handler; tick_stop success=1 dependency=NONE |
| 1792886410108205838 | 3.60 | irq IPI (2); ipi Function call interrupts; cross-call do_nothing, |

6 gaps listed, 2 with no kernel event on or sent to CPU 3.
