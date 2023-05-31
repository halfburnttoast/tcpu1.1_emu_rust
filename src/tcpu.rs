use std::thread;
use std::time;
use std::io::Write; 
mod debug_rom;

const RAM_SIZE: u16 = 0x100;
const STACK_PTR: u8 = 0xFF;

fn print_type_of<T>(_: &T) {
    println!("{}", std::any::type_name::<T>())
}

pub struct CPU {
    ram:        [u8; RAM_SIZE as usize],
    reg_a:      u8,
    reg_ir:     u8,
    reg_pc:     u8,
    flag_cf:    bool,
    flag_zf:    bool,
    is_running: bool,
    max_cycles: i64,
    opcode_matrix: Vec<fn (&mut CPU) -> ()>
}

impl CPU {
    pub fn new() -> CPU {
        println!("# Initializing CPU");
        CPU {
            ram:        [0; RAM_SIZE as usize],
            reg_a:      0,
            reg_ir:     0,
            reg_pc:     0,
            flag_cf:    false,
            flag_zf:    false,
            is_running: true,
            max_cycles: -1,
            opcode_matrix: vec![
                CPU::inst_ldi,		// 0x00
                CPU::inst_ldr,		// 0x01
                CPU::inst_ldri,		// 0x02
                CPU::inst_addi,		// 0x03
                CPU::inst_addr,		// 0x04
                CPU::inst_subi,		// 0x05
                CPU::inst_subr,		// 0x06
                CPU::inst_str,		// 0x07
                CPU::inst_stri,		// 0x08
                CPU::inst_jmp,		// 0x09
                CPU::inst_jeq,		// 0x0A
                CPU::inst_jcs,		// 0x0B
                CPU::inst_jmpi,		// 0x0C
                CPU::inst_jeqi,		// 0x0D
                CPU::inst_ttyi,		// 0x0E
                CPU::inst_ttyo,		// 0x0F
                CPU::inst_halt,		// 0x10
                CPU::inst_rol,		// 0x11
                CPU::inst_inxr,		// 0x12
                CPU::inst_dexr,		// 0x13
                CPU::inst_asl,		// 0x14
                CPU::inst_nandi,	// 0x15
                CPU::inst_nandr,	// 0x16
                CPU::inst_nop,		// 0x17
                CPU::inst_ainc,		// 0x18
                CPU::inst_adec,		// 0x19
                CPU::inst_rinc,		// 0x1A
                CPU::inst_rdec,		// 0x1B
                CPU::inst_rsp,		// 0x1C
                CPU::inst_pha,		// 0x1D
                CPU::inst_pla,		// 0x1E
                CPU::inst_jsr,		// 0x1F
                CPU::inst_rts,		// 0x20
                CPU::inst_ldsa,		// 0x21
                CPU::inst_stsa,		// 0x22
                CPU::inst_sinc,		// 0x23
                CPU::inst_phi,		// 0x24
            ]
        }
    }

    pub fn reset(&mut self) {
        self.reg_a = 0;
        self.reg_ir = 0;
        self.reg_pc = 0;
        self.flag_cf = false;
        self.flag_zf = true;
        println!("# CPU reset.")
    }

    pub fn dump_state_to_console(&self) {
        println!("REG A: {:02X}, REG_IR: {:02X}, REG_PC: {:02X}, CF: {:02X}, ZF: {:02X}, SP: {:02X}", 
            self.reg_a, 
            self.reg_ir, 
            self.reg_pc,
            self.flag_cf as u8,
            self.flag_zf as u8,
            self.ram[STACK_PTR as usize],
        );
    }

    pub fn set_max_cycles(&mut self, num: i64) {
        if num == 0 {
            println!("Attempted to set max cycle count to 0. Must be value must be > 0.");
            panic!("Invalid max_cycles value: {}", num);
        }
        self.max_cycles = num;
    }

    pub fn dump_ram_to_console(&self) {
        print!("# RAM dump:");
        for (i, v) in self.ram.iter().enumerate() {
            if(i % 8) == 0 {
                println!("");
                print!("{:02X}: ", i);
            }
            print!("{:02X} ", v);
        }
        println!("");
    }

    pub fn copy_to_ram(&mut self, image: &[u8]) {
        println!("# Loading ROM image to RAM");
        let length = image.len();
        self.ram[..length].copy_from_slice(&image);
        println!("# Loaded {} bytes into RAM", length);
    }

    pub fn load_debug_rom(&mut self) {
        self.copy_to_ram(debug_rom::DEBUG_ROM);
    }

