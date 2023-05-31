mod tcpu;

fn main() {
    let mut cpu: tcpu::CPU = tcpu::CPU::new();
    cpu.reset();
    cpu.load_debug_rom();
    cpu.run();
}
