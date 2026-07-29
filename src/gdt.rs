use x86_64::VirtAddr;
use x86_64::structures::tss::TaskStateSegment;
use lazy_static::lazy_static;


pub const DOUBLE_FAULT8IST_INDEX: u16 = 0;

lazy_static! {
    static ref TSS: TaskStateSegment = {
        let mut tss = TaskStateSegment::new();
        tss.interrupt_stack_table[DOUBLE_FAULT8IST_INDEX as usize] = {
            const STACK_SIZE: usize = 4096 * 5;
             static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

            let stack_start = VirtAddr::from_ptr(&raw const STACK);
            let stack_end = stack_start + STACK_SIZE as u64;
            stack_end
        };
        tss
    };
}

use x86_64::structures::gdt::{GlobalDescriptorTable, Descriptor};

lazy_static! {
    static ref GDT: GlobalDescriptorTable = {
        let mut gdt = GlobalDescriptorTable::new();
        gdt.append(Descriptor::kernel_code_segment());
        gdt.append(Descriptor::tss_segment(&TSS));
        gdt
    };
}

pub fn init(){
    GDT.load();
}