    pub fn run(&mut self) {
        println!("# CPU start.");
        while self.is_running == true {
            self.do_inst();
            self.cpu_sleep();
            if self.max_cycles >= 0 {
                if self.max_cycles == 0 {
                    println!("\n\n# Max cycle count reached.");
                    self.inst_halt();
                }
                self.max_cycles -= 1;
            }
        }
    }

    fn cpu_sleep(&self) {
        std::io::stdout().flush().expect("Could not flush STDOUT");
        thread::sleep(time::Duration::from_millis(5));
    }

    fn alu_add(&mut self, value: u8, carry_in: u16) {
        let reg_a: u16  = self.reg_a as u16;
        let val: u16    = value as u16;
        let cf: u16     = carry_in;
        let result: u16 = reg_a.wrapping_add(val);
        let result: u16 = result.wrapping_add(cf);
        if result > 0xFF {
            self.flag_cf = true;
        } else {
            self.flag_cf = false;
        }
        self.set_a(result as u8);
    }

    fn do_inst(&mut self) -> () {

        // Get pointer to function at current REG_PC value
        self.reg_ir = self.ram[self.reg_pc as usize];
        let operation: Option<&fn (&mut CPU) -> ()> = self.opcode_matrix.get(self.reg_ir as usize);

        // autoincrement to next PC address
        self.reg_pc = self.reg_pc.wrapping_add(1);

        // Execute decoded function
        match operation {
            Some(operation) => operation(self),
            None => {
                self.dump_ram_to_console();
                self.dump_state_to_console();
                panic!("Attempted to run invalid opcode: 0x{:02X}", self.reg_ir);
            }
        }

        // autincrement to next PC address
        // NOTE: for some instructions this needs to be canceled out, this is 
        // done explicitly in the instruction definitions below as needed.
        self.reg_pc = self.reg_pc.wrapping_add(1);
    }

    // Sets REG_A of the CPU instance. To keep hardware compatibility
    // REG_A should only be updated using this function.
    #[inline(always)]
    fn set_a(&mut self, val: u8) {
        self.reg_a = val;
        if val == 0 {
            self.flag_zf = true;
        } else {
            self.flag_zf = false;
        }
    }

    // Ram dereferencing / addressing modes
    #[inline(always)]
    fn get_mem_imm(&self) -> u8 {
        self.ram[self.reg_pc as usize]
    }
    #[inline(always)]
    fn get_mem_ind(&self) -> u8 {
        self.ram[self.ram[self.reg_pc as usize] as usize]
    }
    #[inline(always)]
    fn get_mem_dind(&self) -> u8 {
        self.ram[self.get_mem_ind() as usize]
    }


/* ------------------------------------    
    Begin CPU instruction definitions 

    Whenever you see a self.reg_pc -= 1 instruction, it's
    to ensure that the next opcode is aligned properly with
    the actual hardware implementation. AKA: It's to negate
    the automatic increment after the instruction call.
---------------------------------------*/

    fn inst_ldi(&mut self) -> () {
        self.set_a(self.get_mem_imm());
    }

    fn inst_ldr(&mut self) -> () {
        self.set_a(self.get_mem_ind());
    }

    fn inst_ldri(&mut self) -> () {
        self.set_a(self.get_mem_dind());
    }

    fn inst_addi(&mut self) -> () {
        self.alu_add(self.get_mem_imm(), 0);
    }

    fn inst_addr(&mut self) -> () {
        self.alu_add(self.get_mem_ind(), 0);
    }

    fn inst_subi(&mut self) -> () {
        self.alu_add(!self.get_mem_imm(), 1);
    }

    fn inst_subr(&mut self) -> () {
        self.alu_add(!self.get_mem_ind(), 1);
    }

    fn inst_str(&mut self) -> () {
        self.ram[self.get_mem_imm() as usize] = self.reg_a;
    }

    fn inst_stri(&mut self) -> () {
        self.ram[self.get_mem_ind() as usize] = self.reg_a;
        self.reg_pc = self.reg_pc.wrapping_add(1);
    }

    fn inst_jmp(&mut self) -> () {
        self.reg_pc = self.get_mem_imm();
        self.reg_pc = self.reg_pc.wrapping_sub(1);
    }

    fn inst_jeq(&mut self) -> () {
        if self.flag_zf == true {
            self.inst_jmp();         
        }
    }

    fn inst_jcs(&mut self) -> () {
        if self.flag_cf == true {
            self.inst_jmp();
        }
    }

    fn inst_jmpi(&mut self) -> () {
        self.reg_pc = self.ram[self.get_mem_imm() as usize];
        self.reg_pc -= 1;
    }

    fn inst_jeqi(&mut self) -> () {
        if self.flag_zf == true {
            self.inst_jmpi();
        } else {
            self.reg_pc += 1;
        }
    }

    fn inst_ttyi(&mut self) -> () {
        panic!("Instruction not yet implemented");
        self.reg_pc -= 1;
    }

    fn inst_ttyo(&mut self) -> () {
        print!("{}", self.reg_a as char);
        self.reg_pc -= 1;
    }

    fn inst_halt(&mut self) -> () {
        println!("\n# CPU Halted.");
        self.dump_ram_to_console();
        self.dump_state_to_console();
        self.is_running = false;
    }

    fn inst_rol(&mut self) -> () {
        self.set_a(self.reg_a << 1);
    }

    fn inst_inxr(&mut self) -> () {
        let ptr: usize = self.get_mem_imm() as usize;
        let val: u8 = self.ram[(self.reg_pc + 1) as usize];
        self.ram[ptr] += val;
        self.reg_pc += 1;
    }
    fn inst_dexr(&mut self) -> () {
        let ptr: usize = self.get_mem_imm() as usize;
        let val: u8 = self.ram[(self.reg_pc + 1) as usize];
        self.ram[ptr] -= val;
        self.reg_pc += 1;
    }

    fn inst_asl(&mut self) -> () {
        let mut val: i32 = self.reg_a as i32;
        val = val << 1;
        val = val | ((val >> 8) & 1);
        self.set_a(val as u8);
    }

    // does not change ALU status flags
    fn inst_nandi(&mut self) -> () {
        let val: u8 = !(self.reg_a & self.get_mem_imm());
        self.set_a(val);
    }

    // does not change ALU status flags
    fn inst_nandr(&mut self) -> () {
        let val: u8 = !(self.reg_a & self.get_mem_ind());
        self.set_a(val);
    }

    fn inst_nop(&mut self) -> () {
        self.reg_pc -= 1;
    }

    fn inst_ainc(&mut self) -> () {
        self.set_a(self.reg_a.wrapping_add(1));
        self.reg_pc -= 1;
    }

    fn inst_adec(&mut self) -> () {
        self.set_a(self.reg_a.wrapping_sub(1));
        self.reg_pc -= 1;
    }

    fn inst_rinc(&mut self) -> () {
        self.ram[self.get_mem_imm() as usize] += 1;
        self.set_a(self.get_mem_ind());
    }

    fn inst_rdec(&mut self) -> () {
        self.ram[self.get_mem_imm() as usize] -= 1;
        self.set_a(self.get_mem_ind());
    }

    fn inst_rsp(&mut self) -> () {
        self.ram[STACK_PTR as usize] = 0xFE;
        self.reg_pc -= 1;
    }

    fn inst_pha(&mut self) -> () {
        let sp: u8 = self.ram[STACK_PTR as usize];
        self.ram[sp as usize] = self.reg_a;
        self.ram[STACK_PTR as usize] -= 1;
    }

    fn inst_pla(&mut self) -> () {
        self.ram[STACK_PTR as usize] += 1;
        let sp: u8 = self.ram[STACK_PTR as usize];
        self.set_a(self.ram[sp as usize]);
        self.reg_pc -= 1;
    }

    fn inst_jsr(&mut self) -> () {
        let sp: u8 = self.ram[STACK_PTR as usize];
        self.ram[sp as usize] = self.get_mem_imm();
        self.ram[STACK_PTR as usize] -= 1;
        self.reg_pc += 1;
        self.reg_pc = self.get_mem_imm().wrapping_sub(1);
    }

    fn inst_rts(&mut self) -> () {
        self.ram[STACK_PTR as usize] += 1;
        let sp: u8 = self.ram[STACK_PTR as usize];
        self.reg_pc = self.ram[sp as usize].wrapping_sub(1);
    }

    fn inst_ldsa(&mut self) -> () {
        let sp: u8 = self.ram[STACK_PTR as usize] + 2;
        self.set_a(self.ram[sp as usize]);
        self.reg_pc -= 1;
    }
    
    fn inst_stsa(&mut self) -> () {
        let sp: u8 = self.ram[STACK_PTR as usize] + 2;
        self.ram[sp as usize] = self.reg_a;
    }

    fn inst_sinc(&mut self) -> () {
        self.ram[STACK_PTR as usize] += 1;
    }

    fn inst_phi(&mut self) -> () {
        let sp: u8 = self.ram[STACK_PTR as usize];
        self.ram[sp as usize] = self.get_mem_imm();
        self.ram[STACK_PTR as usize] -= 1;
    }


/* ------------------------------------    
    End CPU instruction definitions 
---------------------------------------*/


    // debug stuff
    pub fn test_cycle(&mut self) {
        println!("\n\nSINGLE STEP TEST:");
        self.do_inst();
        self.dump_state_to_console();
    }

}